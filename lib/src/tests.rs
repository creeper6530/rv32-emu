//! Ultimate test: `for file in ./c_test_progs/*.elf; cargo run -- $file -vv; end`

#[cfg(target_os = "linux")]
use memmap2::Advice;
use memmap2::MmapOptions;

use super::*;

#[test]
fn sign_extend_test() {
    let pos = 0b010010;
    assert_eq!(18, sign_extend32(pos, 6));

    let neg = 0b100110;
    assert_eq!(0b11111111111111111111111111100110, sign_extend32(neg, 6));

    let neg2 = 0b110101100001;
    assert_eq!(0b11111111111111111111110101100001, sign_extend32(neg2, 12));
}

#[test]
fn memrw_test() {
    let mut ram = [0u8; 1024];
    let memory = SliceMemory::new(&[], &mut ram).unwrap();

    // Test null pointer dereference (address 0)
    assert_eq!(
        memory.read_8(0x0000_0000),
        Err(Fault::InvalidAddress(0x0000_0000))
    );

    // Test reading from the stack top (beyond the valid stack memory range)
    assert_eq!(
        memory.read_8(memory.stack_top()),
        Err(Fault::InvalidAddress(memory.stack_top()))
    );

    // Test reading from an address just below the stack top (valid)
    assert_eq!(memory.read_8(memory.stack_top() - 1), Ok(0));

    // Test reading a halfword that spans the boundary of valid memory
    assert_eq!(
        memory.read_16(memory.stack_top() - 1),
        Err(Fault::InvalidAddress(memory.stack_top() - 1))
    );

    // Test reading from an empty instruction memory
    assert_eq!(
        memory.read_8(memory.instr_start()),
        Err(Fault::InvalidAddress(memory.instr_start()))
    );

    let instr = [0xEF, 0xBE, 0xAD, 0xDE];
    let mut memory = SliceMemory::new(&instr, &mut ram).unwrap();

    // Test reading a word from instruction memory
    assert_eq!(memory.read_32(memory.instr_start()), Ok(0xDEADBEEF));

    // Test unaligned reading a halfword from instruction memory
    assert_eq!(memory.read_16(memory.instr_start() + 1), Ok(0xADBE));

    // Test reading a halfword from instruction memory that spans the boundary of valid memory
    assert_eq!(
        memory.read_16(memory.instr_start() + 3),
        Err(Fault::InvalidAddress(memory.instr_start() + 3))
    );

    // Test writing to instruction memory (which is read-only)
    assert_eq!(
        memory.write_16(memory.instr_start(), 0x1234),
        Err(Fault::ReadOnlyAddress(memory.instr_start()))
    );

    // Test writing to data memory
    let data_addr = memory.stack_top() - 2;
    assert_eq!(memory.write_16(data_addr, 0xABCD), Ok(()));
    assert_eq!(memory.read_16(data_addr), Ok(0xABCD));

    // Test writing a word that spans the boundary of valid memory
    assert_eq!(
        memory.write_32(memory.stack_top() - 2, 0x12345678),
        Err(Fault::InvalidAddress(memory.stack_top() - 2))
    );
}

#[test]
fn initialization_test() {
    let mut memory = SliceMemory::new(&[], &mut []).unwrap();
    let mut cpu = Cpu::new(&mut memory);
    cpu.reset(None).unwrap(); // We don't intend to execute any instructions
}

const PROGRAM_NOP: [u8; 4] = [
    0x13, 0x00, 0x00, 0x00, // nop
];

#[test]
fn program_nop() {
    let mut memory = SliceMemory::new(&PROGRAM_NOP, &mut []).unwrap();
    let mut cpu = Cpu::new(&mut memory);
    cpu.reset(None).unwrap();

    cpu.step().unwrap();
}

const PROGRAM_ADDI: [u8; 8] = [
    0x93, 0x82, 0x12, 0x00, // addi t0, t0, 1
    0x93, 0x82, 0x22, 0x00, // addi t0, t0, 2
];

#[test]
fn program_addi() {
    let mut memory = SliceMemory::new(&PROGRAM_ADDI, &mut []).unwrap();
    let mut cpu = Cpu::new(&mut memory);

    // Only test with a specific entry point...
    cpu.reset(Some(0x1000_0004)).unwrap();

    cpu.step().unwrap();

    assert_eq!(cpu.read_reg(Regs::T0), 2);
}

// Assembler: https://riscv-simulator-five.vercel.app/
const PROGRAM_MEMWRITE: [u8; 16] = [
    // construct 46a98 in t0
    0xb7, 0x72, 0x04, 0x00, // lui t0, 0x47
    0x93, 0x82, 0x82, 0xA9, // addi t0, t0, -1384
    // write t0 to memory at 0x2000_0000
    0x37, 0x03, 0x00, 0x20, // lui t1, 0x20000
    0x23, 0x20, 0x53, 0x00, // sw t0,0(t1)
];

#[test]
fn program_memwrite() {
    // RAM backed by stack-allocated array
    let mut ram = [0u8; 1024];

    let mut memory = SliceMemory::new(&PROGRAM_MEMWRITE, &mut ram).unwrap();
    let mut cpu = Cpu::new(&mut memory);
    cpu.reset(None).unwrap();

    cpu.step().unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();

    assert_eq!(cpu.memory.read_32(0x2000_0000).unwrap(), 0x46A98);
    assert_eq!(cpu.pc, Wrapping(cpu.memory.instr_start() + 16));
}

#[test]
fn program_c_memwrite() {
    // RAM backed by heap-allocated boxed slice
    let mut ram = create_boxed_slice(1024);

    let mut memory =
        SliceMemory::new(include_bytes!("../../c_test_progs/memwrite.bin"), &mut ram).unwrap();
    let mut cpu = Cpu::new(&mut memory);
    cpu.reset(None).unwrap();

    cpu.run().unwrap();

    assert_eq!(
        cpu.memory.read_32(cpu.read_reg(Regs::Sp) - 4).unwrap(),
        0xC0FFEE
    );
}

#[test]
fn program_c_functioncall() {
    // RAM backed by heap-allocated boxed array
    let mut ram = create_boxed_array::<1024>();

    let mut memory = SliceMemory::new(
        include_bytes!("../../c_test_progs/functioncall.bin"),
        &mut *ram, // Need to dereference to coerce the array to a slice
    )
    .unwrap();
    let mut cpu = Cpu::new(&mut memory);
    cpu.reset(None).unwrap();

    // Discard the `a1` register, since C returns a 32bit integer in `a0`
    // (but would return it in `a1` too if it were a 64bit integer)
    assert_eq!(cpu.run().unwrap().0, 15);
}

#[test]
fn program_c_branch() {
    // RAM backed by stack-allocated array
    let mut ram = [0u8; 1024];
    let mut memory =
        SliceMemory::new(include_bytes!("../../c_test_progs/branch.bin"), &mut ram).unwrap();
    memory.write_8(memory.stack_top() - 1, 41).unwrap(); // Odd number

    let mut cpu = Cpu::new(&mut memory);
    cpu.reset(None).unwrap();
    assert_eq!(cpu.run().unwrap().0, 45);

    cpu.reset(None).unwrap(); // Reset the PC to run again
    // We can't write into `ram` directly because of the borrow checker
    cpu.memory.write_8(cpu.memory.stack_top() - 1, 70).unwrap(); // Even number
    assert_eq!(cpu.run().unwrap().0, 12);
}

#[test]
fn program_c_literal() {
    let mut ram = create_boxed_slice(1024);
    let mut memory =
        SliceMemory::new(include_bytes!("../../c_test_progs/literal.bin"), &mut ram).unwrap();
    let mut cpu = Cpu::new(&mut memory);
    cpu.reset(None).unwrap();

    cpu.run().unwrap();

    let mut buffer = [0u8; 8];
    let dst_addr = cpu.read_reg(Regs::Sp) - 8;
    // Clippy linted for `enumerate()` instead of `0..8`
    for (i, item) in buffer.iter_mut().enumerate() {
        *item = cpu.memory.read_8(dst_addr + (i as u32)).unwrap();
    }
    assert_eq!(&buffer, b"Testing\0");
}

#[test]
fn memmap_program_c_functioncall() {
    // Working directory of tests is the root directory of the package
    // https://doc.rust-lang.org/cargo/commands/cargo-test.html#working-directory-of-tests
    let file =
        std::fs::File::open("../c_test_progs/functioncall.bin").expect("Failed to open file!");
    file.lock_shared().expect("Failed to lock file!");

    // SAFETY: If underlying file is any way modified while the memory mapping is in use,
    // we risk UB. Shared locking attempts to prevent this, but it is not guaranteed to work on all platforms.
    let instr_mmap = unsafe { MmapOptions::new().map(&file) }.unwrap();

    let mut data_mmap = MmapOptions::new().len(1024).map_anon().unwrap();
    // Transparent huge pages are only supported on Linux on anonymous memory mappings
    #[cfg(target_os = "linux")]
    data_mmap.advise(Advice::HugePage).unwrap_or_else(|e| {
        warn!("Failed to advise huge pages for data memory: {}", e);
    });
    #[cfg(target_os = "linux")]
    data_mmap.advise(Advice::Random).unwrap_or_else(|e| {
        warn!("Failed to advise random access for data memory: {}", e);
    });

    let mut memory = SliceMemory::new(&instr_mmap, &mut data_mmap).unwrap();

    let mut cpu = Cpu::new(&mut memory);
    cpu.reset(None).unwrap();

    assert_eq!(cpu.run().unwrap().0, 15);

    // Technically should be handled by RAII, but just to be sure.
    file.unlock().expect("Failed to unlock file!");
}

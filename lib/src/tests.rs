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
fn initialization_test() {
    let memory: BoxedMemory<1024, 1024> = BoxedMemory::new(None).unwrap();
    let mut cpu = Cpu::new(memory);
    cpu.reset(None).unwrap(); // We don't intend to execute this!

    assert_eq!(cpu.pc, Wrapping(cpu.memory.instr_mem_base()));
    assert_eq!(cpu.read_reg(Regs::Sp), cpu.memory.data_mem_end());
}

const PROGRAM_NOP: [u8; 4] = [
    0x13, 0x00, 0x00, 0x00, // nop
];

#[test]
fn program_nop() {
    let memory: BoxedMemory<1024, 1024> = BoxedMemory::new(Some(&PROGRAM_NOP)).unwrap();
    let mut cpu = Cpu::new(memory);
    cpu.reset(None).unwrap();

    cpu.step().unwrap();
}

const PROGRAM_ADDI: [u8; 4] = [
    0x93, 0x02, 0x10, 0x00, // addi t0, zero, 1
];

#[test]
fn program_addi() {
    let memory: BoxedMemory<1024, 1024> = BoxedMemory::new(Some(&PROGRAM_ADDI)).unwrap();
    let mut cpu = Cpu::new(memory);
    cpu.reset(None).unwrap();

    cpu.step().unwrap();

    assert_eq!(cpu.read_reg(Regs::T0), 1);
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
    let memory: BoxedMemory<1024, 1024> = BoxedMemory::new(Some(&PROGRAM_MEMWRITE)).unwrap();
    let mut cpu = Cpu::new(memory);
    cpu.reset(None).unwrap();

    cpu.step().unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();

    assert_eq!(cpu.memory.read_32(0x2000_0000).unwrap(), 0x46A98);
    assert_eq!(cpu.pc, Wrapping(cpu.memory.instr_mem_base() + 16));
}

#[test]
fn program_c_memwrite() {
    let memory: BoxedMemory<1024, 1024> =
        BoxedMemory::new(Some(&include_bytes!("../../c_test_progs/memwrite.bin")[..])).unwrap();
    let mut cpu = Cpu::new(memory);
    cpu.reset(None).unwrap();

    cpu.run().unwrap();

    assert_eq!(cpu.memory.read_32(0x2000_0040).unwrap(), 0x101);
}

#[test]
fn program_c_functioncall() {
    let memory: BoxedMemory<1024, 1024> = BoxedMemory::new(Some(
        &include_bytes!("../../c_test_progs/functioncall.bin")[..],
    ))
    .unwrap();
    let mut cpu = Cpu::new(memory);
    cpu.reset(None).unwrap();

    cpu.step().unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    // Execute up until the `jal`, then check that the PC landed at the correct address
    // To be edited when `functioncall.c` is changed`
    assert_eq!(cpu.pc, Wrapping(cpu.memory.instr_mem_base() + 0x28));

    cpu.run().unwrap();

    assert_eq!(cpu.memory.read_16(0x2000_0321).unwrap(), 15);
}

#[test]
fn program_c_branch() {
    let memory: BoxedMemory<1024, 1024> =
        BoxedMemory::new(Some(&include_bytes!("../../c_test_progs/branch.bin")[..])).unwrap();
    let mut cpu = Cpu::new(memory);
    cpu.reset(None).unwrap();

    cpu.memory.write_32(0x2000_00FF, 400).unwrap(); // Even number
    cpu.run().unwrap();
    assert_eq!(cpu.memory.read_32(0x2000_00AA).unwrap(), 123);

    cpu.reset(None).unwrap(); // No need to reload the program

    cpu.memory.write_32(0x2000_00FF, 777).unwrap(); // Odd number
    cpu.run().unwrap();
    assert_eq!(cpu.memory.read_32(0x2000_00AA).unwrap(), 456);
}

#[test]
fn program_c_memwrite_memmap() {
    // Working directory of tests is the root directory of the package
    // https://doc.rust-lang.org/cargo/commands/cargo-test.html#working-directory-of-tests
    let file = std::fs::File::open("../c_test_progs/memwrite.bin").expect("Failed to open file!");
    file.lock_shared().expect("Failed to lock file!");

    // SAFETY: If underlying file is any way modified while the memory mapping is in use,
    // we risk UB. Shared locking attempts to prevent this, but it is not guaranteed to work on all platforms.
    let memory = unsafe { MemmapMemory::new(&file, 2048) }.expect("Failed to open memory mapping!");

    let mut cpu = Cpu::new(memory);
    cpu.reset(None).unwrap();

    cpu.run().unwrap();
    assert_eq!(cpu.memory.read_32(0x2000_0040).unwrap(), 0x101);

    // Technically should be handled by RAII, but just to be sure.
    file.unlock().expect("Failed to unlock file!");
}

#[test]
fn instantiate_memory() {
    // True maximum size that does not overlap.
    let _heap_memory: BoxedMemory<0x10000000, 0x10000000> = BoxedMemory::new(None).unwrap();

    // Needs to be smaller not to exceed the stack size limit of the test runner, maximum size of which is unknown.
    let _array_memory: ArrayMemory<0x200, 0x200> = ArrayMemory::new(None).unwrap();
}

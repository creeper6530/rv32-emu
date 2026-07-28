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
    let mut memory = AddressSpace::new();
    let mut regs = RegArray::new();

    let mut cpu = Cpu::new(&mut regs, &mut memory);
    cpu.reset(None, Some(&[0x00, 0x00, 0x00, 0x00])); // We don't intend to execute this!

    assert_eq!(cpu.pc, Wrapping(INSTR_MEM_BASE));
    assert_eq!(cpu.read_reg(Regs::Sp), DATA_MEM_END);
}

const PROGRAM_NOP: [u8; 4] = [
    0x13, 0x00, 0x00, 0x00, // nop
];

#[test]
fn program_nop() {
    let mut memory = AddressSpace::new();
    let mut regs = RegArray::new();

    let mut cpu = Cpu::new(&mut regs, &mut memory);
    cpu.reset(None, Some(&PROGRAM_NOP));

    cpu.step();
}

const PROGRAM_ADDI: [u8; 4] = [
    0x93, 0x02, 0x10, 0x00, // addi t0, zero, 1
];

#[test]
fn program_addi() {
    let mut memory = AddressSpace::new();
    let mut regs = RegArray::new();

    let mut cpu = Cpu::new(&mut regs, &mut memory);
    cpu.reset(None, Some(&PROGRAM_ADDI));

    cpu.step();

    assert_eq!(cpu.read_reg(Regs::T0), 1);
}

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
    let mut memory = AddressSpace::new();
    let mut regs = RegArray::new();

    let mut cpu = Cpu::new(&mut regs, &mut memory);
    cpu.reset(None, Some(&PROGRAM_MEMWRITE));

    cpu.step();
    cpu.step();
    cpu.step();
    cpu.step();

    assert_eq!(cpu.memory.read_32(0x2000_0000), 0x46A98);
    assert_eq!(cpu.pc, Wrapping(INSTR_MEM_BASE + 16));
}

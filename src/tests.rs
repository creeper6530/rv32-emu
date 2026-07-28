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
    assert_eq!(cpu.pc, Wrapping(0));
    for i in 0..32 {
        assert_eq!(cpu.read_reg(i.into()), 0);
    }
}

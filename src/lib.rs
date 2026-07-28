#![no_std]

use core::num::Wrapping;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/*
# MEMORY MAP:

0x1000_0000 - 0x1000_03FF: Instruction memory (1 KiB)
0x2000_0000 - 0x2000_03FF: Data memory (1 KiB)
*/

const INSTR_MEM_SIZE: u32 = 1 * 1024; // 1 KiB
const DATA_MEM_SIZE: u32 = 1 * 1024; // 1 KiB
const INSTR_MEM_BASE: u32 = 0x1000_0000;
const DATA_MEM_BASE: u32 = 0x2000_0000;
const INSTR_MEM_END: u32 = INSTR_MEM_BASE + INSTR_MEM_SIZE;
const DATA_MEM_END: u32 = DATA_MEM_BASE + DATA_MEM_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Fault {
    InvalidMemoryAccess(u32),
    UndecodedInstruction(u32),
    InvalidInstruction(Instruction),

    AllZeroInstruction,
    Halt,
}

// All accesses are little-endian
#[derive(Debug, Clone, Copy)]
pub struct AddressSpace {
    instr: [u8; INSTR_MEM_SIZE as usize], // 1 KiB of instruction memory
    data: [u8; DATA_MEM_SIZE as usize],   // 1 KiB of data memory
}
impl AddressSpace {
    fn new() -> Self {
        AddressSpace {
            instr: [0; INSTR_MEM_SIZE as usize],
            data: [0; DATA_MEM_SIZE as usize],
        }
    }

    #[inline]
    fn read_8(&self, addr: u32) -> Result<u8, Fault> {
        match addr {
            // Instruction memory
            INSTR_MEM_BASE..INSTR_MEM_END => Ok(self.instr[(addr - INSTR_MEM_BASE) as usize]),
            // Data memory
            DATA_MEM_BASE..DATA_MEM_END => Ok(self.data[(addr - DATA_MEM_BASE) as usize]),
            _ => {
                error!("Invalid memory read at address: {:#X}", addr);
                Err(Fault::InvalidMemoryAccess(addr))
            }
        }
    }

    #[inline]
    fn read_16(&self, addr: u32) -> Result<u16, Fault> {
        match addr {
            // Instruction memory
            INSTR_MEM_BASE..INSTR_MEM_END => {
                let index = (addr - INSTR_MEM_BASE) as usize;
                Ok(u16::from_le_bytes(
                    *self.instr[index..=index + 1].as_array::<2>().unwrap(),
                ))
            }
            // Data memory
            DATA_MEM_BASE..DATA_MEM_END => {
                let index = (addr - DATA_MEM_BASE) as usize;
                Ok(u16::from_le_bytes(
                    *self.data[index..=index + 1].as_array::<2>().unwrap(),
                ))
            }
            _ => {
                error!("Invalid memory read at address: {:#X}", addr);
                Err(Fault::InvalidMemoryAccess(addr))
            }
        }
    }

    #[inline]
    fn read_32(&self, addr: u32) -> Result<u32, Fault> {
        match addr {
            // Instruction memory
            INSTR_MEM_BASE..INSTR_MEM_END => {
                let index = (addr - INSTR_MEM_BASE) as usize;
                Ok(u32::from_le_bytes(
                    *self.instr[index..=index + 3].as_array::<4>().unwrap(),
                ))
            }
            // Data memory
            DATA_MEM_BASE..DATA_MEM_END => {
                let index = (addr - DATA_MEM_BASE) as usize;
                Ok(u32::from_le_bytes(
                    *self.data[index..=index + 3].as_array::<4>().unwrap(),
                ))
            }
            _ => {
                error!("Invalid memory read at address: {:#X}", addr);
                Err(Fault::InvalidMemoryAccess(addr))
            }
        }
    }

    #[inline]
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault> {
        match addr {
            // Instruction memory
            INSTR_MEM_BASE..INSTR_MEM_END => {
                error!(
                    "Instruction memory is read-only, cannot write to address: {:#X}",
                    addr
                );
                Err(Fault::InvalidMemoryAccess(addr))
            }
            // Data memory
            DATA_MEM_BASE..DATA_MEM_END => {
                self.data[(addr - DATA_MEM_BASE) as usize] = value;
                Ok(())
            }
            _ => {
                error!("Invalid memory write at address: {:#X}", addr);
                Err(Fault::InvalidMemoryAccess(addr))
            }
        }
    }

    #[inline]
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault> {
        match addr {
            // Instruction memory
            INSTR_MEM_BASE..INSTR_MEM_END => {
                error!(
                    "Instruction memory is read-only, cannot write to address: {:#X}",
                    addr
                );
                Err(Fault::InvalidMemoryAccess(addr))
            }
            // Data memory
            DATA_MEM_BASE..DATA_MEM_END => {
                let index = (addr - DATA_MEM_BASE) as usize;
                self.data[index..=index + 1].copy_from_slice(&value.to_le_bytes());
                Ok(())
            }
            _ => {
                error!("Invalid memory write at address: {:#X}", addr);
                Err(Fault::InvalidMemoryAccess(addr))
            }
        }
    }

    #[inline]
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault> {
        match addr {
            // Instruction memory
            INSTR_MEM_BASE..INSTR_MEM_END => {
                error!(
                    "Instruction memory is read-only, cannot write to address: {:#X}",
                    addr
                );
                Err(Fault::InvalidMemoryAccess(addr))
            }
            // Data memory
            DATA_MEM_BASE..DATA_MEM_END => {
                let index = (addr - DATA_MEM_BASE) as usize;
                self.data[index..=index + 3].copy_from_slice(&value.to_le_bytes());
                Ok(())
            }
            _ => {
                error!("Invalid memory write at address: {:#X}", addr);
                Err(Fault::InvalidMemoryAccess(addr))
            }
        }
    }
}
impl Default for AddressSpace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RegArray([u32; 32]);
impl RegArray {
    pub fn new() -> Self {
        RegArray::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Regs {
    Zero = 0,
    Ra = 1,
    Sp = 2,
    Gp = 3,
    Tp = 4,
    T0 = 5,
    T1 = 6,
    T2 = 7,
    S0 = 8,
    S1 = 9,
    A0 = 10,
    A1 = 11,
    A2 = 12,
    A3 = 13,
    A4 = 14,
    A5 = 15,
    A6 = 16,
    A7 = 17,
    S2 = 18,
    S3 = 19,
    S4 = 20,
    S5 = 21,
    S6 = 22,
    S7 = 23,
    S8 = 24,
    S9 = 25,
    S10 = 26,
    S11 = 27,
    T3 = 28,
    T4 = 29,
    T5 = 30,
    T6 = 31,
}
impl From<u8> for Regs {
    fn from(value: u8) -> Self {
        match value {
            0 => Regs::Zero,
            1 => Regs::Ra,
            2 => Regs::Sp,
            3 => Regs::Gp,
            4 => Regs::Tp,
            5 => Regs::T0,
            6 => Regs::T1,
            7 => Regs::T2,
            8 => Regs::S0,
            9 => Regs::S1,
            10 => Regs::A0,
            11 => Regs::A1,
            12 => Regs::A2,
            13 => Regs::A3,
            14 => Regs::A4,
            15 => Regs::A5,
            16 => Regs::A6,
            17 => Regs::A7,
            18 => Regs::S2,
            19 => Regs::S3,
            20 => Regs::S4,
            21 => Regs::S5,
            22 => Regs::S6,
            23 => Regs::S7,
            24 => Regs::S8,
            25 => Regs::S9,
            26 => Regs::S10,
            27 => Regs::S11,
            28 => Regs::T3,
            29 => Regs::T4,
            30 => Regs::T5,
            31 => Regs::T6,
            _ => panic!("Invalid register index: {}", value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Instruction {
    RType {
        opcode: u8,
        rd: Regs,
        funct3: u8,
        rs1: Regs,
        rs2: Regs,
        funct7: u8,
    },
    IType {
        opcode: u8,
        rd: Regs,
        funct3: u8,
        rs1: Regs,
        imm: u32,
    },
    SType {
        opcode: u8,
        funct3: u8,
        rs1: Regs,
        rs2: Regs,
        imm: u32,
    },
    BType {
        opcode: u8,
        funct3: u8,
        rs1: Regs,
        rs2: Regs,
        imm: u32,
    },
    UType {
        opcode: u8,
        rd: Regs,
        imm: u32,
    },
    JType {
        opcode: u8,
        rd: Regs,
        imm: u32,
    },
}

#[derive(Debug)]
pub struct Cpu<'a> {
    regs: &'a mut RegArray,
    pc: Wrapping<u32>,
    memory: &'a mut AddressSpace,
}

impl<'a> Cpu<'a> {
    #[inline]
    fn write_reg(&mut self, reg: Regs, value: u32) {
        if reg != Regs::Zero {
            self.regs.0[reg as usize] = value;
        }
    }

    #[inline]
    fn read_reg(&self, reg: Regs) -> u32 {
        if reg == Regs::Zero {
            0
        } else {
            self.regs.0[reg as usize]
        }
    }

    #[inline]
    fn decode(&self, instr: u32) -> Result<Instruction, Fault> {
        if instr == 0 {
            error!(
                "All-zero instruction encountered at PC (illegal): {:#X}",
                self.pc.0
            );
            return Err(Fault::AllZeroInstruction);
        }

        let opcode = (instr & 0x7F) as u8;

        if opcode & 0b11 != 0b11 {
            error!("Compressed instructions are not supported");
            return Err(Fault::UndecodedInstruction(instr));
        }

        match opcode {
            0b0110011 /*| 0b0101111*/ => {
                // R-type
                let rd = ((instr >> 7) as u8 & 0x1F).into();
                let funct3 = (instr >> 12) as u8 & 0x7;
                let rs1 = ((instr >> 15) as u8 & 0x1F).into();
                let rs2 = ((instr >> 20) as u8 & 0x1F).into();
                let funct7 = (instr >> 25) as u8 & 0x7F;
                Ok(Instruction::RType {
                    opcode,
                    rd,
                    funct3,
                    rs1,
                    rs2,
                    funct7,
                })
            }
            0b0010011 | 0b0000011 | 0b1100111 | 0b1110011 => {
                // I-type
                let rd = ((instr >> 7) as u8 & 0x1F).into();
                let funct3 = (instr >> 12) as u8 & 0x7;
                let rs1 = ((instr >> 15) as u8 & 0x1F).into();
                let imm = (instr >> 20) & 0xFFF;
                Ok(Instruction::IType {
                    opcode,
                    rd,
                    funct3,
                    rs1,
                    imm,
                })
            }
            0b0100011 => {
                // S-type
                let funct3 = (instr >> 12) as u8 & 0x7;
                let rs1 = ((instr >> 15) as u8 & 0x1F).into();
                let rs2 = ((instr >> 20) as u8 & 0x1F).into();
                let imm = ((instr >> 7) & 0x1F) | (((instr >> 25) & 0x7F) << 5);
                Ok(Instruction::SType {
                    opcode,
                    funct3,
                    rs1,
                    rs2,
                    imm,
                })
            }
            0b1100011 => {
                // B-type
                let funct3 = (instr >> 12) as u8 & 0x7;
                let rs1 = ((instr >> 15) as u8 & 0x1F).into();
                let rs2 = ((instr >> 20) as u8 & 0x1F).into();
                let imm: u32 = ((instr >> 7)  & 0b0000000011110) // imm[4:1]
                             | ((instr >> 20) & 0b0011111100000) // imm[10:5]
                             | ((instr << 4)  & 0b0100000000000) // imm[11]
                             | ((instr >> 19) & 0b1000000000000);// imm[12]
                Ok(Instruction::BType {
                    opcode,
                    funct3,
                    rs1,
                    rs2,
                    imm,
                })
            }
            0b1101111 => {
                // J-type
                let rd = ((instr >> 7) as u8 & 0x1F).into();
                let imm: u32 = ((instr >> 20) & 0b000000000011111111110) // imm[10:1]
                             | ((instr >> 9)  & 0b000000000100000000000) // imm[11]
                             | ( instr        & 0b011111111000000000000) // imm[19:12]
                             | ((instr >> 11) & 0b100000000000000000000);// imm[20]
                Ok(Instruction::JType { opcode, rd, imm })
            }
            0b0110111 => {
                // U-type
                let rd = ((instr >> 7) as u8 & 0x1F).into();
                let imm = instr & 0xFFFFF000;
                Ok(Instruction::UType { opcode, rd, imm })
            }
            _ => Err(Fault::UndecodedInstruction(instr)),
        }
    }

    #[inline]
    fn execute(&mut self, instr: Instruction) -> Result<(), Fault> {
        match instr {
            Instruction::RType { .. } => self.execute_arith_reg_reg(instr),
            Instruction::IType { opcode, .. } => match opcode {
                0b0010011 => self.execute_arith_reg_imm(instr),
                0b0000011 => self.execute_load(instr),
                0b1100111 => self.execute_jalr(instr),
                0b1110011 => self.execute_envops(instr),
                _ => unreachable!("I-type instruction with bad opcode: {:?}", instr),
            },
            Instruction::SType { .. } => self.execute_store(instr),
            Instruction::BType { .. } => self.execute_branch(instr),
            Instruction::UType { opcode, .. } => match opcode {
                0b0110111 => self.execute_lui(instr),
                0b0010111 => self.execute_auipc(instr),
                _ => unreachable!("U-type instruction with bad opcode: {:?}", instr),
            },
            Instruction::JType { .. } => self.execute_jal(instr),
        }
    }

    #[inline(always)]
    /// TODO: Use the never type for this once it stabilises
    fn execute_arith_reg_reg(&mut self, instr: Instruction) -> Result<(), Fault> {
        let Instruction::RType {
            opcode: _,
            rd,
            funct3,
            rs1,
            rs2,
            funct7,
        } = instr
        else {
            panic!("Expected R-type instruction");
        };

        match funct3 {
            0x0 => {
                if funct7 == 0x20 {
                    // SUB
                    let result = self.read_reg(rs1).wrapping_sub(self.read_reg(rs2));
                    self.write_reg(rd, result);
                } else {
                    // if funct7 == 0x00
                    // ADD
                    let result = self.read_reg(rs1).wrapping_add(self.read_reg(rs2));
                    self.write_reg(rd, result);
                }
            }
            0x4 => {
                // XOR
                let result = self.read_reg(rs1) ^ self.read_reg(rs2);
                self.write_reg(rd, result);
            }
            0x6 => {
                // OR
                let result = self.read_reg(rs1) | self.read_reg(rs2);
                self.write_reg(rd, result);
            }
            0x7 => {
                // AND
                let result = self.read_reg(rs1) & self.read_reg(rs2);
                self.write_reg(rd, result);
            }
            0x1 => {
                // SLL
                let shamt = self.read_reg(rs2) & 0x1F;
                let result = self.read_reg(rs1) << shamt;
                self.write_reg(rd, result);
            }
            0x5 => {
                if funct7 == 0x20 {
                    // SRA
                    // Rust does arithmetic right shift for signed integers
                    let shamt = self.read_reg(rs2) & 0x1F;
                    let result = ((self.read_reg(rs1) as i32) >> shamt) as u32;
                    self.write_reg(rd, result);
                } else {
                    // if funct7 == 0x00
                    // SRL
                    // Rust does logical right shift for unsigned integers
                    let shamt = self.read_reg(rs2) & 0x1F;
                    let result = self.read_reg(rs1) >> shamt;
                    self.write_reg(rd, result);
                }
            }
            0x2 => {
                // SLT
                let result = (self.read_reg(rs1) as i32) < (self.read_reg(rs2) as i32);
                self.write_reg(rd, result as u32);
            }
            0x3 => {
                // SLTU
                let result = self.read_reg(rs1) < self.read_reg(rs2);
                self.write_reg(rd, result as u32);
            }
            _ => unreachable!("funct3 is 3 bits, instruction malformed: {:?}", instr),
        }

        Ok(())
    }

    #[inline(always)]
    fn execute_arith_reg_imm(&mut self, instr: Instruction) -> Result<(), Fault> {
        let Instruction::IType {
            opcode: _,
            rd,
            funct3,
            rs1,
            imm,
        } = instr
        else {
            panic!("Expected I-type instruction");
        };

        match funct3 {
            0x0 => {
                // ADDI
                let result = self.read_reg(rs1).wrapping_add(sign_extend32(imm, 12));
                self.write_reg(rd, result);
            }
            0x4 => {
                // XORI
                let result = self.read_reg(rs1) ^ sign_extend32(imm, 12);
                self.write_reg(rd, result);
            }
            0x6 => {
                // ORI
                let result = self.read_reg(rs1) | sign_extend32(imm, 12);
                self.write_reg(rd, result);
            }
            0x7 => {
                // ANDI
                let result = self.read_reg(rs1) & sign_extend32(imm, 12);
                self.write_reg(rd, result);
            }
            0x1 => {
                // SLLI
                let shamt = imm & 0x1F;
                let result = self.read_reg(rs1) << shamt;
                self.write_reg(rd, result);
            }
            0x5 => {
                let shamt = imm & 0x1F;
                if (imm >> 10) & 0x1 == 0 {
                    // imm[5:11] == 0x00
                    // SRLI
                    let result = self.read_reg(rs1) >> shamt;
                    self.write_reg(rd, result);
                } else {
                    // imm[5:11] == 0x20
                    // SRAI
                    let result = ((self.read_reg(rs1) as i32) >> shamt) as u32;
                    self.write_reg(rd, result);
                }
            }
            0x2 => {
                // SLTI
                let result = (self.read_reg(rs1) as i32) < ((sign_extend32(imm, 12)) as i32);
                self.write_reg(rd, result as u32);
            }
            0x3 => {
                // SLTIU
                let result = self.read_reg(rs1) < sign_extend32(imm, 12);
                self.write_reg(rd, result as u32);
            }
            _ => unreachable!("funct3 is 3 bits, instruction malformed: {:?}", instr),
        }
        Ok(())
    }

    #[inline(always)]
    fn execute_load(&mut self, instr: Instruction) -> Result<(), Fault> {
        let Instruction::IType {
            opcode: _,
            rd,
            funct3,
            rs1,
            imm,
        } = instr
        else {
            panic!("Expected I-type instruction");
        };

        let addr = self.read_reg(rs1).wrapping_add(sign_extend32(imm, 12));

        match funct3 {
            0x0 => {
                // LB
                // Upcasting as signed sign-extends
                let value = self.memory.read_8(addr)? as i8 as i32 as u32;
                self.write_reg(rd, value);
            }
            0x1 => {
                // LH
                // Upcasting as signed sign-extends
                let value = self.memory.read_16(addr)? as i16 as i32 as u32;
                self.write_reg(rd, value);
            }
            0x2 => {
                // LW
                let value = self.memory.read_32(addr)?;
                self.write_reg(rd, value);
            }
            0x4 => {
                // LBU
                // Upcasting as unsigned zero-extends
                let value = self.memory.read_8(addr)? as u32;
                self.write_reg(rd, value);
            }
            0x5 => {
                // LHU
                // Upcasting as unsigned zero-extends
                let value = self.memory.read_16(addr)? as u32;
                self.write_reg(rd, value);
            }
            _ => {
                error!("Illegal instruction: {:?}", instr);
                return Err(Fault::InvalidInstruction(instr));
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn execute_store(&mut self, instr: Instruction) -> Result<(), Fault> {
        let Instruction::SType {
            opcode: _,
            funct3,
            rs1,
            rs2,
            imm,
        } = instr
        else {
            panic!("Expected S-type instruction");
        };

        let addr = self.read_reg(rs1).wrapping_add(sign_extend32(imm, 12));

        match funct3 {
            0x0 => {
                // SB
                let value = (self.read_reg(rs2) & 0xFF) as u8;
                self.memory.write_8(addr, value)?;
            }
            0x1 => {
                // SH
                let value = (self.read_reg(rs2) & 0xFFFF) as u16;
                self.memory.write_16(addr, value)?;
            }
            0x2 => {
                // SW
                let value = self.read_reg(rs2);
                self.memory.write_32(addr, value)?;
            }
            _ => {
                error!("Illegal instruction: {:?}", instr);
                return Err(Fault::InvalidInstruction(instr));
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn execute_branch(&mut self, instr: Instruction) -> Result<(), Fault> {
        let Instruction::BType {
            opcode: _,
            funct3,
            rs1,
            rs2,
            imm,
        } = instr
        else {
            panic!("Expected B-type instruction");
        };

        let rs1_val = self.read_reg(rs1);
        let rs2_val = self.read_reg(rs2);

        let take_branch = match funct3 {
            0x0 => rs1_val == rs2_val,                   // BEQ
            0x1 => rs1_val != rs2_val,                   // BNE
            0x4 => (rs1_val as i32) < (rs2_val as i32),  // BLT
            0x5 => (rs1_val as i32) >= (rs2_val as i32), // BGE
            0x6 => rs1_val < rs2_val,                    // BLTU
            0x7 => rs1_val >= rs2_val,                   // BGEU
            _ => {
                error!("Illegal instruction: {:?}", instr);
                return Err(Fault::InvalidInstruction(instr));
            }
        };

        if take_branch {
            self.pc += Wrapping(sign_extend32(imm, 13));
        }
        Ok(())
    }

    #[inline(always)]
    fn execute_jal(&mut self, instr: Instruction) -> Result<(), Fault> {
        let Instruction::JType { opcode: _, rd, imm } = instr else {
            panic!("Expected J-type instruction");
        };

        self.write_reg(rd, (self.pc + Wrapping(4)).0);
        self.pc += Wrapping(sign_extend32(imm, 21));
        Ok(())
    }

    #[inline(always)]
    fn execute_jalr(&mut self, instr: Instruction) -> Result<(), Fault> {
        let Instruction::IType {
            opcode: _,
            rd,
            funct3: _,
            rs1,
            imm,
        } = instr
        else {
            error!("Expected I-type instruction");
            return Err(Fault::InvalidInstruction(instr));
        };

        self.write_reg(rd, (self.pc + Wrapping(4)).0);

        let target_address = self.read_reg(rs1).wrapping_add(sign_extend32(imm, 12));
        self.pc = Wrapping(target_address & !1); // Clear the least significant bit
        Ok(())
    }

    #[inline(always)]
    fn execute_lui(&mut self, instr: Instruction) -> Result<(), Fault> {
        let Instruction::UType { opcode: _, rd, imm } = instr else {
            error!("Expected U-type instruction");
            return Err(Fault::InvalidInstruction(instr));
        };

        // Immediate already ANDed with 0xFFFFF000 in decode
        // Lower 12 bits are supposed to be zeroed
        self.write_reg(rd, imm);
        Ok(())
    }

    #[inline(always)]
    fn execute_auipc(&mut self, instr: Instruction) -> Result<(), Fault> {
        let Instruction::UType { opcode: _, rd, imm } = instr else {
            error!("Expected U-type instruction");
            return Err(Fault::InvalidInstruction(instr));
        };

        // Immediate already ANDed with 0xFFFFF000 in decode
        // Lower 12 bits are supposed to be zeroed
        let result = self.pc + Wrapping(imm);
        self.write_reg(rd, result.0);
        Ok(())
    }

    #[inline(always)]
    fn execute_envops(&mut self, instr: Instruction) -> Result<(), Fault> {
        let Instruction::IType {
            opcode: _,
            rd: _,
            funct3: _,
            rs1: _,
            imm,
        } = instr
        else {
            error!("Expected I-type instruction");
            return Err(Fault::InvalidInstruction(instr));
        };

        match imm {
            0x000 => {
                // ECALL
                warn!("ECALL invoked at PC: {:#X}", self.pc.0);

                // Currently NOOP
            }
            0x001 => {
                // EBREAK
                warn!("EBREAK invoked at PC: {:#X}", self.pc.0);
                return Err(Fault::Halt);
            }
            _ => {
                error!("Illegal instruction: {:?}", instr);
                return Err(Fault::InvalidInstruction(instr));
            }
        }
        Ok(())
    }
}

impl<'a> Cpu<'a> {
    pub fn new(regs: &'a mut RegArray, memory: &'a mut AddressSpace) -> Self {
        Cpu {
            regs,
            pc: Wrapping(0),
            memory,
        }
    }

    /// Meant to be called like `let mut cpu = Cpu::new(...).init(...);`,
    /// not `cpu.init(...)` on an existing instance
    pub fn reset(&mut self, entry_point: Option<u32>, program: Option<&[u8]>) {
        info!("Resetting CPU");

        self.pc = Wrapping(entry_point.unwrap_or(INSTR_MEM_BASE));

        // Initialize the stack pointer to the top of the data memory (growing downwards on RISC-V)
        self.write_reg(Regs::Sp, DATA_MEM_END);

        if let Some(program) = program {
            self.memory.instr[0..program.len()].copy_from_slice(program);
        }
    }

    pub fn step(&mut self) -> Result<(), Fault> {
        let raw_instr = self.memory.read_32(self.pc.0)?;

        let instr = self.decode(raw_instr)?;
        self.execute(instr)?;

        self.pc += Wrapping(4);
        Ok(())
    }

    /// Runs until a halt instruction is encountered or a fault occurs
    ///
    /// Halt is triggered by an ECALL with a0=1
    pub fn run(&mut self) -> Result<(), Fault> {
        loop {
            match self.step() {
                Ok(_) => {}
                Err(Fault::Halt) => {
                    info!("CPU halted at PC: {:#X}", self.pc.0);
                    return Ok(());
                }
                Err(e) => {
                    error!("CPU fault at PC: {:#X}: {:?}", self.pc.0, e);
                    return Err(e);
                }
            }
        }
    }
}

// Stolen from `binutils` crate under MIT license
#[inline]
pub fn sign_extend32(data: u32, size: u32) -> u32 {
    assert!(size > 0 && size <= 32);
    let shamt = 32 - size;
    (((data << shamt) as i32) >> shamt) as u32
}

#[cfg(test)]
mod tests;

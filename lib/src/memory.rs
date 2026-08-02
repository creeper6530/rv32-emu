#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::{vec, vec::Vec};

#[cfg(feature = "object")]
use object::{Object, ObjectSegment};

use crate::Fault;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

// --------------------------------------------------

/// All accesses are little-endian, and instruction and data memory may overlap.
///
/// Stack pointer shall be initialized to the end of data memory,
/// and program counter to the start of instruction memory, unless other entry point is specified.
pub trait AddressSpace {
    /// First instruction address (base of instruction memory).
    fn instr_start(&self) -> u32;
    /// Stack top - the first address above the stack (end of data memory).
    fn stack_top(&self) -> u32;

    fn read_8(&self, addr: u32) -> Result<u8, Fault>;
    fn read_16(&self, addr: u32) -> Result<u16, Fault>;
    fn read_32(&self, addr: u32) -> Result<u32, Fault>;
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault>;
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault>;
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault>;
}

// --------------------------------------------------

pub struct SliceMemory<'a> {
    instr: &'a [u8],
    data: &'a mut [u8],
}

impl<'a> SliceMemory<'a> {
    const INSTR_MEM_BASE: u32 = 0x1000_0000;
    const DATA_MEM_BASE: u32 = 0x2000_0000;

    pub fn new(instr: &'a [u8], data: &'a mut [u8]) -> Self {
        Self { instr, data }
    }

    #[inline(always)]
    fn instr_mem_end(&self) -> u32 {
        Self::INSTR_MEM_BASE + self.instr.len() as u32
    }
    #[inline(always)]
    fn data_mem_end(&self) -> u32 {
        Self::DATA_MEM_BASE + self.data.len() as u32
    }
}

impl<'a> AddressSpace for SliceMemory<'a> {
    #[inline(always)]
    fn instr_start(&self) -> u32 {
        Self::INSTR_MEM_BASE
    }
    #[inline(always)]
    fn stack_top(&self) -> u32 {
        Self::DATA_MEM_BASE + self.data.len() as u32
    }

    #[inline]
    fn read_8(&self, addr: u32) -> Result<u8, Fault> {
        // We can't use generic params in (range) patterns (E0158), so we have to use if instead.
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() {
            Ok(self.instr[(addr - Self::INSTR_MEM_BASE) as usize])
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() {
            Ok(self.data[(addr - Self::DATA_MEM_BASE) as usize])
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidAddress(addr))
        }
    }

    #[inline]
    fn read_16(&self, addr: u32) -> Result<u16, Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() - 1 {
            // -1 because we read 2 bytes
            let index = (addr - Self::INSTR_MEM_BASE) as usize;
            Ok(u16::from_le_bytes([
                self.instr[index],
                self.instr[index + 1],
            ]))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() - 1 {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            Ok(u16::from_le_bytes([self.data[index], self.data[index + 1]]))
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidAddress(addr))
        }
    }

    #[inline]
    fn read_32(&self, addr: u32) -> Result<u32, Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() - 3 {
            // -3 because we read 4 bytes
            let index = (addr - Self::INSTR_MEM_BASE) as usize;
            Ok(u32::from_le_bytes([
                self.instr[index],
                self.instr[index + 1],
                self.instr[index + 2],
                self.instr[index + 3],
            ]))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() - 3 {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            Ok(u32::from_le_bytes([
                self.data[index],
                self.data[index + 1],
                self.data[index + 2],
                self.data[index + 3],
            ]))
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidAddress(addr))
        }
    }

    #[inline]
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::ReadOnlyAddress(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() {
            self.data[(addr - Self::DATA_MEM_BASE) as usize] = value;
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidAddress(addr))
        }
    }

    #[inline]
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() - 1 {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::ReadOnlyAddress(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() - 1 {
            // I could write `value.to_le_bytes().iter().enumerate().for_each(|(i, b)| self.data[index + i] = *b);`,
            // but that optimizes to the same code as below, and is less readable IMO.

            let index = (addr - Self::DATA_MEM_BASE) as usize;
            let bytes = value.to_le_bytes();
            self.data[index] = bytes[0];
            self.data[index + 1] = bytes[1];
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidAddress(addr))
        }
    }

    #[inline]
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() - 3 {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::ReadOnlyAddress(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() - 3 {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            let bytes = value.to_le_bytes();
            self.data[index] = bytes[0];
            self.data[index + 1] = bytes[1];
            self.data[index + 2] = bytes[2];
            self.data[index + 3] = bytes[3];
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidAddress(addr))
        }
    }
}

// --------------------------------------------------

#[cfg(feature = "object")]
pub struct ObjectMemory<'obj, 'stack, T>
where
    T: Object<'obj> + 'obj,
{
    object: T,
    stack: &'stack mut [u8],

    _object_lifetime: core::marker::PhantomData<&'obj T>,
}

#[cfg(feature = "object")]
impl<'obj, 'stack, T> ObjectMemory<'obj, 'stack, T>
where
    T: Object<'obj> + 'obj,
{
    const STACK_MEM_BASE: u32 = 0x3000_0000;

    pub fn new(object: T, stack: &'stack mut [u8]) -> Result<Self, Fault> {
        Ok(Self {
            object,
            stack,

            _object_lifetime: core::marker::PhantomData,
        })
    }
}

#[cfg(feature = "object")]
impl<'obj, 'stack, T> AddressSpace for ObjectMemory<'obj, 'stack, T>
where
    T: Object<'obj> + 'obj,
{
    #[inline(always)]
    fn instr_start(&self) -> u32 {
        self.object.entry() as u32
    }
    #[inline(always)]
    fn stack_top(&self) -> u32 {
        Self::STACK_MEM_BASE + self.stack.len() as u32
    }

    #[inline]
    fn read_8(&self, addr: u32) -> Result<u8, Fault> {
        // `.segments()` is already an iterator
        for seg in self.object.segments() {
            let start: u32 = seg.address().try_into().map_err(|_| Fault::ObjectError)?;
            let end: u32 = start + u32::try_from(seg.size()).map_err(|_| Fault::ObjectError)?;

            if addr >= start && addr < end {
                let offset = (addr - start) as usize;
                let data = seg.data()?;
                return Ok(data[offset]);
            }
        }

        if addr >= Self::STACK_MEM_BASE && addr < self.stack_top() {
            let offset = (addr - Self::STACK_MEM_BASE) as usize;
            return Ok(self.stack[offset]);
        }

        Err(Fault::InvalidAddress(addr))
    }

    #[inline]
    fn read_16(&self, addr: u32) -> Result<u16, Fault> {
        todo!()
    }

    #[inline]
    fn read_32(&self, addr: u32) -> Result<u32, Fault> {
        todo!()
    }

    #[inline]
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault> {
        todo!()
    }

    #[inline]
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault> {
        todo!()
    }

    #[inline]
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault> {
        todo!()
    }
}

// --------------------------------------------------

#[cfg(feature = "alloc")]
#[inline(always)]
pub fn create_boxed_slice(size: usize) -> Box<[u8]> {
    let res: Box<[u8]> = vec![0_u8; size].into_boxed_slice();
    debug_assert_eq!(res.len(), size);
    res
}

#[cfg(feature = "alloc")]
#[inline(always)]
pub fn create_boxed_array<const N: usize>() -> Box<[u8; N]> {
    let res: Box<[u8; N]> = vec![0_u8; N].into_boxed_slice().try_into().unwrap();
    debug_assert_eq!(res.len(), N);
    res
}

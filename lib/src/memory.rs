#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::vec;

#[cfg(feature = "memmap2")]
use memmap2::{Advice, MmapOptions};
#[cfg(feature = "memmap2")]
use std::fs::File;

use crate::Fault;
use core::fmt::Debug;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

// --------------------------------------------------

/// All accesses are little-endian, and instruction and data memory may overlap.
///
/// Stack pointer shall be initialized to the end of data memory,
/// and program counter to the start of instruction memory, unless other entry point is specified.
pub trait AddressSpace {
    /// Start of instruction memory (may just return an associated constant).
    fn instr_mem_base(&self) -> u32;
    /// Start of data memory (may just return an associated constant).
    fn data_mem_base(&self) -> u32;
    /// End of instruction memory (non-inclusive, may just return an associated constant).
    fn instr_mem_end(&self) -> u32;
    /// End of data memory (non-inclusive, may just return an associated constant).
    fn data_mem_end(&self) -> u32;

    fn read_8(&self, addr: u32) -> Result<u8, Fault>;
    fn read_16(&self, addr: u32) -> Result<u16, Fault>;
    fn read_32(&self, addr: u32) -> Result<u32, Fault>;
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault>;
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault>;
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault>;
}

// Implements for `Box<dyn AddressSpace>` and `&mut dyn AddressSpace`
// This allows `Cpu` to accept a dynamically-dispatched memory implementation
// (for example when we don't know at compile time which implementation we want)
impl<T: core::ops::DerefMut<Target = dyn AddressSpace>> AddressSpace for T {
    // Just one dereference would create infinite recursion, so we always need to dereference twice.

    #[inline(always)]
    fn instr_mem_base(&self) -> u32 {
        (**self).instr_mem_base()
    }
    #[inline(always)]
    fn data_mem_base(&self) -> u32 {
        (**self).data_mem_base()
    }
    #[inline(always)]
    fn instr_mem_end(&self) -> u32 {
        (**self).instr_mem_end()
    }
    #[inline(always)]
    fn data_mem_end(&self) -> u32 {
        (**self).data_mem_end()
    }

    #[inline]
    fn read_8(&self, addr: u32) -> Result<u8, Fault> {
        (**self).read_8(addr)
    }
    #[inline]
    fn read_16(&self, addr: u32) -> Result<u16, Fault> {
        (**self).read_16(addr)
    }
    #[inline]
    fn read_32(&self, addr: u32) -> Result<u32, Fault> {
        (**self).read_32(addr)
    }
    #[inline]
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault> {
        (**self).write_8(addr, value)
    }
    #[inline]
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault> {
        (**self).write_16(addr, value)
    }
    #[inline]
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault> {
        (**self).write_32(addr, value)
    }
}

impl Debug for dyn AddressSpace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "dyn AddressSpace {{ instr_mem_base: {:#X}, data_mem_base: {:#X}, instr_mem_end: {:#X}, data_mem_end: {:#X} }}",
            self.instr_mem_base(),
            self.data_mem_base(),
            self.instr_mem_end(),
            self.data_mem_end()
        )
    }
}

// --------------------------------------------------

/// Struct is only composed of two Boxes, so it shall not be heap-allocated itself.
///
/// In this implementation instruction and data memory do not overlap.
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct BoxedMemory<const INSTR_MEM_SIZE: usize, const DATA_MEM_SIZE: usize> {
    instr: Box<[u8; INSTR_MEM_SIZE]>,
    data: Box<[u8; DATA_MEM_SIZE]>,
}

#[cfg(feature = "alloc")]
impl<const INSTR_MEM_SIZE: usize, const DATA_MEM_SIZE: usize>
    BoxedMemory<INSTR_MEM_SIZE, DATA_MEM_SIZE>
{
    const INSTR_MEM_BASE: u32 = 0x1000_0000;
    const DATA_MEM_BASE: u32 = 0x2000_0000;
    const INSTR_MEM_END: u32 = {
        assert!(
            INSTR_MEM_SIZE > 0,
            "Instruction memory size must be greater than 0"
        );
        assert!(
            (Self::INSTR_MEM_BASE + INSTR_MEM_SIZE as u32) <= Self::DATA_MEM_BASE,
            "Instruction and data memory must not overlap"
        );
        Self::INSTR_MEM_BASE + INSTR_MEM_SIZE as u32
    };
    const DATA_MEM_END: u32 = {
        assert!(DATA_MEM_SIZE > 0, "Data memory size must be greater than 0");
        Self::DATA_MEM_BASE + DATA_MEM_SIZE as u32
    };

    pub fn new(program: Option<&[u8]>) -> Result<Self, Fault> {
        // We need this hack because `Box::new([0; N])` first creates a `[0; N]` on the stack
        // and then moves it to the heap, which still causes a stack overflow for large N.
        let mut new = BoxedMemory {
            instr: vec![0; INSTR_MEM_SIZE]
                .into_boxed_slice()
                .try_into()
                .unwrap(),
            data: vec![0; DATA_MEM_SIZE]
                .into_boxed_slice()
                .try_into()
                .unwrap(),
        };

        if let Some(program) = program {
            if program.len() > new.instr.len() {
                error!(
                    "Program size ({}) exceeds instruction memory size ({})",
                    program.len(),
                    new.instr.len()
                );
                return Err(Fault::MemoryTooSmall);
            }
            new.instr[0..program.len()].copy_from_slice(program);
        }

        Ok(new)
    }
}

#[cfg(feature = "alloc")]
impl<const INSTR_MEM_SIZE: usize, const DATA_MEM_SIZE: usize> AddressSpace
    for BoxedMemory<INSTR_MEM_SIZE, DATA_MEM_SIZE>
{
    #[inline(always)]
    fn instr_mem_base(&self) -> u32 {
        Self::INSTR_MEM_BASE
    }
    #[inline(always)]
    fn data_mem_base(&self) -> u32 {
        Self::DATA_MEM_BASE
    }
    #[inline(always)]
    fn instr_mem_end(&self) -> u32 {
        Self::INSTR_MEM_END
    }
    #[inline(always)]
    fn data_mem_end(&self) -> u32 {
        Self::DATA_MEM_END
    }

    #[inline]
    fn read_8(&self, addr: u32) -> Result<u8, Fault> {
        // We can't use generic params in (range) patterns (E0158), so we have to use if instead.
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            Ok(self.instr[(addr - Self::INSTR_MEM_BASE) as usize])
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            Ok(self.data[(addr - Self::DATA_MEM_BASE) as usize])
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn read_16(&self, addr: u32) -> Result<u16, Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            let index = (addr - Self::INSTR_MEM_BASE) as usize;
            Ok(u16::from_le_bytes([
                self.instr[index],
                self.instr[index + 1],
            ]))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            Ok(u16::from_le_bytes([self.data[index], self.data[index + 1]]))
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn read_32(&self, addr: u32) -> Result<u32, Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            let index = (addr - Self::INSTR_MEM_BASE) as usize;
            Ok(u32::from_le_bytes([
                self.instr[index],
                self.instr[index + 1],
                self.instr[index + 2],
                self.instr[index + 3],
            ]))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            Ok(u32::from_le_bytes([
                self.data[index],
                self.data[index + 1],
                self.data[index + 2],
                self.data[index + 3],
            ]))
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::InvalidMemoryAccess(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            self.data[(addr - Self::DATA_MEM_BASE) as usize] = value;
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::InvalidMemoryAccess(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            let bytes = value.to_le_bytes();
            self.data[index] = bytes[0];
            self.data[index + 1] = bytes[1];
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::InvalidMemoryAccess(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            let bytes = value.to_le_bytes();
            self.data[index] = bytes[0];
            self.data[index + 1] = bytes[1];
            self.data[index + 2] = bytes[2];
            self.data[index + 3] = bytes[3];
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }
}

#[cfg(feature = "alloc")]
impl<const INSTR_MEM_SIZE: usize, const DATA_MEM_SIZE: usize> Default
    for BoxedMemory<INSTR_MEM_SIZE, DATA_MEM_SIZE>
{
    fn default() -> Self {
        Self::new(None).expect("None should not lead to errors!")
    }
}

// Deriving Debug for BoxedMemory would cause a stack overflow upon attempting to format it,
// so we implement Debug manually to avoid printing the entire memory contents.
impl<const INSTR_MEM_SIZE: usize, const DATA_MEM_SIZE: usize> Debug
    for BoxedMemory<INSTR_MEM_SIZE, DATA_MEM_SIZE>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "BoxedMemory {{ instr_mem_base: {:#X}, data_mem_base: {:#X}, instr_mem_end: {:#X}, data_mem_end: {:#X} }}",
            self.instr_mem_base(),
            self.data_mem_base(),
            self.instr_mem_end(),
            self.data_mem_end()
        )
    }
}

// --------------------------------------------------

/// Near clone of `BoxedMemory`, but uses arrays instead of heap-allocated boxes.
///
/// In this implementation instruction and data memory do not overlap.
#[derive(Clone, Copy)]
pub struct ArrayMemory<const INSTR_MEM_SIZE: usize, const DATA_MEM_SIZE: usize> {
    instr: [u8; INSTR_MEM_SIZE],
    data: [u8; DATA_MEM_SIZE],
}

impl<const INSTR_MEM_SIZE: usize, const DATA_MEM_SIZE: usize>
    ArrayMemory<INSTR_MEM_SIZE, DATA_MEM_SIZE>
{
    const INSTR_MEM_BASE: u32 = 0x1000_0000;
    const DATA_MEM_BASE: u32 = 0x2000_0000;
    const INSTR_MEM_END: u32 = {
        assert!(
            INSTR_MEM_SIZE > 0,
            "Instruction memory size must be greater than 0"
        );
        assert!(
            (Self::INSTR_MEM_BASE + INSTR_MEM_SIZE as u32) <= Self::DATA_MEM_BASE,
            "Instruction and data memory must not overlap"
        );
        Self::INSTR_MEM_BASE + INSTR_MEM_SIZE as u32
    };
    const DATA_MEM_END: u32 = {
        assert!(DATA_MEM_SIZE > 0, "Data memory size must be greater than 0");
        Self::DATA_MEM_BASE + DATA_MEM_SIZE as u32
    };

    pub fn new(program: Option<&[u8]>) -> Result<Self, Fault> {
        let mut new = ArrayMemory {
            instr: [0; INSTR_MEM_SIZE],
            data: [0; DATA_MEM_SIZE],
        };

        if let Some(program) = program {
            if program.len() > new.instr.len() {
                error!(
                    "Program size ({}) exceeds instruction memory size ({})",
                    program.len(),
                    new.instr.len()
                );
                return Err(Fault::MemoryTooSmall);
            }
            new.instr[0..program.len()].copy_from_slice(program);
        }

        Ok(new)
    }
}

impl<const INSTR_MEM_SIZE: usize, const DATA_MEM_SIZE: usize> AddressSpace
    for ArrayMemory<INSTR_MEM_SIZE, DATA_MEM_SIZE>
{
    #[inline(always)]
    fn instr_mem_base(&self) -> u32 {
        Self::INSTR_MEM_BASE
    }
    #[inline(always)]
    fn data_mem_base(&self) -> u32 {
        Self::DATA_MEM_BASE
    }
    #[inline(always)]
    fn instr_mem_end(&self) -> u32 {
        Self::INSTR_MEM_END
    }
    #[inline(always)]
    fn data_mem_end(&self) -> u32 {
        Self::DATA_MEM_END
    }

    #[inline]
    fn read_8(&self, addr: u32) -> Result<u8, Fault> {
        // We can't use generic params in (range) patterns (E0158), so we have to use if instead.
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            Ok(self.instr[(addr - Self::INSTR_MEM_BASE) as usize])
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            Ok(self.data[(addr - Self::DATA_MEM_BASE) as usize])
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn read_16(&self, addr: u32) -> Result<u16, Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            let index = (addr - Self::INSTR_MEM_BASE) as usize;
            Ok(u16::from_le_bytes([
                self.instr[index],
                self.instr[index + 1],
            ]))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            Ok(u16::from_le_bytes([self.data[index], self.data[index + 1]]))
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn read_32(&self, addr: u32) -> Result<u32, Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            let index = (addr - Self::INSTR_MEM_BASE) as usize;
            Ok(u32::from_le_bytes([
                self.instr[index],
                self.instr[index + 1],
                self.instr[index + 2],
                self.instr[index + 3],
            ]))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            Ok(u32::from_le_bytes([
                self.data[index],
                self.data[index + 1],
                self.data[index + 2],
                self.data[index + 3],
            ]))
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::InvalidMemoryAccess(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            self.data[(addr - Self::DATA_MEM_BASE) as usize] = value;
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::InvalidMemoryAccess(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            let bytes = value.to_le_bytes();
            self.data[index] = bytes[0];
            self.data[index + 1] = bytes[1];
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < Self::INSTR_MEM_END {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::InvalidMemoryAccess(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < Self::DATA_MEM_END {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            let bytes = value.to_le_bytes();
            self.data[index] = bytes[0];
            self.data[index + 1] = bytes[1];
            self.data[index + 2] = bytes[2];
            self.data[index + 3] = bytes[3];
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }
}

impl<const INSTR_MEM_SIZE: usize, const DATA_MEM_SIZE: usize> Default
    for ArrayMemory<INSTR_MEM_SIZE, DATA_MEM_SIZE>
{
    fn default() -> Self {
        Self::new(None).expect("None should not lead to errors!")
    }
}

// Technically the ArrayMemory struct is already Debug, since it's relatively small
// and should fit on the stack, but we maintain consistency.
impl<const INSTR_MEM_SIZE: usize, const DATA_MEM_SIZE: usize> Debug
    for ArrayMemory<INSTR_MEM_SIZE, DATA_MEM_SIZE>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "ArrayMemory {{ instr_mem_base: {:#X}, data_mem_base: {:#X}, instr_mem_end: {:#X}, data_mem_end: {:#X} }}",
            self.instr_mem_base(),
            self.data_mem_base(),
            self.instr_mem_end(),
            self.data_mem_end()
        )
    }
}

// --------------------------------------------------

/// Memory implementation using memory-mapped files for instruction memory
/// and anonymous memory for data memory (effectively a large malloc).
///
/// Instruction memory is read-only, while data memory is read-write.
/// Instruction and data memory do not overlap.
#[cfg(feature = "memmap2")]
pub struct MemmapMemory {
    instr: memmap2::Mmap,
    data: memmap2::MmapMut,
}

#[cfg(feature = "memmap2")]
impl MemmapMemory {
    const INSTR_MEM_BASE: u32 = 0x1000_0000;
    const DATA_MEM_BASE: u32 = 0x2000_0000;

    /// # Safety
    ///
    /// All file-backed memory map constructors are marked unsafe because of the potential for UB
    /// using the map if the underlying file is subsequently modified, in or out of process.
    /// Applications must consider the risk and take appropriate precautions when using file-backed maps.
    /// Solutions such as file permissions, locks or process-private (e.g. unlinked) files exist but are platform specific and limited.
    ///
    /// [Source](https://docs.rs/memmap2/latest/memmap2/struct.MmapOptions.html#safety)
    pub unsafe fn new(instr_file: &File, mem_size: usize) -> Result<Self, Fault> {
        // SAFETY: See function documentation
        let instr_mmap = unsafe { MmapOptions::new().map(instr_file) }?;

        let data_mmap = MmapOptions::new().len(mem_size).map_anon()?;
        // Transparent huge pages are only supported on Linux on anonymous memory mappings
        #[cfg(target_os = "linux")]
        data_mmap.advise(Advice::HugePage).unwrap_or_else(|e| {
            warn!("Failed to advise huge pages for data memory: {}", e);
        });
        #[cfg(target_os = "linux")]
        data_mmap.advise(Advice::Random).unwrap_or_else(|e| {
            warn!("Failed to advise random access for data memory: {}", e);
        });

        Ok(Self {
            instr: instr_mmap,
            data: data_mmap,
        })
    }

    pub fn new_fileless(program: &[u8], mem_size: usize) -> Result<Self, Fault> {
        let mut instr_mmap = MmapOptions::new().len(program.len()).map_anon()?;
        #[cfg(target_os = "linux")]
        instr_mmap.advise(Advice::HugePage).unwrap_or_else(|e| {
            warn!("Failed to advise huge pages for instruction memory: {}", e);
        });
        instr_mmap.copy_from_slice(program);

        let data_mmap = MmapOptions::new().len(mem_size).map_anon()?;
        #[cfg(target_os = "linux")]
        data_mmap.advise(Advice::HugePage).unwrap_or_else(|e| {
            warn!("Failed to advise huge pages for data memory: {}", e);
        });
        #[cfg(target_os = "linux")]
        data_mmap.advise(Advice::Random).unwrap_or_else(|e| {
            warn!("Failed to advise random access for data memory: {}", e);
        });

        Ok(Self {
            instr: instr_mmap.make_read_only()?,
            data: data_mmap,
        })
    }
}

impl AddressSpace for MemmapMemory {
    #[inline(always)]
    fn instr_mem_base(&self) -> u32 {
        Self::INSTR_MEM_BASE
    }
    #[inline(always)]
    fn data_mem_base(&self) -> u32 {
        Self::DATA_MEM_BASE
    }
    #[inline(always)]
    fn instr_mem_end(&self) -> u32 {
        Self::INSTR_MEM_BASE + self.instr.len() as u32
    }
    #[inline(always)]
    fn data_mem_end(&self) -> u32 {
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
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn read_16(&self, addr: u32) -> Result<u16, Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() {
            let index = (addr - Self::INSTR_MEM_BASE) as usize;
            Ok(u16::from_le_bytes([
                self.instr[index],
                self.instr[index + 1],
            ]))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            Ok(u16::from_le_bytes([self.data[index], self.data[index + 1]]))
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn read_32(&self, addr: u32) -> Result<u32, Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() {
            let index = (addr - Self::INSTR_MEM_BASE) as usize;
            Ok(u32::from_le_bytes([
                self.instr[index],
                self.instr[index + 1],
                self.instr[index + 2],
                self.instr[index + 3],
            ]))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            Ok(u32::from_le_bytes([
                self.data[index],
                self.data[index + 1],
                self.data[index + 2],
                self.data[index + 3],
            ]))
        } else {
            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
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
            Err(Fault::InvalidMemoryAccess(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() {
            self.data[(addr - Self::DATA_MEM_BASE) as usize] = value;
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::InvalidMemoryAccess(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            let bytes = value.to_le_bytes();
            self.data[index] = bytes[0];
            self.data[index + 1] = bytes[1];
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }

    #[inline]
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault> {
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() {
            error!(
                "Instruction memory is read-only, cannot write to address: {:#X}",
                addr
            );
            Err(Fault::InvalidMemoryAccess(addr))
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() {
            let index = (addr - Self::DATA_MEM_BASE) as usize;
            let bytes = value.to_le_bytes();
            self.data[index] = bytes[0];
            self.data[index + 1] = bytes[1];
            self.data[index + 2] = bytes[2];
            self.data[index + 3] = bytes[3];
            Ok(())
        } else {
            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidMemoryAccess(addr))
        }
    }
}

impl Debug for MemmapMemory {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "MemmapMemory {{ instr_mem_base: {:#X}, data_mem_base: {:#X}, instr_mem_end: {:#X}, data_mem_end: {:#X} }}",
            self.instr_mem_base(),
            self.data_mem_base(),
            self.instr_mem_end(),
            self.data_mem_end()
        )
    }
}

// --------------------------------------------------

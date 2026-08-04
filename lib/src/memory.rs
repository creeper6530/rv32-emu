#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::vec;

#[cfg(feature = "object")]
use object::{Object, ObjectSegment};

#[cfg(feature = "std")]
use std::io::{Read, Write};

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

    fn read_8(&mut self, addr: u32) -> Result<u8, Fault>;
    fn read_16(&mut self, addr: u32) -> Result<u16, Fault>;
    fn read_32(&mut self, addr: u32) -> Result<u32, Fault>;
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault>;
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault>;
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault>;
}

// --------------------------------------------------

pub struct SliceMemory<'instr, 'data, 'mmio> {
    instr: &'instr [u8],
    data: &'data mut [u8],
    mmio: &'mmio mut [&'mmio mut dyn MMIO],
}

impl<'instr, 'data, 'mmio> SliceMemory<'instr, 'data, 'mmio> {
    const INSTR_MEM_BASE: u32 = 0x1000_0000;
    const DATA_MEM_BASE: u32 = 0x2000_0000;

    pub fn new(
        instr: &'instr [u8],
        data: &'data mut [u8],
        mmio: &'mmio mut [&'mmio mut dyn MMIO],
    ) -> Result<Self, Fault> {
        if Self::INSTR_MEM_BASE + instr.len() as u32 <= Self::DATA_MEM_BASE {
            Ok(Self { instr, data, mmio })
        } else {
            Err(Fault::MemoryTooSmall)
        }
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

impl<'instr, 'data, 'mmio> AddressSpace for SliceMemory<'instr, 'data, 'mmio> {
    #[inline(always)]
    fn instr_start(&self) -> u32 {
        Self::INSTR_MEM_BASE
    }
    #[inline(always)]
    fn stack_top(&self) -> u32 {
        Self::DATA_MEM_BASE + self.data.len() as u32
    }

    #[inline]
    fn read_8(&mut self, addr: u32) -> Result<u8, Fault> {
        // We can't use generic params in (range) patterns (E0158), so we have to use if instead.
        // Instruction memory
        if addr >= Self::INSTR_MEM_BASE && addr < self.instr_mem_end() {
            Ok(self.instr[(addr - Self::INSTR_MEM_BASE) as usize])
        }
        // Data memory
        else if addr >= Self::DATA_MEM_BASE && addr < self.data_mem_end() {
            Ok(self.data[(addr - Self::DATA_MEM_BASE) as usize])
        } else {
            for mmio in self.mmio.iter_mut() {
                if mmio.range().contains(&addr) {
                    return mmio.read_8(addr);
                }
            }

            error!("Invalid memory read at address: {:#X}", addr);
            Err(Fault::InvalidAddress(addr))
        }
    }

    #[inline]
    fn read_16(&mut self, addr: u32) -> Result<u16, Fault> {
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
    fn read_32(&mut self, addr: u32) -> Result<u32, Fault> {
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
            /* for mmio in self.mmio.iter_mut() {
                if mmio.range().contains(&addr) {
                    return mmio.read_32(addr);
                }
            } */

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
            for mmio in self.mmio.iter_mut() {
                if mmio.range().contains(&addr) {
                    return mmio.write_8(addr, value);
                }
            }

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
            /* for mmio in self.mmio.iter_mut() {
                if mmio.range().contains(&addr) {
                    return mmio.write_32(addr, value);
                }
            } */

            error!("Invalid memory write at address: {:#X}", addr);
            Err(Fault::InvalidAddress(addr))
        }
    }
}

// --------------------------------------------------

#[cfg(feature = "object")]
#[derive(Debug)]
struct WritableSegment {
    start: u32,
    end: u32,
    data: Vec<u8>,
}

#[cfg(feature = "object")]
pub struct ObjectMemory<'obj, 'stack, 'mmio, T>
where
    T: Object<'obj>,
{
    object: T,
    writable_segments: Vec<WritableSegment>,
    stack: &'stack mut [u8],
    mmio: &'mmio mut [&'mmio mut dyn MMIO],

    _object_lifetime: core::marker::PhantomData<&'obj T>,
}

#[cfg(feature = "object")]
impl<'obj, 'stack, 'mmio, T> ObjectMemory<'obj, 'stack, 'mmio, T>
where
    T: Object<'obj>,
{
    const STACK_MEM_BASE: u32 = 0x3000_0000;

    pub fn new(
        object: T,
        stack: &'stack mut [u8],
        mmio: &'mmio mut [&'mmio mut dyn MMIO],
    ) -> Result<Self, Fault> {
        let mut writable_segments = Vec::new();

        for segment in object.segments() {
            let start = u32::try_from(segment.address()).map_err(|_| Fault::ObjectError)?;
            let size = u32::try_from(segment.size()).map_err(|_| Fault::ObjectError)?;
            let end = start.checked_add(size).ok_or(Fault::ObjectError)?;

            if segment.permissions().writable() {
                let file_data = segment.data().map_err(|_| Fault::ObjectError)?;
                let mut data = vec![0_u8; size as usize];
                let copy_len = core::cmp::min(file_data.len(), data.len());
                data[..copy_len].copy_from_slice(&file_data[..copy_len]);

                debug!(
                    "Segment: {:#X} - {:#X} ({} bytes), writable",
                    start, end, size,
                );

                writable_segments.push(WritableSegment { start, end, data });
            } else {
                debug!(
                    "Segment: {:#X} - {:#X} ({} bytes), read-only",
                    start, end, size,
                );
            }
        }

        Ok(Self {
            object,
            writable_segments,
            stack,
            mmio,
            _object_lifetime: core::marker::PhantomData,
        })
    }
}

#[cfg(feature = "object")]
impl<'obj, 'stack, 'mmio, T> AddressSpace for ObjectMemory<'obj, 'stack, 'mmio, T>
where
    T: Object<'obj>,
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
    fn read_8(&mut self, addr: u32) -> Result<u8, Fault> {
        if let Some(segment) = self
            .writable_segments
            .iter()
            .find(|segment| addr >= segment.start && addr < segment.end)
        {
            return Ok(segment.data[(addr - segment.start) as usize]);
        }

        // `.segments()` is already an iterator
        for seg in self.object.segments() {
            let start: u32 = u32::try_from(seg.address()).map_err(|_| Fault::ObjectError)?;
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

        for mmio in self.mmio.iter_mut() {
            if mmio.range().contains(&addr) {
                return mmio.read_8(addr);
            }
        }

        Err(Fault::InvalidAddress(addr))
    }

    #[inline]
    fn read_16(&mut self, addr: u32) -> Result<u16, Fault> {
        if let Some(segment) = self
            .writable_segments
            .iter()
            .find(|segment| addr >= segment.start && addr < segment.end - 1)
        {
            let offset = (addr - segment.start) as usize;
            return Ok(u16::from_le_bytes([
                segment.data[offset],
                segment.data[offset + 1],
            ]));
        }

        for segment in self.object.segments() {
            let start = u32::try_from(segment.address()).map_err(|_| Fault::ObjectError)?;
            let end = start + u32::try_from(segment.size()).map_err(|_| Fault::ObjectError)?;

            if addr >= start && addr < end - 1 {
                let offset = (addr - start) as usize;
                let data = segment.data().map_err(|_| Fault::ObjectError)?;
                return Ok(u16::from_le_bytes([data[offset], data[offset + 1]]));
            }
        }

        if addr >= Self::STACK_MEM_BASE && addr < self.stack_top() - 1 {
            let offset = (addr - Self::STACK_MEM_BASE) as usize;
            return Ok(u16::from_le_bytes([
                self.stack[offset],
                self.stack[offset + 1],
            ]));
        }

        Err(Fault::InvalidAddress(addr))
    }

    #[inline]
    fn read_32(&mut self, addr: u32) -> Result<u32, Fault> {
        if let Some(segment) = self
            .writable_segments
            .iter()
            .find(|segment| addr >= segment.start && addr < segment.end - 3)
        {
            let offset = (addr - segment.start) as usize;
            return Ok(u32::from_le_bytes([
                segment.data[offset],
                segment.data[offset + 1],
                segment.data[offset + 2],
                segment.data[offset + 3],
            ]));
        }

        for segment in self.object.segments() {
            let start = u32::try_from(segment.address()).map_err(|_| Fault::ObjectError)?;
            let end = start + u32::try_from(segment.size()).map_err(|_| Fault::ObjectError)?;

            if addr >= start && addr < end - 3 {
                let offset = (addr - start) as usize;
                let data = segment.data().map_err(|_| Fault::ObjectError)?;
                return Ok(u32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]));
            }
        }

        if addr >= Self::STACK_MEM_BASE && addr < self.stack_top() - 3 {
            let offset = (addr - Self::STACK_MEM_BASE) as usize;
            return Ok(u32::from_le_bytes([
                self.stack[offset],
                self.stack[offset + 1],
                self.stack[offset + 2],
                self.stack[offset + 3],
            ]));
        }

        Err(Fault::InvalidAddress(addr))
    }

    #[inline]
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault> {
        if let Some(segment) = self
            .writable_segments
            .iter_mut()
            .find(|segment| addr >= segment.start && addr < segment.end)
        {
            segment.data[(addr - segment.start) as usize] = value;
            return Ok(());
        }

        if addr >= Self::STACK_MEM_BASE && addr < self.stack_top() {
            let offset = (addr - Self::STACK_MEM_BASE) as usize;
            self.stack[offset] = value;
            return Ok(());
        }

        for mmio in self.mmio.iter_mut() {
            if mmio.range().contains(&addr) {
                return mmio.write_8(addr, value);
            }
        }

        Err(Fault::InvalidAddress(addr))
    }

    #[inline]
    fn write_16(&mut self, addr: u32, value: u16) -> Result<(), Fault> {
        let bytes = value.to_le_bytes();

        if let Some(segment) = self
            .writable_segments
            .iter_mut()
            .find(|segment| addr >= segment.start && addr < segment.end - 1)
        {
            let offset = (addr - segment.start) as usize;
            segment.data[offset] = bytes[0];
            segment.data[offset + 1] = bytes[1];
            return Ok(());
        }

        if addr >= Self::STACK_MEM_BASE && addr < self.stack_top() - 1 {
            let offset = (addr - Self::STACK_MEM_BASE) as usize;
            self.stack[offset] = bytes[0];
            self.stack[offset + 1] = bytes[1];
            return Ok(());
        }

        Err(Fault::InvalidAddress(addr))
    }

    #[inline]
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault> {
        let bytes = value.to_le_bytes();

        if let Some(segment) = self
            .writable_segments
            .iter_mut()
            .find(|segment| addr >= segment.start && addr < (segment.end - 3))
        {
            let offset = (addr - segment.start) as usize;
            segment.data[offset] = bytes[0];
            segment.data[offset + 1] = bytes[1];
            segment.data[offset + 2] = bytes[2];
            segment.data[offset + 3] = bytes[3];
            return Ok(());
        }

        if addr >= Self::STACK_MEM_BASE && addr < self.stack_top() - 3 {
            let offset = (addr - Self::STACK_MEM_BASE) as usize;
            self.stack[offset] = bytes[0];
            self.stack[offset + 1] = bytes[1];
            self.stack[offset + 2] = bytes[2];
            self.stack[offset + 3] = bytes[3];
            return Ok(());
        }

        Err(Fault::InvalidAddress(addr))
    }
}

// --------------------------------------------------

pub trait MMIO {
    fn range(&self) -> core::ops::RangeInclusive<u32>;

    fn read_8(&mut self, addr: u32) -> Result<u8, Fault>;
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault>;

    // We first need to solve the problem of address ranging with 4byte registers,
    // for not we simply allow only 1byte registers.
    /* fn read_32(&mut self, addr: u32) -> Result<u32, Fault>;
    fn write_32(&mut self, addr: u32, value: u32) -> Result<(), Fault>; */
}

// --------------------------------------------------

#[cfg(feature = "std")]
/**
# Registers
| Offset | Name   | Description       | Access | Size |
|:------:|:-------|:------------------|:------:|:----:|
| 0x0    | INPUT  | Input characters  | RO     | 8 |
| 0x1    | OUTPUT | Output characters | WO     | 8 |
| 0x2    | FLAGS  | Flags register    | RW     | 8 |

## Flags
| Bits | Description | Type |
|:----:|:------------|:----:|
| 31:3 | Reserved | - |
| 2    | Flush output – write 1 to flush output buffer | SC |
| 1    | Output lock ­– 1 if output is locked, 0 otherwise | RW |
| 0    | Input lock – 1 if input is locked, 0 otherwise | RW |
*/
pub struct Stdio {
    input: std::io::Stdin,
    output: std::io::Stdout,

    input_lock: Option<std::io::StdinLock<'static>>,
    output_lock: Option<std::io::StdoutLock<'static>>,
}

#[cfg(feature = "std")]
impl Stdio {
    const BASE: u32 = 0xA000_0000;
    const INPUT_OFFSET: u32 = 0x0;
    const OUTPUT_OFFSET: u32 = 0x1;
    const FLAGS_OFFSET: u32 = 0x2;

    pub fn new() -> Self {
        Self {
            input: std::io::stdin(),
            output: std::io::stdout(),
            input_lock: None,
            output_lock: None,
        }
    }
}

#[cfg(feature = "std")]
impl MMIO for Stdio {
    #[inline(always)]
    fn range(&self) -> core::ops::RangeInclusive<u32> {
        Self::BASE..=(Self::BASE + Self::FLAGS_OFFSET)
    }

    #[inline]
    fn read_8(&mut self, addr: u32) -> Result<u8, Fault> {
        match addr.checked_sub(Self::BASE) {
            Some(Self::INPUT_OFFSET) => {
                // INPUT
                let mut buffer: [u8; 1] = [0];
                self.input
                    .read_exact(&mut buffer)
                    .map_err(|_| Fault::IOError)?;
                Ok(buffer[0])
            }
            Some(Self::OUTPUT_OFFSET) => Err(Fault::WriteOnlyRegister), // OUTPUT
            Some(Self::FLAGS_OFFSET) => {
                // FLAGS
                // Bit 0: input lock, Bit 1: output lock, Bit 2: write-only
                Ok((self.output_lock.is_some() as u8) << 1 | (self.input_lock.is_some() as u8))
            }
            _ => Err(Fault::InvalidAddress(addr)),
        }
    }

    #[inline]
    fn write_8(&mut self, addr: u32, value: u8) -> Result<(), Fault> {
        match addr.checked_sub(Self::BASE) {
            Some(Self::INPUT_OFFSET) => Err(Fault::ReadOnlyRegister), // INPUT
            Some(Self::OUTPUT_OFFSET) => {
                trace!("Writing 8 bits to OUTPUT: {:?}", value as char);
                // OUTPUT
                let bytes = value.to_le_bytes();
                self.output.write_all(&bytes).map_err(|_| Fault::IOError)?;
                Ok(())
            }
            Some(Self::FLAGS_OFFSET) => {
                // FLAGS
                // Read-only bits are ignored
                if (value & (1 << 1)) != 0 {
                    if self.output_lock.is_none() {
                        trace!("Acquiring output lock");
                        self.output_lock = Some(self.output.lock());
                    }
                } else {
                    if self.output_lock.is_some() {
                        trace!("Releasing output lock");
                        self.output_lock = None;
                    }
                };

                if (value & (1 << 2)) != 0 {
                    trace!("Flushing output buffer");
                    self.output.flush().map_err(|_| Fault::IOError)?
                }

                Ok(())
            }
            _ => Err(Fault::InvalidAddress(addr)),
        }
    }

    /* #[inline(always)]
    fn read_32(&mut self, _addr: u32) -> Result<u32, Fault> {
        Err(Fault::WrongRegisterSize {
            expected: 8,
            actual: 32,
        })
    }

    #[inline(always)]
    fn write_32(&mut self, _addr: u32, _value: u32) -> Result<(), Fault> {
        Err(Fault::WrongRegisterSize {
            expected: 8,
            actual: 32,
        })
    } */
}

#[cfg(feature = "std")]
impl Default for Stdio {
    fn default() -> Self {
        Self::new()
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

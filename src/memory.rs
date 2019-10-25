use crate::externals::Memory;
use wasmparser::MemoryImmediate;

const PAGE_SIZE: usize = 0x10000;

pub struct InstanceMemory {
    buffer: Vec<u8>,
    max: usize,
}

impl InstanceMemory {
    pub fn new(min: usize, max: usize) -> InstanceMemory {
        InstanceMemory {
            buffer: vec![0; min * PAGE_SIZE],
            max,
        }
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
}

fn combine_and_validate_offsets(memarg: &MemoryImmediate, offset: u32) -> u32 {
    let offset = memarg.offset.wrapping_add(offset);
    macro_rules! unaligned {
        () => {
            panic!("unaligned memory access");
        };
    }
    match memarg.flags {
        0 => (),
        1 => {
            if offset & 1 != 0 {
                unaligned!();
            }
        }
        2 => {
            if offset & 3 != 0 {
                unaligned!();
            }
        }
        3 => {
            if offset & 7 != 0 {
                unaligned!();
            }
        }
        4 => {
            if offset & 15 != 0 {
                unaligned!();
            }
        }
        _ => unimplemented!("combine_and_validate_offsets for align > 5"),
    }
    offset
}

impl Memory for InstanceMemory {
    fn current(&self) -> u32 {
        (self.buffer.len() / PAGE_SIZE) as u32
    }
    fn grow(&mut self, _delta: u32) -> u32 {
        unimplemented!("InstanceMemory::grow");
    }
    fn content_ptr(&self, memarg: &MemoryImmediate, offset: u32) -> *const u8 {
        let offset = combine_and_validate_offsets(memarg, offset) as usize;
        &self.buffer[offset]
    }
    fn content_ptr_mut(&mut self, memarg: &MemoryImmediate, offset: u32) -> *mut u8 {
        let offset = combine_and_validate_offsets(memarg, offset) as usize;
        &mut self.buffer[offset]
    }
}

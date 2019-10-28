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

#[inline]
fn combine_offsets(memarg: &MemoryImmediate, offset: u32) -> u32 {
    memarg.offset.wrapping_add(offset)
}

impl Memory for InstanceMemory {
    fn current(&self) -> u32 {
        (self.buffer.len() / PAGE_SIZE) as u32
    }
    fn grow(&mut self, delta: u32) -> u32 {
        let old_len = self.current();
        let new_len = old_len.checked_add(delta);
        if new_len.is_none() || new_len.unwrap() as usize > self.max {
            return !0;
        }
        let new_len = (new_len.unwrap() as usize) * PAGE_SIZE;
        self.buffer.resize(new_len, 0);
        old_len
    }
    fn content_ptr(&self, memarg: &MemoryImmediate, offset: u32) -> *const u8 {
        let offset = combine_offsets(memarg, offset) as usize;
        &self.buffer[offset]
    }
    fn content_ptr_mut(&mut self, memarg: &MemoryImmediate, offset: u32) -> *mut u8 {
        let offset = combine_offsets(memarg, offset) as usize;
        &mut self.buffer[offset]
    }
}

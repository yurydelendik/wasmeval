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

impl Memory for InstanceMemory {
    fn content_ptr(&self, memarg: &MemoryImmediate, offset: u32) -> *const u8 {
        unsafe {
            self.buffer
                .as_ptr()
                .offset(memarg.offset as isize + offset as isize)
        }
    }
    fn content_ptr_mut(&mut self, memarg: &MemoryImmediate, offset: u32) -> *mut u8 {
        unsafe {
            self.buffer
                .as_mut_ptr()
                .offset(memarg.offset as isize + offset as isize)
        }
    }
}

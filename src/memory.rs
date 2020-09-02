use crate::externals::Memory;
use std::cell::RefCell;
use wasmparser::MemoryImmediate;

const PAGE_SIZE: usize = 0x10000;

pub struct InstanceMemory {
    buffer: RefCell<Vec<u8>>,
    max: usize,
}

impl InstanceMemory {
    pub fn new(min: usize, max: usize) -> InstanceMemory {
        InstanceMemory {
            buffer: RefCell::new(vec![0; min * PAGE_SIZE]),
            max,
        }
    }
}

#[inline]
fn combine_offsets(memarg: &MemoryImmediate, offset: u32) -> usize {
    memarg.offset as usize + offset as usize
}

impl Memory for InstanceMemory {
    fn current(&self) -> u32 {
        (self.buffer.borrow().len() / PAGE_SIZE) as u32
    }
    fn grow(&self, delta: u32) -> u32 {
        let old_len = self.current();
        let new_len = old_len.checked_add(delta);
        if new_len.is_none() || new_len.unwrap() as usize > self.max {
            return !0;
        }
        let new_len = (new_len.unwrap() as usize) * PAGE_SIZE;
        self.buffer.borrow_mut().resize(new_len, 0);
        old_len
    }
    fn content_ptr(&self, memarg: &MemoryImmediate, offset: u32, size: u32) -> *const u8 {
        let offset = combine_offsets(memarg, offset);
        if offset + size as usize > self.buffer.borrow().len() {
            return std::ptr::null();
        }
        &self.buffer.borrow()[offset]
    }
    fn content_ptr_mut(&self, memarg: &MemoryImmediate, offset: u32, size: u32) -> *mut u8 {
        let offset = combine_offsets(memarg, offset);
        if offset + size as usize > self.buffer.borrow().len() {
            return std::ptr::null_mut();
        }
        &mut self.buffer.borrow_mut()[offset]
    }
    fn clone_from_slice(&self, offset: u32, chunk: &[u8]) {
        let offset = offset as usize;
        self.buffer.borrow_mut()[offset..(offset + chunk.len())].clone_from_slice(chunk);
    }
}

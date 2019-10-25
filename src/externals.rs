use std::cell::RefCell;
use std::rc::Rc;
use wasmparser::MemoryImmediate;

use crate::values::{Trap, Val};

pub trait Func {
    fn params_arity(&self) -> usize;
    fn results_arity(&self) -> usize;
    fn call(&self, params: &[Val]) -> Result<Box<[Val]>, Rc<Trap>>;
}

pub trait Memory {
    fn current(&self) -> u32;
    fn grow(&mut self, delta: u32) -> u32;
    fn content_ptr(&self, memarg: &MemoryImmediate, offset: u32) -> *const u8;
    fn content_ptr_mut(&mut self, memarg: &MemoryImmediate, offset: u32) -> *mut u8;
}

pub trait Global {
    fn content(&self) -> &Val;
    fn content_mut(&mut self) -> &mut Val;
}

#[derive(Clone)]
pub enum External<'a> {
    Func(Rc<RefCell<dyn Func + 'a>>),
    Memory(Rc<RefCell<dyn Memory>>),
    Global(Rc<RefCell<dyn Global>>),
}

impl<'a> External<'a> {
    pub fn func(&self) -> Option<&Rc<RefCell<dyn Func + 'a>>> {
        if let External::Func(f) = self {
            Some(f)
        } else {
            None
        }
    }
}

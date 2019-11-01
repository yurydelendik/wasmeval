use std::cell::RefCell;
use std::rc::Rc;
use wasmparser::MemoryImmediate;

use crate::values::{Trap, Val};

pub trait Func {
    fn params_arity(&self) -> usize;
    fn results_arity(&self) -> usize;
    fn call(&self, params: &[Val]) -> Result<Box<[Val]>, Trap>;
}

pub trait Memory {
    fn current(&self) -> u32;
    fn grow(&mut self, delta: u32) -> u32;
    fn content_ptr(&self, memarg: &MemoryImmediate, offset: u32, size: u32) -> *const u8;
    fn content_ptr_mut(&mut self, memarg: &MemoryImmediate, offset: u32, size: u32) -> *mut u8;
    fn clone_from_slice(&mut self, offset: u32, chunk: &[u8]);
}

pub trait Global {
    fn content(&self) -> &Val;
    fn content_mut(&mut self) -> &mut Val;
}

#[derive(Debug)]
pub struct TableOutOfBounds;

pub trait Table {
    fn get_func(&self, index: u32) -> Result<Option<Rc<RefCell<dyn Func>>>, TableOutOfBounds>;
    fn set_func(
        &mut self,
        index: u32,
        f: Option<Rc<RefCell<dyn Func>>>,
    ) -> Result<(), TableOutOfBounds>;
}

#[derive(Clone)]
pub enum External {
    Func(Rc<RefCell<dyn Func>>),
    Memory(Rc<RefCell<dyn Memory>>),
    Global(Rc<RefCell<dyn Global>>),
    Table(Rc<RefCell<dyn Table>>),
}

impl<'a> External {
    pub fn func(&self) -> Option<&Rc<RefCell<dyn Func>>> {
        if let External::Func(f) = self {
            Some(f)
        } else {
            None
        }
    }

    pub fn memory(&self) -> Option<&Rc<RefCell<dyn Memory>>> {
        if let External::Memory(m) = self {
            Some(m)
        } else {
            None
        }
    }

    pub fn table(&self) -> Option<&Rc<RefCell<dyn Table>>> {
        if let External::Table(t) = self {
            Some(t)
        } else {
            None
        }
    }
}

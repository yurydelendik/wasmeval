use std::rc::Rc;
use std::sync::Arc;
pub use wasmparser::MemoryImmediate;

use crate::values::{Trap, Val, ValType};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncType {
    pub params: Box<[ValType]>,
    pub returns: Box<[ValType]>,
}

impl From<wasmparser::FuncType> for FuncType {
    fn from(ty: wasmparser::FuncType) -> Self {
        let params = ty.params.into_iter().map(|t| t.clone().into()).collect();
        let returns = ty.returns.into_iter().map(|t| t.clone().into()).collect();
        Self { params, returns }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Limits {
    pub initial: u32,
    pub maximum: Option<u32>,
}

impl From<wasmparser::ResizableLimits> for Limits {
    fn from(l: wasmparser::ResizableLimits) -> Self {
        Self {
            initial: l.initial,
            maximum: l.maximum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryType {
    pub limits: Limits,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableType {
    pub element: ValType,
    pub limits: Limits,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlobalType {
    pub ty: ValType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExternType {
    Func(FuncType),
    Memory(MemoryType),
    Global(GlobalType),
    Table(TableType),
}

pub trait Func {
    fn ty(&self) -> &Arc<FuncType>;
    fn call(&self, stack: &mut [Val]) -> Result<(), Trap>;
    fn call_wrapped(&self, args: &[Val], results: &mut [Val]) -> Result<(), Trap> {
        let mut stack = vec![Default::default(); 10000];
        stack[..args.len()].clone_from_slice(args);
        self.call(&mut stack)?;
        results.clone_from_slice(&stack[..results.len()]);
        Ok(())
    }
}

pub trait Memory {
    fn current(&self) -> u32;
    fn grow(&self, delta: u32) -> u32;
    fn content_ptr(&self, memarg: &MemoryImmediate, offset: u32, size: u32) -> *const u8;
    fn content_ptr_mut(&self, memarg: &MemoryImmediate, offset: u32, size: u32) -> *mut u8;
    fn clone_from_slice(&self, offset: u32, chunk: &[u8]);
}

pub trait Global {
    fn content(&self) -> Val;
    fn set_content(&self, val: &Val);
}

#[derive(Debug)]
pub struct TableOutOfBounds;

pub trait Table {
    fn get_func(&self, index: u32) -> Result<Option<Rc<dyn Func>>, TableOutOfBounds>;
    fn get_func_with_type(
        &self,
        index: u32,
        _type_index: u32,
    ) -> Result<Option<Rc<dyn Func>>, TableOutOfBounds> {
        // TODO really check type
        self.get_func(index)
    }
    fn set_func(&self, index: u32, f: Option<Rc<dyn Func>>) -> Result<(), TableOutOfBounds>;
}

#[derive(Clone)]
pub enum External {
    Func(Rc<dyn Func>),
    Memory(Rc<dyn Memory>),
    Global(Rc<dyn Global>),
    Table(Rc<dyn Table>),
}

impl<'a> External {
    pub fn func(&self) -> Option<&Rc<dyn Func>> {
        if let External::Func(f) = self {
            Some(f)
        } else {
            None
        }
    }

    pub fn memory(&self) -> Option<&Rc<dyn Memory>> {
        if let External::Memory(m) = self {
            Some(m)
        } else {
            None
        }
    }

    pub fn table(&self) -> Option<&Rc<dyn Table>> {
        if let External::Table(t) = self {
            Some(t)
        } else {
            None
        }
    }
}

use std::rc::Rc;

use crate::externals::{Func, Global, Memory, Table};
use crate::instance::InstanceData;
use crate::module::ModuleData;
use crate::values::Val;

pub trait EvalContext {
    fn get_function(&self, index: u32) -> Rc<dyn Func>;
    fn get_global(&self, index: u32) -> Rc<dyn Global>;
    fn get_memory(&self) -> Rc<dyn Memory>;
    fn get_table(&self, index: u32) -> Rc<dyn Table>;
    fn get_type(&self, index: u32) -> Rc<dyn FuncType>;
}

impl EvalContext for Rc<InstanceData> {
    fn get_function(&self, index: u32) -> Rc<dyn Func> {
        self.funcs[index as usize].clone()
    }
    fn get_global(&self, index: u32) -> Rc<dyn Global> {
        self.globals[index as usize].clone()
    }
    fn get_memory(&self) -> Rc<dyn Memory> {
        const INDEX: usize = 0;
        self.memories[INDEX].clone()
    }
    fn get_table(&self, index: u32) -> Rc<dyn Table> {
        self.tables[index as usize].clone()
    }
    fn get_type(&self, index: u32) -> Rc<dyn FuncType> {
        Rc::new(ModuleFuncType(self.module_data.clone(), index as usize))
    }
}

pub trait FuncType {
    fn ty(&self) -> &wasmparser::FuncType;
}

struct ModuleFuncType(Rc<ModuleData>, usize);

impl FuncType for ModuleFuncType {
    fn ty(&self) -> &wasmparser::FuncType {
        &self.0.types[self.1]
    }
}

static mut FRAME_LOCALS: Option<Vec<Val>> = None;

pub(crate) struct Frame<'a> {
    context: &'a (dyn EvalContext + 'a),
    fp: usize,
    size: usize,
}

impl<'a> Frame<'a> {
    pub fn new(context: &'a (dyn EvalContext + 'a), size: usize) -> Self {
        let fp = unsafe {
            if FRAME_LOCALS.is_none() {
                FRAME_LOCALS = Some(Vec::with_capacity(0x1000));
            }
            let locals = FRAME_LOCALS.as_mut().unwrap();
            let len = locals.len();
            locals.resize_with(len + size, Default::default);
            len
        };
        Frame { context, fp, size }
    }
    pub fn get_local(&self, index: u32) -> &Val {
        debug_assert!((index as usize) < self.size);
        unsafe { &FRAME_LOCALS.as_ref().unwrap()[self.fp + index as usize] }
    }
    pub fn get_local_mut(&self, index: u32) -> &mut Val {
        debug_assert!((index as usize) < self.size);
        unsafe { &mut FRAME_LOCALS.as_mut().unwrap()[self.fp + index as usize] }
    }
    pub fn locals_mut(&self) -> &mut [Val] {
        unsafe { &mut FRAME_LOCALS.as_mut().unwrap()[self.fp..self.fp + self.size] }
    }
    pub fn context(&'a self) -> &'a (dyn EvalContext + 'a) {
        self.context
    }
}

impl<'a> Drop for Frame<'a> {
    fn drop(&mut self) {
        unsafe {
            let locals = FRAME_LOCALS.as_mut().unwrap();
            debug_assert!(self.fp + self.size == locals.len());
            locals.truncate(self.fp);
        }
    }
}

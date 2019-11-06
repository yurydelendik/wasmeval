use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::externals::{Func, Global, Memory, Table};
use crate::instance::InstanceData;
use crate::module::ModuleData;
use crate::values::Val;

pub trait EvalContext {
    fn get_function(&self, index: u32) -> Rc<RefCell<dyn Func>>;
    fn get_global(&self, index: u32) -> Rc<RefCell<dyn Global>>;
    fn get_memory(&self) -> Rc<RefCell<dyn Memory>>;
    fn get_table(&self, index: u32) -> Rc<RefCell<dyn Table>>;
    fn get_type(&self, index: u32) -> Rc<RefCell<dyn FuncType>>;
}

impl EvalContext for Rc<RefCell<InstanceData>> {
    fn get_function(&self, index: u32) -> Rc<RefCell<dyn Func>> {
        self.borrow().funcs[index as usize].clone()
    }
    fn get_global(&self, index: u32) -> Rc<RefCell<dyn Global>> {
        self.borrow().globals[index as usize].clone()
    }
    fn get_memory(&self) -> Rc<RefCell<dyn Memory>> {
        const INDEX: usize = 0;
        self.borrow().memories[INDEX].clone()
    }
    fn get_table(&self, index: u32) -> Rc<RefCell<dyn Table>> {
        self.borrow().tables[index as usize].clone()
    }
    fn get_type(&self, index: u32) -> Rc<RefCell<dyn FuncType>> {
        Rc::new(RefCell::new(ModuleFuncType(
            self.borrow().module_data.clone(),
            index as usize,
        )))
    }
}

pub trait FuncType {
    fn ty(&self) -> Ref<wasmparser::FuncType>;
}

struct ModuleFuncType(Rc<RefCell<ModuleData>>, usize);

impl FuncType for ModuleFuncType {
    fn ty(&self) -> Ref<wasmparser::FuncType> {
        Ref::map(self.0.borrow(), |m| &m.types[self.1])
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

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::externals::{Func, Global, Memory, Table};
use crate::instance::InstanceData;
use crate::module::ModuleData;
use crate::values::Val;

pub struct Local(pub Val);

pub(crate) struct EvalContext {
    instance_data: Rc<RefCell<InstanceData>>,
}

impl EvalContext {
    pub fn new(instance_data: Rc<RefCell<InstanceData>>) -> Self {
        EvalContext { instance_data }
    }
    pub fn get_function(&self, index: u32) -> Ref<Rc<RefCell<dyn Func>>> {
        Ref::map(self.instance_data.borrow(), |i| &i.funcs[index as usize])
    }
    pub fn get_global(&self, index: u32) -> Ref<Rc<RefCell<dyn Global>>> {
        Ref::map(self.instance_data.borrow(), |i| &i.globals[index as usize])
    }
    pub fn get_memory(&self) -> Ref<Rc<RefCell<dyn Memory>>> {
        const INDEX: usize = 0;
        Ref::map(self.instance_data.borrow(), |i| &i.memories[INDEX])
    }
    pub fn get_table(&self, index: u32) -> Ref<Rc<RefCell<dyn Table>>> {
        Ref::map(self.instance_data.borrow(), |i| &i.tables[index as usize])
    }
    pub fn get_type(&self, index: u32) -> ModuleFuncType {
        ModuleFuncType(
            self.instance_data.borrow().module_data.clone(),
            index as usize,
        )
    }
}

pub(crate) struct ModuleFuncType(Rc<RefCell<ModuleData>>, usize);

impl ModuleFuncType {
    pub fn ty(&self) -> Ref<wasmparser::FuncType> {
        Ref::map(self.0.borrow(), |m| &m.types[self.1])
    }
}

pub(crate) struct Frame<'a> {
    #[allow(dead_code)]
    context: &'a EvalContext,
    locals: Vec<Local>,
}

impl<'a> Frame<'a> {
    pub fn new(context: &'a EvalContext, locals: Vec<Local>) -> Self {
        Frame { context, locals }
    }
    pub fn get_local(&self, index: u32) -> &Val {
        &self.locals[index as usize].0
    }
    pub fn get_local_mut(&mut self, index: u32) -> &mut Val {
        &mut self.locals[index as usize].0
    }
}

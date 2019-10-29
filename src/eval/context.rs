use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::externals::{Func, Global, Memory, Table};
use crate::instance::InstanceData;
use crate::values::Val;

pub struct Local(pub Val);

pub(crate) struct EvalContext<'a> {
    instance_data: Rc<RefCell<InstanceData<'a>>>,
}

impl<'a> EvalContext<'a> {
    pub fn new(instance_data: Rc<RefCell<InstanceData<'a>>>) -> Self {
        EvalContext { instance_data }
    }
    pub fn get_function(&self, index: u32) -> Ref<Rc<RefCell<dyn Func + 'a>>> {
        Ref::map(self.instance_data.borrow(), |i| &i.funcs[index as usize])
    }
    pub fn get_global(&self, index: u32) -> Ref<Rc<RefCell<dyn Global>>> {
        Ref::map(self.instance_data.borrow(), |i| &i.globals[index as usize])
    }
    pub fn get_memory(&self) -> Ref<Rc<RefCell<dyn Memory>>> {
        const INDEX: usize = 0;
        Ref::map(self.instance_data.borrow(), |i| &i.memories[INDEX])
    }
    pub fn get_table(&self, index: u32) -> Ref<Rc<RefCell<dyn Table<'a> + 'a>>> {
        Ref::map(self.instance_data.borrow(), |i| &i.tables[index as usize])
    }
}

pub(crate) struct Frame<'a, 'e> {
    #[allow(dead_code)]
    context: &'a EvalContext<'e>,
    locals: Vec<Local>,
}

impl<'a, 'e> Frame<'a, 'e> {
    pub fn new(context: &'a EvalContext<'e>, locals: Vec<Local>) -> Self {
        Frame { context, locals }
    }
    pub fn get_local(&self, index: u32) -> &Val {
        &self.locals[index as usize].0
    }
    pub fn get_local_mut(&mut self, index: u32) -> &mut Val {
        &mut self.locals[index as usize].0
    }
}

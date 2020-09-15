use std::rc::Rc;
use std::sync::Arc;

use crate::externals::{Func, FuncType, Global, Memory, Table};
use crate::instance::InstanceData;

pub trait EvalContext {
    fn get_function(&self, index: u32) -> Rc<dyn Func>;
    fn get_global(&self, index: u32) -> Rc<dyn Global>;
    fn get_memory(&self) -> Rc<dyn Memory>;
    fn get_table(&self, index: u32) -> Rc<dyn Table>;
    fn get_type(&self, index: u32) -> Arc<FuncType>;
}

impl<'a> EvalContext for Rc<InstanceData> {
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
    fn get_type(&self, index: u32) -> Arc<FuncType> {
        self.module_data.types[index as usize].clone()
    }
}

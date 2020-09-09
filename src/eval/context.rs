use std::rc::Rc;

use crate::externals::{Func, Global, Memory, Table};
use crate::instance::InstanceData;
use crate::module::ModuleData;

pub trait EvalContext {
    fn get_function(&self, index: u32) -> Rc<dyn Func>;
    fn get_global(&self, index: u32) -> Rc<dyn Global>;
    fn get_memory(&self) -> Rc<dyn Memory>;
    fn get_table(&self, index: u32) -> Rc<dyn Table>;
    fn get_type(&self, index: u32) -> Rc<dyn FuncType>;
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

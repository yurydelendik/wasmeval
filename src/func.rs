use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::eval::BytecodeCache;
use crate::eval::{eval, EvalContext, EvalSource, Local};
use crate::externals::Func;
use crate::instance::InstanceData;
use crate::values::{get_default_value, Trap, Val};

pub(crate) struct InstanceFunction<'a> {
    instance_data: Rc<RefCell<InstanceData<'a>>>,
    defined_index: usize,
}

impl<'a> InstanceFunction<'a> {
    pub(crate) fn new(
        data: Rc<RefCell<InstanceData<'a>>>,
        defined_index: usize,
    ) -> InstanceFunction {
        InstanceFunction {
            instance_data: data,
            defined_index,
        }
    }
}

struct InstanceFunctionBody<'a> {
    bytecode: BytecodeCache<'a>,
}

impl<'a> InstanceFunctionBody<'a> {
    pub fn new(body: &'a wasmparser::FunctionBody<'a>) -> Self {
        let reader = body.get_operators_reader().expect("operators reader");
        let bytecode = BytecodeCache::new(reader);
        InstanceFunctionBody { bytecode }
    }
}

impl<'a> EvalSource for InstanceFunctionBody<'a> {
    fn bytecode(&self) -> &BytecodeCache {
        &self.bytecode
    }
}

impl<'a> Func for InstanceFunction<'a> {
    fn params_arity(&self) -> usize {
        let module_data = Ref::map(self.instance_data.borrow(), |data| {
            data.module_data.as_ref()
        });
        let func_type = module_data.borrow().func_types[self.defined_index];
        let func_type: Ref<wasmparser::FuncType> =
            Ref::map(module_data.borrow(), |data| &data.types[func_type as usize]);
        func_type.params.len()
    }

    fn results_arity(&self) -> usize {
        let module_data = Ref::map(self.instance_data.borrow(), |data| {
            data.module_data.as_ref()
        });
        let func_type = module_data.borrow().func_types[self.defined_index];
        let func_type: Ref<wasmparser::FuncType> =
            Ref::map(module_data.borrow(), |data| &data.types[func_type as usize]);
        func_type.returns.len()
    }

    fn call(&self, params: &[Val]) -> Result<Box<[Val]>, Trap> {
        let module_data = Ref::map(self.instance_data.borrow(), |data| {
            data.module_data.as_ref()
        });
        let body = Ref::map(module_data.borrow(), |data| {
            &data.func_bodies[self.defined_index]
        });
        let locals = read_body_locals(params, &body);
        let mut ctx = EvalContext::new(self.instance_data.clone());
        let body = Box::new(InstanceFunctionBody::new(&body));
        let result = eval(&mut ctx, &*body, locals, self.results_arity());
        result
    }
}

fn read_body_locals(params: &[Val], body: &wasmparser::FunctionBody) -> Vec<Local> {
    let mut locals = Vec::new();
    for param in params {
        locals.push(Local(param.clone()));
    }
    for local in body.get_locals_reader().expect("reader").into_iter() {
        let (count, ty) = local.expect("local def");
        for _ in 0..count {
            let local_val = get_default_value(ty.into());
            locals.push(Local(local_val));
        }
    }
    locals
}

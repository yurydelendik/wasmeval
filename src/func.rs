use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::eval::BytecodeCache;
use crate::eval::{eval, EvalContext, EvalSource, Local};
use crate::externals::Func;
use crate::instance::InstanceData;
use crate::module::ModuleData;
use crate::values::{get_default_value, Trap, Val};

pub(crate) struct InstanceFunction {
    instance_data: Rc<RefCell<InstanceData>>,
    defined_index: usize,
}

impl InstanceFunction {
    pub(crate) fn new(data: Rc<RefCell<InstanceData>>, defined_index: usize) -> InstanceFunction {
        InstanceFunction {
            instance_data: data,
            defined_index,
        }
    }
}

struct InstanceFunctionBody {
    bytecode: BytecodeCache,
}

impl InstanceFunctionBody {
    pub fn new(
        module_data: Rc<RefCell<ModuleData>>,
        body: &wasmparser::FunctionBody<'static>,
    ) -> Self {
        let reader = body.get_operators_reader().expect("operators reader");
        let bytecode = BytecodeCache::new(module_data, reader);
        InstanceFunctionBody { bytecode }
    }
}

impl EvalSource for InstanceFunctionBody {
    fn bytecode(&self) -> &BytecodeCache {
        &self.bytecode
    }
}

impl Func for InstanceFunction {
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
        let module_data = self.instance_data.borrow().module_data.clone();
        let body = Ref::map(module_data.borrow(), |data| {
            &data.func_bodies[self.defined_index]
        });
        let locals = read_body_locals(params, &body);
        let mut ctx = EvalContext::new(self.instance_data.clone());
        let module_data = self.instance_data.borrow().module_data.clone();
        let body = Box::new(InstanceFunctionBody::new(module_data, &body));
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

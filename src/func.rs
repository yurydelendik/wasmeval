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
    cache: RefCell<Option<InstanceFunctionBody>>,
}

impl InstanceFunction {
    pub(crate) fn new(data: Rc<RefCell<InstanceData>>, defined_index: usize) -> InstanceFunction {
        InstanceFunction {
            instance_data: data,
            defined_index,
            cache: RefCell::new(None),
        }
    }
}

struct InstanceFunctionBody {
    bytecode: BytecodeCache,
    locals: Vec<(u32, Val)>,
}

impl InstanceFunctionBody {
    pub fn new(
        module_data: Rc<RefCell<ModuleData>>,
        body: &wasmparser::FunctionBody<'static>,
    ) -> Self {
        let mut locals = Vec::new();
        for local in body.get_locals_reader().expect("reader").into_iter() {
            let (count, ty) = local.expect("local def");
            let local_val = get_default_value(ty.into());
            locals.push((count, local_val));
        }

        let reader = body.get_operators_reader().expect("operators reader");
        let bytecode = BytecodeCache::new(module_data, reader);

        InstanceFunctionBody { bytecode, locals }
    }

    pub fn create_locals(&self, params: &[Val]) -> Vec<Local> {
        let mut locals = Vec::new();
        for param in params {
            locals.push(Local(param.clone()));
        }
        for (count, val) in self.locals.iter() {
            for _ in 0..*count {
                locals.push(Local(val.clone()));
            }
        }
        locals
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
        if self.cache.borrow().is_none() {
            let module_data = self.instance_data.borrow().module_data.clone();
            let body = Ref::map(module_data.borrow(), |data| {
                &data.func_bodies[self.defined_index]
            });
            let module_data = self.instance_data.borrow().module_data.clone();
            let body = InstanceFunctionBody::new(module_data, &body);
            *self.cache.borrow_mut() = Some(body);
        }
        let body = self.cache.borrow();
        let locals = body.as_ref().unwrap().create_locals(params);
        let mut ctx = EvalContext::new(self.instance_data.clone());
        let result = eval(
            &mut ctx,
            body.as_ref().unwrap(),
            locals,
            self.results_arity(),
        );
        result
    }
}

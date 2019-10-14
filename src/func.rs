use std::cell::{Ref, RefCell};
use std::rc::Rc;

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

struct InstanceFunctionBody<'a>(&'a wasmparser::FunctionBody<'a>);

impl<'a> EvalSource for InstanceFunctionBody<'a> {
    fn create_reader(&self) -> wasmparser::OperatorsReader {
        self.0.get_operators_reader().expect("operators reader")
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

    fn call(&self, params: &[Val]) -> Result<Box<[Val]>, Rc<RefCell<Trap>>> {
        let module_data = Ref::map(self.instance_data.borrow(), |data| {
            data.module_data.as_ref()
        });
        let body = Ref::map(module_data.borrow(), |data| {
            &data.func_bodies[self.defined_index]
        });
        let locals = {
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
        };
        let mut ctx = EvalContext {
            instance_data: self.instance_data.clone(),
            locals,
            stack: Vec::new(),
        };
        eval(&mut ctx, &InstanceFunctionBody(&body));
        Ok(ctx.stack.into_boxed_slice())
    }
}

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::eval::BytecodeCache;
use crate::eval::{eval, EvalContext, EvalSource, Frame, Local};
use crate::externals::Func;
use crate::instance::InstanceData;
use crate::module::ModuleData;
use crate::values::{get_default_value, Trap, Val};

pub(crate) struct InstanceFunction {
    instance_data: Rc<RefCell<InstanceData>>,
    defined_index: usize,
    cache: RefCell<Option<InstanceFunctionBody>>,
    context: EvalContext,
}

impl InstanceFunction {
    pub(crate) fn new(data: Rc<RefCell<InstanceData>>, defined_index: usize) -> InstanceFunction {
        let context = EvalContext::new(data.clone());
        InstanceFunction {
            instance_data: data,
            defined_index,
            cache: RefCell::new(None),
            context,
        }
    }
}

struct InstanceFunctionBody {
    bytecode: BytecodeCache,
    locals: Vec<(u32, Val)>,
    frame_size: usize,
}

impl InstanceFunctionBody {
    pub fn new(
        module_data: Rc<RefCell<ModuleData>>,
        body: &wasmparser::FunctionBody<'static>,
        params_arity: usize,
        results_arity: usize,
        ctx: &EvalContext,
    ) -> Self {
        let mut locals = Vec::new();
        let mut frame_size = params_arity;
        for local in body.get_locals_reader().expect("reader").into_iter() {
            let (count, ty) = local.expect("local def");
            let local_val = get_default_value(ty.into());
            locals.push((count, local_val));
            frame_size += count as usize;
        }

        let reader = body.get_operators_reader().expect("operators reader");
        let bytecode = BytecodeCache::new(module_data, reader, ctx, results_arity);

        InstanceFunctionBody {
            bytecode,
            locals,
            frame_size,
        }
    }

    pub fn create_frame<'a>(&self, ctx: &'a EvalContext, params: &[Val]) -> Frame<'a> {
        let mut locals = Vec::with_capacity(self.frame_size);
        for param in params {
            locals.push(Local(param.clone()));
        }
        for (count, val) in self.locals.iter() {
            for _ in 0..*count {
                locals.push(Local(val.clone()));
            }
        }
        Frame::new(&ctx, locals)
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

    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap> {
        debug_assert!(self.results_arity() == results.len());
        if self.cache.borrow().is_none() {
            let module_data = self.instance_data.borrow().module_data.clone();
            let body = Ref::map(module_data.borrow(), |data| {
                &data.func_bodies[self.defined_index]
            });
            let module_data = self.instance_data.borrow().module_data.clone();
            let body = InstanceFunctionBody::new(
                module_data,
                &body,
                self.params_arity(),
                self.results_arity(),
                &self.context,
            );
            *self.cache.borrow_mut() = Some(body);
        }
        let body = self.cache.borrow();
        let mut frame = body.as_ref().unwrap().create_frame(&self.context, params);
        eval(&mut frame, body.as_ref().unwrap(), results)
    }
}

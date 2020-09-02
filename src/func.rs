use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::eval::BytecodeCache;
use crate::eval::{eval, EvalContext, EvalSource, Frame};
use crate::externals::Func;
use crate::instance::InstanceData;
use crate::module::ModuleData;
use crate::values::{get_default_value, Trap, Val};

pub(crate) trait InstanceFunctionSource {
    fn instance_data(&self) -> Rc<InstanceData>;
}

impl InstanceFunctionSource for Rc<RefCell<Weak<InstanceData>>> {
    fn instance_data(&self) -> Rc<InstanceData> {
        self.borrow().upgrade().unwrap()
    }
}

pub(crate) struct InstanceFunction {
    source: Box<dyn InstanceFunctionSource>,
    defined_index: usize,
    cache: RefCell<Option<InstanceFunctionBody>>,
}

impl InstanceFunction {
    pub(crate) fn new(
        source: Box<dyn InstanceFunctionSource>,
        defined_index: usize,
    ) -> InstanceFunction {
        InstanceFunction {
            source,
            defined_index,
            cache: RefCell::new(None),
        }
    }
}

struct InstanceFunctionBody {
    _module_data: Rc<ModuleData>,
    bytecode: BytecodeCache,
    locals: Vec<(u32, Val)>,
    frame_size: usize,
}

impl InstanceFunctionBody {
    pub fn new(
        module_data: Rc<ModuleData>,
        body: &wasmparser::FunctionBody<'static>,
        params_arity: usize,
        results_arity: usize,
        ctx: &dyn EvalContext,
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
        let bytecode = BytecodeCache::new(reader, ctx, results_arity);

        InstanceFunctionBody {
            _module_data: module_data,
            bytecode,
            locals,
            frame_size,
        }
    }

    pub fn create_frame<'a>(&self, ctx: &'a (dyn EvalContext + 'a), params: &[Val]) -> Frame<'a> {
        let f = Frame::new(ctx, self.frame_size);
        let locals = f.locals_mut();
        locals[..params.len()].clone_from_slice(params);
        let mut j = params.len();
        for (count, val) in self.locals.iter() {
            for _ in 0..*count {
                locals[j] = val.clone();
                j += 1;
            }
        }
        f
    }
}

impl EvalSource for InstanceFunctionBody {
    fn bytecode(&self) -> &BytecodeCache {
        &self.bytecode
    }
}

impl InstanceFunction {
    fn instance_data(&self) -> Rc<InstanceData> {
        self.source.instance_data()
    }
}

impl Func for InstanceFunction {
    fn params_arity(&self) -> usize {
        let instance_data = self.instance_data();
        let module_data = &instance_data.module_data;
        let func_type = module_data.func_types[self.defined_index];
        let func_type = &module_data.types[func_type as usize];
        func_type.params.len()
    }

    fn results_arity(&self) -> usize {
        let instance_data = self.instance_data();
        let module_data = &instance_data.module_data;
        let func_type = module_data.func_types[self.defined_index];
        let func_type = &module_data.types[func_type as usize];
        func_type.returns.len()
    }

    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap> {
        debug_assert!(self.results_arity() == results.len());
        let instance_data = self.instance_data();
        if self.cache.borrow().is_none() {
            let module_data = &instance_data.module_data;
            let body = &module_data.func_bodies[self.defined_index];
            let body = InstanceFunctionBody::new(
                module_data.clone(),
                &body,
                self.params_arity(),
                self.results_arity(),
                &instance_data,
            );
            *self.cache.borrow_mut() = Some(body);
        }
        let body = self.cache.borrow();
        let mut frame = body.as_ref().unwrap().create_frame(&instance_data, params);
        eval(&mut frame, body.as_ref().unwrap(), results)
    }
}

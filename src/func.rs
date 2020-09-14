use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::eval::BytecodeCache;
use crate::eval::{eval, EvalContext, EvalSource};
use crate::externals::{Func, FuncType};
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
    func_type: Rc<FuncType>,
}

impl InstanceFunction {
    pub(crate) fn new(
        source: Box<dyn InstanceFunctionSource>,
        defined_index: usize,
        func_type: Rc<FuncType>,
    ) -> InstanceFunction {
        InstanceFunction {
            source,
            defined_index,
            cache: RefCell::new(None),
            func_type,
        }
    }
}

struct InstanceFunctionBody {
    _module_data: Rc<ModuleData>,
    bytecode: BytecodeCache,
    locals: Vec<(u32, Val)>,
    params_arity: usize,
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
            params_arity,
            frame_size,
        }
    }

    pub fn init_frame<'a>(&self, stack: &'a mut [Val]) -> usize {
        let mut j = self.params_arity;
        for (count, val) in self.locals.iter() {
            for _ in 0..*count {
                stack[j] = val.clone();
                j += 1;
            }
        }
        self.frame_size
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
    fn ty(&self) -> &Rc<FuncType> {
        &self.func_type
    }

    fn call(&self, stack: &mut [Val]) -> Result<(), Trap> {
        let ty = self.ty();
        let instance_data = self.instance_data();
        if self.cache.borrow().is_none() {
            let module_data = &instance_data.module_data;
            let body = &module_data.func_bodies[self.defined_index];
            let body = InstanceFunctionBody::new(
                module_data.clone(),
                &body,
                ty.params.len(),
                ty.returns.len(),
                &instance_data,
            );
            *self.cache.borrow_mut() = Some(body);
        }
        let body = self.cache.borrow();
        let sp = body.as_ref().unwrap().init_frame(stack);
        eval(
            &instance_data,
            body.as_ref().unwrap(),
            ty.returns.len(),
            stack,
            sp,
        )
    }
}

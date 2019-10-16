use failure::{bail, Error};
use std::cell::RefCell;
use std::rc::Rc;
use wasmparser::{ExternalKind, ImportSectionEntryType, InitExpr, OperatorsReader};

use crate::eval::{eval_const, BytecodeCache, EvalContext, EvalSource};
use crate::externals::{External, Func, Global, Memory};
use crate::func::InstanceFunction;
use crate::global::InstanceGlobal;
use crate::memory::InstanceMemory;
use crate::module::{Module, ModuleData};

pub(crate) struct InstanceData<'a> {
    pub module_data: Rc<RefCell<ModuleData<'a>>>,
    pub memories: Vec<Rc<RefCell<dyn Memory>>>,
    pub globals: Vec<Rc<RefCell<dyn Global>>>,
    pub funcs: Vec<Rc<RefCell<dyn Func + 'a>>>,
}

pub struct Instance<'a> {
    data: Rc<RefCell<InstanceData<'a>>>,
    exports: Vec<External<'a>>,
}

impl<'a> Instance<'a> {
    pub fn new(module: &Module<'a>, externals: &[External<'a>]) -> Result<Instance<'a>, Error> {
        let module_data = module.data();
        if module_data.borrow().imports.len() != externals.len() {
            bail!("incompatible number of imports");
        }
        let mut memories = Vec::new();
        let mut funcs = Vec::new();
        let mut globals = Vec::new();
        for (import, external) in module_data.borrow().imports.iter().zip(externals) {
            match import.ty {
                ImportSectionEntryType::Function(_sig) => {
                    if let External::Func(f) = external {
                        funcs.push(f.clone());
                    } else {
                        bail!("incompatible func import");
                    }
                }
                ImportSectionEntryType::Memory(_mt) => {
                    if let External::Memory(m) = external {
                        memories.push(m.clone());
                    } else {
                        bail!("incompatible memory import");
                    }
                }
                ImportSectionEntryType::Global(_gt) => {
                    if let External::Global(g) = external {
                        globals.push(g.clone());
                    } else {
                        bail!("incompatible global import");
                    }
                }
                _ => bail!("unsupported import type: {:?}", import.ty),
            }
        }
        let data = Rc::new(RefCell::new(InstanceData {
            module_data: module_data.clone(),
            memories,
            globals,
            funcs,
        }));

        for m in module_data.borrow().memories.iter() {
            let limits = &m.limits;
            let memory = InstanceMemory::new(
                limits.initial as usize,
                limits.maximum.unwrap_or(65535) as usize,
            );
            data.borrow_mut()
                .memories
                .push(Rc::new(RefCell::new(memory)));
        }
        for g in module_data.borrow().globals.iter() {
            struct S<'s>(BytecodeCache<'s>);
            impl<'s> EvalSource for S<'s> {
                fn bytecode(&self) -> &BytecodeCache {
                    &self.0
                }
            }
            let bytecode = BytecodeCache::new(g.init_expr.get_operators_reader());
            let init_expr_source = S(bytecode);
            let mut ctx = EvalContext::new(data.clone());
            let init_val = eval_const(&mut ctx, &init_expr_source);
            let global = InstanceGlobal::new(init_val);
            data.borrow_mut()
                .globals
                .push(Rc::new(RefCell::new(global)));
        }
        for i in 0..module_data.borrow().func_types.len() {
            let f: InstanceFunction<'a> = InstanceFunction::new(data.clone(), i);
            data.borrow_mut().funcs.push(Rc::new(RefCell::new(f)));
        }

        let mut exports = Vec::new();
        for export in module_data.borrow().exports.iter() {
            let index = export.index as usize;
            let data = data.borrow();
            exports.push(match export.kind {
                ExternalKind::Function => External::Func(data.funcs[index].clone()),
                ExternalKind::Memory => External::Memory(data.memories[index].clone()),
                ExternalKind::Global => External::Global(data.globals[index].clone()),
                ExternalKind::Table => unimplemented!(),
            });
        }

        Ok(Instance { data, exports })
    }

    pub fn exports(&self) -> &[External<'a>] {
        &self.exports
    }
}

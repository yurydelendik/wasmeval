use anyhow::{bail, Error};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use wasmparser::{
    DataKind, ElementItem, ElementKind, ExternalKind, ImportSectionEntryType, InitExpr, MemoryType,
};

use crate::eval::{eval_const, BytecodeCache, EvalSource};
use crate::externals::{External, Func, Global, Memory, Table};
use crate::func::InstanceFunction;
use crate::global::InstanceGlobal;
use crate::memory::InstanceMemory;
use crate::module::{Module, ModuleData};
use crate::table::InstanceTable;
use crate::values::Val;

pub(crate) struct InstanceData {
    pub module_data: Arc<ModuleData>,
    pub memories: Vec<Rc<dyn Memory>>,
    pub globals: Vec<Rc<dyn Global>>,
    pub funcs: Vec<Rc<dyn Func>>,
    pub tables: Vec<Rc<dyn Table>>,
}

pub struct Instance {
    #[allow(dead_code)]
    data: Rc<InstanceData>,
    exports: Vec<External>,
}

impl Instance {
    pub fn new(module: &Module, externals: &[External]) -> Result<Instance, Error> {
        let module_data = module.data();
        if module_data.imports.len() != externals.len() {
            bail!("incompatible number of imports");
        }
        let mut memories = Vec::new();
        let mut funcs = Vec::new();
        let mut globals = Vec::new();
        let mut tables = Vec::new();
        for (import, external) in module_data.imports.iter().zip(externals) {
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
                ImportSectionEntryType::Table(_tt) => {
                    if let External::Table(t) = external {
                        tables.push(t.clone());
                    } else {
                        bail!("incompatible table import");
                    }
                }
                i => unreachable!("unsupported: {:?}", i),
            }
        }

        for m in module_data.memories.iter() {
            let limits = match m {
                MemoryType::M32 {
                    ref limits,
                    shared: false,
                } => limits,
                x => {
                    bail!("unsupported memory type {:?}", x);
                }
            };
            let memory = InstanceMemory::new(
                limits.initial as usize,
                limits.maximum.unwrap_or(65535) as usize,
            );
            memories.push(Rc::new(memory));
        }
        for t in module_data.tables.iter() {
            let limits = &t.limits;
            let table = InstanceTable::new(
                limits.initial as usize,
                limits.maximum.unwrap_or(0xffff_ffff) as usize,
            );
            tables.push(Rc::new(table));
        }

        let mut instance_data = Rc::new(InstanceData {
            module_data: module_data.clone(),
            memories: vec![],
            globals,
            funcs: vec![],
            tables: vec![],
        });
        for g in module_data.globals.iter() {
            let init_val = eval_init_expr(&instance_data, &g.init_expr);
            let global = InstanceGlobal::new(init_val);
            let data = Rc::get_mut(&mut instance_data).unwrap();
            data.globals.push(Rc::new(global));
        }

        let source: Rc<RefCell<Weak<InstanceData>>> = Rc::new(RefCell::new(Weak::new()));

        for (i, ft) in module_data.func_types.iter().enumerate() {
            let ty = module_data.types[*ft as usize].clone();
            let f: InstanceFunction = InstanceFunction::new(Box::new(source.clone()), i, ty);
            funcs.push(Rc::new(f));
        }

        for chunk in module_data.data.iter() {
            match chunk.kind {
                DataKind::Active {
                    memory_index,
                    ref init_expr,
                } => {
                    let start = eval_init_expr(&instance_data, init_expr).i32().unwrap() as u32;
                    // TODO check boundaries
                    memories[memory_index as usize].clone_from_slice(start, chunk.data);
                }
                DataKind::Passive => (),
            }
        }

        for element in module_data.elements.iter() {
            match element.kind {
                ElementKind::Active {
                    table_index,
                    ref init_expr,
                } => {
                    let start = eval_init_expr(&instance_data, init_expr).i32().unwrap() as u32;
                    for (i, item) in element
                        .items
                        .get_items_reader()
                        .expect("reader")
                        .into_iter()
                        .enumerate()
                    {
                        let index = item
                            .ok()
                            .and_then(|item| match item {
                                ElementItem::Func(index) => Some(index),
                                _ => None,
                            })
                            .expect("func_index");
                        let f = funcs[index as usize].clone();
                        tables[table_index as usize]
                            .set_func(start + i as u32, Some(f))
                            .expect("element set out-of-bounds");
                    }
                }
                ElementKind::Passive => (),
                ElementKind::Declared => (),
            }
        }
        let globals = Rc::try_unwrap(instance_data).ok().unwrap().globals;

        let mut exports = Vec::new();
        for export in module_data.exports.iter() {
            let index = export.index as usize;
            exports.push(match export.kind {
                ExternalKind::Function => External::Func(funcs[index].clone()),
                ExternalKind::Memory => External::Memory(memories[index].clone()),
                ExternalKind::Global => External::Global(globals[index].clone()),
                ExternalKind::Table => External::Table(tables[index].clone()),
                _ => {
                    panic!("TODO");
                }
            });
        }

        let instance_data = Rc::new(InstanceData {
            module_data: module_data.clone(),
            memories,
            globals,
            funcs,
            tables,
        });
        *source.borrow_mut() = Rc::downgrade(&instance_data);

        // Call start
        if let Some(start_func) = module_data.start_func {
            let f = instance_data.funcs[start_func as usize].clone();
            debug_assert!(f.ty().params.len() == 0 && f.ty().returns.len() == 0);
            let mut stack = vec![Default::default(); 10000];
            f.call(&mut stack)?;
        }

        Ok(Instance {
            data: instance_data,
            exports,
        })
    }

    pub fn exports(&self) -> &[External] {
        &self.exports
    }
}

fn eval_init_expr(data: &Rc<InstanceData>, init_expr: &InitExpr<'static>) -> Val {
    struct S(BytecodeCache);
    impl EvalSource for S {
        fn bytecode(&self) -> &BytecodeCache {
            &self.0
        }
    }
    let bytecode = BytecodeCache::new(init_expr.get_operators_reader(), data, 1);
    let init_expr_source = S(bytecode);
    eval_const(data, &init_expr_source)
}

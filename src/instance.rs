use failure::{bail, format_err, Error};
use std::cell::RefCell;
use std::rc::Rc;
use wasmparser::{DataKind, ElementKind, ExternalKind, ImportSectionEntryType, InitExpr};

use crate::eval::{eval_const, BytecodeCache, EvalContext, EvalSource};
use crate::externals::{External, Func, Global, Memory, Table};
use crate::func::InstanceFunction;
use crate::global::InstanceGlobal;
use crate::memory::InstanceMemory;
use crate::module::{Module, ModuleData};
use crate::table::InstanceTable;
use crate::values::Val;

pub(crate) struct InstanceData<'a> {
    pub module_data: Rc<RefCell<ModuleData<'a>>>,
    pub memories: Vec<Rc<RefCell<dyn Memory>>>,
    pub globals: Vec<Rc<RefCell<dyn Global>>>,
    pub funcs: Vec<Rc<RefCell<dyn Func + 'a>>>,
    pub tables: Vec<Rc<RefCell<dyn Table<'a> + 'a>>>,
}

pub struct Instance<'a> {
    #[allow(dead_code)]
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
        let mut tables = Vec::new();
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
                ImportSectionEntryType::Table(_tt) => {
                    if let External::Table(t) = external {
                        tables.push(t.clone());
                    } else {
                        bail!("incompatible table import");
                    }
                }
            }
        }
        let data = Rc::new(RefCell::new(InstanceData {
            module_data: module_data.clone(),
            memories,
            globals,
            funcs,
            tables,
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
        for t in module_data.borrow().tables.iter() {
            let limits = &t.limits;
            let table = InstanceTable::new(
                limits.initial as usize,
                limits.maximum.unwrap_or(0xffff_ffff) as usize,
            );
            data.borrow_mut().tables.push(Rc::new(RefCell::new(table)));
        }
        for g in module_data.borrow().globals.iter() {
            let init_val = eval_init_expr(&data, &g.init_expr);
            let global = InstanceGlobal::new(init_val);
            data.borrow_mut()
                .globals
                .push(Rc::new(RefCell::new(global)));
        }
        for i in 0..module_data.borrow().func_types.len() {
            let f: InstanceFunction<'a> = InstanceFunction::new(data.clone(), i);
            data.borrow_mut().funcs.push(Rc::new(RefCell::new(f)));
        }

        for chunk in module_data.borrow().data.iter() {
            match chunk.kind {
                DataKind::Active {
                    memory_index,
                    ref init_expr,
                } => {
                    let start = eval_init_expr(&data, init_expr).i32().unwrap() as u32;
                    // TODO check boundaries
                    data.borrow().memories[memory_index as usize]
                        .borrow_mut()
                        .clone_from_slice(start, chunk.data);
                }
                DataKind::Passive => (),
            }
        }

        for element in module_data.borrow().elements.iter() {
            match element.kind {
                ElementKind::Active {
                    table_index,
                    ref init_expr,
                } => {
                    let start = eval_init_expr(&data, init_expr).i32().unwrap() as u32;
                    for (i, item) in element
                        .items
                        .get_items_reader()
                        .expect("reader")
                        .into_iter()
                        .enumerate()
                    {
                        let index = item.expect("func_index");
                        let f = data.borrow().funcs[index as usize].clone();
                        data.borrow().tables[table_index as usize]
                            .borrow_mut()
                            .set_func(start + i as u32, f)
                            .expect("element set out-of-bounds");
                    }
                }
                ElementKind::Passive(_) => (),
            }
        }

        let mut exports = Vec::new();
        for export in module_data.borrow().exports.iter() {
            let index = export.index as usize;
            let data = data.borrow();
            exports.push(match export.kind {
                ExternalKind::Function => External::Func(data.funcs[index].clone()),
                ExternalKind::Memory => External::Memory(data.memories[index].clone()),
                ExternalKind::Global => External::Global(data.globals[index].clone()),
                ExternalKind::Table => External::Table(data.tables[index].clone()),
            });
        }

        if let Some(start_func) = module_data.borrow().start_func {
            let f = data.borrow().funcs[start_func as usize].clone();
            debug_assert!(f.borrow().params_arity() == 0 && f.borrow().results_arity() == 0);
            // TODO handle better start's trap
            f.borrow()
                .call(&[])
                .map_err(|_trap| format_err!("start function trapped"))?;
        }

        Ok(Instance { data, exports })
    }

    pub fn exports(&self) -> &[External<'a>] {
        &self.exports
    }
}

fn eval_init_expr(data: &Rc<RefCell<InstanceData>>, init_expr: &InitExpr<'_>) -> Val {
    struct S<'s>(BytecodeCache<'s>);
    impl<'s> EvalSource for S<'s> {
        fn bytecode(&self) -> &BytecodeCache {
            &self.0
        }
    }
    let bytecode = BytecodeCache::new(init_expr.get_operators_reader());
    let init_expr_source = S(bytecode);
    let mut ctx = EvalContext::new(data.clone());
    eval_const(&mut ctx, &init_expr_source)
}

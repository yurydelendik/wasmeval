use crate::eval::EvalContext;
use std::collections::HashMap;
use std::vec::Vec;
use wasmparser::OperatorsReader;

pub use wasmparser::Operator;

pub(crate) struct BytecodeCache {
    // TODO anchor Operator<'static> lifetime to ModuleData
    operators: Vec<Operator<'static>>,
    ends: HashMap<usize, usize>,
    elses: HashMap<usize, usize>,
    max_control_depth: usize,
    break_cache: HashMap<(usize, u32), BreakDestination>,
    params: HashMap<usize, usize>,
}

#[derive(Debug, Clone)]
pub enum BreakDestination {
    BlockEnd(usize, usize),
    LoopStart(usize, usize),
}

fn get_returns_count(context: &dyn EvalContext, ty: &wasmparser::TypeOrFuncType) -> usize {
    use wasmparser::{Type, TypeOrFuncType};
    match ty {
        TypeOrFuncType::Type(Type::EmptyBlockType) => 0,
        TypeOrFuncType::Type(_) => 1,
        TypeOrFuncType::FuncType(index) => {
            let ty = context.get_type(*index);
            let len = ty.returns.len();
            len
        }
    }
}

fn get_params_count(context: &dyn EvalContext, ty: &wasmparser::TypeOrFuncType) -> usize {
    use wasmparser::TypeOrFuncType;
    match ty {
        TypeOrFuncType::Type(_) => 0,
        TypeOrFuncType::FuncType(index) => {
            let ty = context.get_type(*index);
            let len = ty.params.len();
            len
        }
    }
}

impl BytecodeCache {
    pub fn new(
        reader: OperatorsReader<'static>,
        context: &dyn EvalContext,
        returns_count: usize,
    ) -> Self {
        let operators = reader
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("ops");
        let mut ends = HashMap::new();
        let mut elses = HashMap::new();
        let mut max_control_depth = 0;
        let mut break_cache = HashMap::new();
        let mut params = HashMap::new();

        let mut control: Vec<(usize, Option<usize>, Vec<(usize, u32)>)> = Vec::new();
        for i in (0..operators.len()).rev() {
            match operators[i] {
                Operator::End => {
                    control.push((i, None, Vec::new()));
                    max_control_depth = max_control_depth.max(control.len());
                }
                Operator::Loop { ref ty } => {
                    let (_, _, jumps) = control.pop().unwrap();
                    let params_count = get_params_count(context, ty);
                    params.insert(i, params_count);
                    for br in jumps {
                        break_cache.insert(br, BreakDestination::LoopStart(i + 1, params_count));
                    }
                }
                Operator::Block { ref ty } => {
                    let (end, _, jumps) = control.pop().unwrap();
                    let returns_count = get_returns_count(context, ty);
                    params.insert(i, get_params_count(context, ty));
                    for br in jumps {
                        break_cache.insert(br, BreakDestination::BlockEnd(end + 1, returns_count));
                    }
                }
                Operator::If { ref ty } => {
                    let (end, maybe_else, jumps) = control.pop().unwrap();
                    if let Some(el) = maybe_else {
                        elses.insert(i, el + 1);
                    } else {
                        elses.insert(i, end + 1);
                    }
                    params.insert(i, get_params_count(context, ty));
                    let returns_count = get_returns_count(context, ty);
                    for br in jumps {
                        break_cache.insert(br, BreakDestination::BlockEnd(end + 1, returns_count));
                    }
                }

                Operator::Else => {
                    control.last_mut().unwrap().1 = Some(i);
                    ends.insert(i, control.last_mut().unwrap().0 + 1);
                }

                Operator::Br { relative_depth } | Operator::BrIf { relative_depth } => {
                    let j = control.len() - 1 - relative_depth as usize;
                    control[j].2.push((i, relative_depth));
                }
                Operator::BrTable { ref table } => {
                    for entry in table.targets() {
                        let relative_depth = entry.unwrap().0;
                        let j = control.len() - 1 - relative_depth as usize;
                        control[j].2.push((i, relative_depth));
                    }
                }

                _ => (),
            }
        }

        assert!(control.len() == 1);
        let (end, _, jumps) = control.into_iter().next().unwrap();
        for br in jumps {
            break_cache.insert(br, BreakDestination::BlockEnd(end + 1, returns_count));
        }

        BytecodeCache {
            operators,
            ends,
            elses,
            max_control_depth,
            break_cache,
            params,
        }
    }

    pub fn break_to(&self, from: usize, depth: u32) -> BreakDestination {
        self.break_cache[&(from, depth)].clone()
    }

    pub fn skip_to_else(&self, from_if: usize) -> usize {
        self.elses[&from_if].clone()
    }

    pub fn skip_to_end(&self, from_else: usize) -> usize {
        self.ends[&from_else].clone()
    }

    pub fn len(&self) -> usize {
        self.operators.len()
    }

    pub fn operators(&self) -> &[Operator] {
        &self.operators
    }

    pub fn block_params_count(&self, i: usize) -> usize {
        self.params[&i].clone()
    }

    pub fn position(&self, i: usize) -> usize {
        // TODO real bytecode position
        i
    }

    pub fn max_control_depth(&self) -> usize {
        self.max_control_depth
    }
}

pub(crate) trait EvalSource {
    fn bytecode(&self) -> &BytecodeCache;
}

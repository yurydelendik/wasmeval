use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use wasmparser::{Operator, OperatorsReader};

use crate::externals::{Func, Global, Memory};
use crate::instance::InstanceData;
use crate::values::Val;

pub struct Local(pub Val);

pub(crate) struct EvalContext<'a> {
    pub instance_data: Rc<RefCell<InstanceData<'a>>>,
    pub locals: Vec<Local>,
    pub stack: Vec<Val>,
}

impl<'a> EvalContext<'a> {
    fn get_function(&self, index: u32) -> Rc<RefCell<dyn Func + 'a>> {
        self.instance_data.borrow().funcs[index as usize].clone()
    }
    fn get_global(&self, index: u32) -> Rc<RefCell<dyn Global>> {
        self.instance_data.borrow().globals[index as usize].clone()
    }
    fn get_memory(&self) -> Rc<RefCell<dyn Memory>> {
        const INDEX: usize = 0;
        self.instance_data.borrow().memories[INDEX].clone()
    }
}

pub(crate) trait EvalSource {
    fn create_reader(&self) -> OperatorsReader;
}

struct OperatorsCache<'a> {
    operators: Vec<Operator<'a>>,
    parents: HashMap<usize, usize>,
    ends: Vec<(usize, usize)>,
    loops: HashMap<usize, usize>,
    elses: HashMap<usize, usize>,
}

impl<'a> OperatorsCache<'a> {
    pub fn new(source: &'a dyn EvalSource) -> Self {
        let operators = source
            .create_reader()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("ops");
        let mut parents = HashMap::new();
        let mut ends = Vec::new();
        let mut loops = HashMap::new();
        let mut elses = HashMap::new();

        let mut control = Vec::new();
        for i in (0..operators.len()).rev() {
            match operators[i] {
                Operator::End => {
                    if let Some(&(last, _)) = control.last() {
                        parents.insert(i, last);
                        ends.push((i, last));
                    }
                    control.push((i, None));
                }
                Operator::Loop { .. } => {
                    let (end, _) = control.pop().unwrap();
                    ends.push((i, end));
                    loops.insert(end, i);
                }
                Operator::Block { .. } => {
                    let (end, _) = control.pop().unwrap();
                    ends.push((i, end));
                }
                Operator::If { .. } => {
                    let (end, maybe_else) = control.pop().unwrap();
                    if let Some(el) = maybe_else {
                        elses.insert(i, el);
                    }
                    ends.push((i, end));
                }
                Operator::Else => {
                    control.last_mut().unwrap().1 = Some(i);
                }
                _ => (),
            }
        }

        assert!(control.len() == 1);
        ends.push((0, control[0].0));
        ends.reverse();

        OperatorsCache {
            operators,
            parents,
            ends,
            loops,
            elses,
        }
    }

    pub fn break_to(&self, from: usize, depth: u32) -> usize {
        let mut end = match self.ends.binary_search_by_key(&from, |&(i, _)| i) {
            Ok(i) => self.ends[i].1,
            Err(i) => self.ends[i - 1].1,
        };
        for i in 0..depth {
            end = self.parents[&end];
        }
        (if let Some(i) = self.loops.get(&end) {
            *i
        } else {
            end
        }) + 1
    }

    pub fn len(&self) -> usize {
        self.operators.len()
    }

    pub fn operators(&self) -> &[Operator] {
        &self.operators
    }
}

pub(crate) fn eval<'a>(context: &'a mut EvalContext, source: &dyn EvalSource) {
    let cache = OperatorsCache::new(source);
    let operators = cache.operators();
    let mut i = 0;
    let mut stack: Vec<Val> = context.stack.split_off(0);

    loop {
        let op = &operators[i];

        let mut goto = |relative_depth| {
            i = cache.break_to(i, relative_depth);
        };

        match op {
            Operator::End => {
                if i + 1 >= cache.len() {
                    break;
                }
            }
            Operator::Loop { .. } => (),
            Operator::Block { .. } => (),
            Operator::BrIf { relative_depth } => {
                let c = stack.pop().unwrap().i32().unwrap();
                if c != 0 {
                    goto(*relative_depth);
                    continue;
                }
            }
            Operator::Br { relative_depth } => {
                goto(*relative_depth);
                continue;
            }
            Operator::Return => {
                break;
            }

            Operator::Call { function_index } => {
                let f = context.get_function(*function_index);
                let params = stack.split_off(stack.len() - f.borrow().params_arity());
                let result = f.borrow().call(&params);
                match result {
                    Ok(returns) => stack.extend_from_slice(&returns),
                    Err(_) => unimplemented!("call trap"),
                }
            }
            Operator::I32Const { value } => stack.push(Val::I32(*value)),
            Operator::GetGlobal { global_index } => {
                let g = context.get_global(*global_index);
                stack.push(g.borrow().content().clone());
            }
            Operator::SetGlobal { global_index } => {
                let g = context.get_global(*global_index);
                *g.borrow_mut().content_mut() = stack.pop().unwrap();
            }
            Operator::GetLocal { local_index } => {
                stack.push(context.locals[*local_index as usize].0.clone())
            }
            Operator::SetLocal { local_index } => {
                context.locals[*local_index as usize].0 = stack.pop().unwrap()
            }
            Operator::I32Sub => {
                let b = stack.pop().unwrap().i32().unwrap();
                let a = stack.pop().unwrap().i32().unwrap();
                stack.push(Val::I32(a - b));
            }
            Operator::I32Store { memarg } => {
                let val = stack.pop().unwrap().i32().unwrap();
                let offset = stack.pop().unwrap().i32().unwrap() as u32;
                let ptr = context
                    .get_memory()
                    .borrow_mut()
                    .content_ptr_mut(memarg, offset);
                unsafe {
                    *(ptr as *mut i32) = val;
                }
            }
            Operator::I32Load { memarg } => {
                let offset = stack.pop().unwrap().i32().unwrap() as u32;
                let ptr = context
                    .get_memory()
                    .borrow_mut()
                    .content_ptr(memarg, offset);
                let val = unsafe { *(ptr as *const i32) };
                stack.push(Val::I32(val));
            }
            Operator::I32GtU => {
                let b = stack.pop().unwrap().i32().unwrap() as u32;
                let a = stack.pop().unwrap().i32().unwrap() as u32;
                stack.push(Val::I32(if a > b { 1 } else { 0 }));
            }
            Operator::I32Eq => {
                let b = stack.pop().unwrap().i32().unwrap() as u32;
                let a = stack.pop().unwrap().i32().unwrap() as u32;
                stack.push(Val::I32(if a == b { 1 } else { 0 }));
            }
            Operator::I32RemU => {
                let b = stack.pop().unwrap().i32().unwrap() as u32;
                let a = stack.pop().unwrap().i32().unwrap() as u32;
                stack.push(Val::I32((a % b) as i32));
            }
            Operator::I32And => {
                let b = stack.pop().unwrap().i32().unwrap();
                let a = stack.pop().unwrap().i32().unwrap();
                stack.push(Val::I32(a & b));
            }
            Operator::I32Add => {
                let b = stack.pop().unwrap().i32().unwrap();
                let a = stack.pop().unwrap().i32().unwrap();
                stack.push(Val::I32(a + b));
            }

            x => unimplemented!("{:?}", x),
        }
        i += 1;
    }
    context.stack.extend(stack.into_iter());
}

pub(crate) fn eval_const<'a>(
    instance_data: Rc<RefCell<InstanceData<'a>>>,
    source: &dyn EvalSource,
) -> Val {
    let mut ctx = EvalContext {
        instance_data,
        locals: vec![],
        stack: vec![],
    };
    eval(&mut ctx, source);
    ctx.stack[0].clone()
}

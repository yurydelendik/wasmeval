use std::cell::RefCell;
use std::rc::Rc;

use crate::externals::{Func, Global, Memory};
use crate::instance::InstanceData;
use crate::values::Val;

pub(crate) use bytecode::{BytecodeCache, EvalSource, Operator};
pub(crate) use context::{EvalContext, Frame, Local};

mod bytecode;
mod context;

pub(crate) fn eval<'a>(
    context: &'a mut EvalContext,
    source: &dyn EvalSource,
    locals: Vec<Local>,
) -> Vec<Val> {
    let bytecode = source.bytecode();
    let operators = bytecode.operators();
    let mut i = 0;
    let mut frame = Frame::new(context, locals);
    let mut stack: Vec<Val> = Vec::new();

    macro_rules! val_ty {
        (i32) => {
            Val::I32
        };
        (i64) => {
            Val::I64
        };
        (f32) => {
            Val::F32
        };
        (f64) => {
            Val::F64
        };
    }
    macro_rules! push {
        ($e:expr; $ty:ident) => {
            stack.push(val_ty!($ty)($e))
        };
    }
    macro_rules! pop {
        ($ty:ident) => {
            stack.pop().unwrap().$ty().unwrap()
        };
    }
    macro_rules! step {
        (|$a:ident: $ty_a:ident, $b:ident: $ty_b:ident| -> $ty:ident $e:expr) => {{
            let $b = pop!($ty_b);
            let $a = pop!($ty_a);
            push!($e; $ty);
        }};
    }
    macro_rules! load {
        ($memarg:expr; $ty:ident) => {{
            let offset = pop!(i32) as u32;
            let ptr = context
                .get_memory()
                .borrow_mut()
                .content_ptr($memarg, offset);
            let val = unsafe { *(ptr as *const $ty) };
            push!(val; $ty);
        }};
    }
    macro_rules! store {
        ($memarg:expr; $ty:ident) => {{
            let val = pop!($ty);
            let offset = pop!(i32) as u32;
            let ptr = context
                .get_memory()
                .borrow_mut()
                .content_ptr_mut($memarg, offset);
            unsafe {
                *(ptr as *mut $ty) = val;
            }
        }};
    }

    loop {
        let op = &operators[i];

        macro_rules! break_to {
            ($depth:expr) => {{
                i = bytecode.break_to(i, $depth);
                continue;
            }};
        }

        match op {
            Operator::End => {
                if i + 1 >= bytecode.len() {
                    break;
                }
            }
            Operator::Loop { .. } => (),
            Operator::Block { .. } => (),
            Operator::BrIf { relative_depth } => {
                let c = stack.pop().unwrap().i32().unwrap();
                if c != 0 {
                    break_to!(*relative_depth);
                }
            }
            Operator::Br { relative_depth } => break_to!(*relative_depth),
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
            Operator::I32Const { value } => push!(*value; i32),
            Operator::GetGlobal { global_index } => {
                let g = context.get_global(*global_index);
                stack.push(g.borrow().content().clone());
            }
            Operator::SetGlobal { global_index } => {
                let g = context.get_global(*global_index);
                *g.borrow_mut().content_mut() = stack.pop().unwrap();
            }
            Operator::GetLocal { local_index } => stack.push(frame.get_local(*local_index).clone()),
            Operator::SetLocal { local_index } => {
                *frame.get_local_mut(*local_index) = stack.pop().unwrap();
            }
            Operator::I32Store { memarg } => {
                store!(memarg; i32);
            }
            Operator::I32Load { memarg } => {
                load!(memarg; i32);
            }
            Operator::I32GtU => step!(|a:i32, b:i32| -> i32 if a > b { 1 } else { 0 }),
            Operator::I32Eq => step!(|a:i32, b:i32| -> i32 if a == b { 1 } else { 0 }),
            Operator::I32RemU => {
                step!(|a: i32, b: i32| -> i32 { ((a as u32) % (b as u32)) as i32 })
            }
            Operator::I32And => step!(|a:i32, b:i32| -> i32 a & b),
            Operator::I32Add => step!(|a:i32, b:i32| -> i32 a + b),
            Operator::I32Sub => step!(|a:i32, b:i32| -> i32 a - b),

            x => unimplemented!("{:?}", x),
        }
        i += 1;
    }
    stack
}

pub(crate) fn eval_const<'a>(context: &'a mut EvalContext, source: &dyn EvalSource) -> Val {
    let result = eval(context, source, vec![]);
    debug_assert!(result.len() == 1);
    result.into_iter().next().unwrap()
}

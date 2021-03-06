use crate::values::{Trap, TrapKind, Val};
use std::rc::Rc;

use self::f32 as wasm_f32;
use self::f64 as wasm_f64;

pub(crate) use bytecode::{BreakDestination, BytecodeCache, EvalSource, Operator};

pub use context::EvalContext;

mod bytecode;
mod context;
mod f32;
mod f64;

#[allow(dead_code)]
const STACK_LIMIT: u32 = 10;

fn get_br_table_entry(table: &wasmparser::BrTable, i: u32) -> u32 {
    let i = table.len().min(i as usize);
    let it = table.targets();
    it.skip(i)
        .next()
        .expect("br_table entry")
        .expect("valid br_table entry")
        .0
}

struct EvalStack<'a> {
    stack: &'a mut [Val],
    sp: usize,
}
impl<'a> EvalStack<'a> {
    fn compress_stack_items(&mut self, start: usize, len: usize) {
        if len == 0 {
            return;
        }
        for i in (start + len)..self.sp {
            let v = ::std::mem::replace(&mut self.stack[i], Default::default());
            self.stack[i - len] = v;
        }
        self.sp -= len;
    }
    fn push(&mut self, v: Val) {
        self.stack[self.sp] = v;
        self.sp += 1;
    }
    fn pop(&mut self) -> Val {
        self.sp -= 1;
        let v = ::std::mem::replace(&mut self.stack[self.sp], Default::default());
        v
    }
    fn last(&self) -> Val {
        self.stack[self.sp - 1].clone()
    }
    fn last_mut(&mut self) -> &mut Val {
        &mut self.stack[self.sp - 1]
    }
    fn len(&self) -> usize {
        self.sp
    }
    fn local(&self, index: u32) -> Val {
        self.stack[index as usize].clone()
    }
    fn local_mut(&mut self, index: u32) -> &mut Val {
        &mut self.stack[index as usize]
    }
}

#[allow(unused_variables)]
pub(crate) fn eval<'a>(
    context: &'a (dyn EvalContext + 'a),
    source: &dyn EvalSource,
    return_arity: usize,
    stack_: &mut [Val],
    sp_: usize,
) -> Result<(), Trap> {
    let mut stack = EvalStack {
        stack: stack_,
        sp: sp_,
    };

    let bytecode = source.bytecode();
    let operators = bytecode.operators();
    let mut i = 0;
    let mut block_returns = Vec::with_capacity(bytecode.max_control_depth() + 1);
    let mut memory_cache: Option<Rc<_>> = None;
    block_returns.push(0);
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
    macro_rules! val_size {
        (i32) => {
            4
        };
        (i64) => {
            8
        };
        (f32) => {
            4
        };
        (f64) => {
            8
        };
    }
    macro_rules! rust_ty {
        (i32) => {
            i32
        };
        (i64) => {
            i64
        };
        (f32) => {
            u32
        };
        (f64) => {
            u64
        };
    }
    macro_rules! push {
        ($e:expr; $ty:ident) => {
            stack.push(val_ty!($ty)($e));
        };
    }
    macro_rules! pop {
        ($ty:ident) => {
            stack.pop().$ty().unwrap()
        };
    }
    macro_rules! trap {
        ($kind:expr) => {
            return Err(Trap::new($kind, bytecode.position(i)));
        };
    }
    macro_rules! step {
        (|$a:ident: $ty_a:ident| -> $ty:ident $e:expr) => {{
            let $a = pop!($ty_a);
            push!($e; $ty);
        }};
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
                .content_ptr($memarg, offset, val_size!($ty));
            if ptr.is_null() {
                trap!(TrapKind::OutOfBounds);
            }
            let val = unsafe { *(ptr as *const rust_ty!($ty)) };
            push!(val; $ty);
        }};
        ($memarg:expr; $ty:ident as $tt:ident) => {{
            let offset = pop!(i32) as u32;
            let ptr = context
                .get_memory()
                .content_ptr($memarg, offset, std::mem::size_of::<$tt>() as u32);
            if ptr.is_null() {
                trap!(TrapKind::OutOfBounds);
            }
            let val = unsafe { *(ptr as *const $tt) } as rust_ty!($ty);
            push!(val; $ty);
        }};
    }
    macro_rules! memory {
        () => {
            memory_cache.get_or_insert_with(|| context.get_memory().clone())
        };
    }
    macro_rules! store {
        ($memarg:expr; $ty:ident) => {{
            let val = pop!($ty);
            let offset = pop!(i32) as u32;
            let ptr = memory!().content_ptr_mut($memarg, offset, val_size!($ty));
            if ptr.is_null() {
                trap!(TrapKind::OutOfBounds);
            }
            unsafe {
                *(ptr as *mut rust_ty!($ty)) = val;
            }
        }};
        ($memarg:expr; $ty:ident as $tt:ident) => {{
            let val = pop!($ty) as $tt;
            let offset = pop!(i32) as u32;
            let ptr = memory!().content_ptr_mut($memarg, offset, std::mem::size_of::<$tt>() as u32);
            if ptr.is_null() {
                trap!(TrapKind::OutOfBounds);
            }
            unsafe {
                *(ptr as *mut $tt) = val;
            }
        }};
    }

    macro_rules! break_to {
        ($depth:expr) => {{
            let target_depth = block_returns.len() - $depth as usize - 1;
            match bytecode.break_to(i, $depth) {
                BreakDestination::BlockEnd(end, tail_len) => {
                    i = end;
                    let leave = block_returns[target_depth];
                    block_returns.truncate(target_depth);
                    stack.compress_stack_items(leave, stack.len() - tail_len - leave);
                }
                BreakDestination::LoopStart(start, tail_len) => {
                    i = start;
                    let leave = block_returns[target_depth];
                    block_returns.truncate(target_depth + 1);
                    stack.compress_stack_items(leave, stack.len() - tail_len - leave);
                }
            }
            continue;
        }};
    }
    macro_rules! call {
        ($f:expr) => {{
            // TODO better signature check
            let params_len = $f.ty().params.len();
            let returns_len = $f.ty().returns.len();
            let result = $f.call(&mut stack.stack[stack.sp - params_len..]);
            match result {
                Ok(()) => {
                    stack.sp = stack.sp + returns_len - params_len;
                }
                Err(trap) => {
                    return Err(trap);
                }
            }
        }};
    }
    macro_rules! op_notimpl {
        () => {
            trap!(TrapKind::User(format!(
                "operator not implemented {:?}",
                operators[i]
            )));
        };
    }

    // TODO validate stack state
    // TODO handle traps

    while i < operators.len() {
        match &operators[i] {
            Operator::Unreachable => {
                trap!(TrapKind::Unreachable);
            }
            Operator::Nop => (),
            Operator::Block { ty } | Operator::Loop { ty } => {
                block_returns.push(stack.len() - bytecode.block_params_count(i));
            }
            Operator::If { ty } => {
                let c = pop!(i32);
                block_returns.push(stack.len() - bytecode.block_params_count(i));
                if c == 0 {
                    i = bytecode.skip_to_else(i);
                    continue;
                }
            }
            Operator::Else => {
                i = bytecode.skip_to_end(i);
                block_returns.pop().unwrap();
                continue;
            }
            Operator::End => {
                if i + 1 >= bytecode.len() {
                    break;
                }
                block_returns.pop().unwrap();
            }
            Operator::Br { relative_depth } => break_to!(*relative_depth),
            Operator::BrIf { relative_depth } => {
                let c = pop!(i32);
                if c != 0 {
                    break_to!(*relative_depth);
                }
            }
            Operator::BrTable { table } => {
                let i = pop!(i32);
                break_to!(get_br_table_entry(table, i as u32));
            }
            Operator::Return => {
                break;
            }
            Operator::Call { function_index } => {
                let f = context.get_function(*function_index);
                call!(f)
            }
            Operator::CallIndirect { index, table_index } => {
                let func_index = pop!(i32) as u32;
                let table = context.get_table(*table_index);
                let ty = context.get_type(*index);
                let f = match table.get_func_with_type(func_index, *index) {
                    Ok(Some(f)) => f,
                    Ok(None) => trap!(TrapKind::Uninitialized),
                    Err(_) => trap!(TrapKind::UndefinedElement),
                };
                if f.ty().as_ref() != ty.as_ref() {
                    trap!(TrapKind::SignatureMismatch);
                }
                call!(f)
            }
            Operator::Drop => {
                stack.pop();
            }
            Operator::Select => {
                let c = pop!(i32);
                if c != 0 {
                    stack.pop();
                } else {
                    let v = stack.pop();
                    *stack.last_mut() = v;
                }
            }
            Operator::TypedSelect { .. } => op_notimpl!(),
            Operator::LocalGet { local_index } => stack.push(stack.local(*local_index)),
            Operator::LocalSet { local_index } => {
                *stack.local_mut(*local_index) = stack.pop();
            }
            Operator::LocalTee { local_index } => {
                *stack.local_mut(*local_index) = stack.last();
            }
            Operator::GlobalGet { global_index } => {
                let g = context.get_global(*global_index);
                stack.push(g.content());
            }
            Operator::GlobalSet { global_index } => {
                let g = context.get_global(*global_index);
                g.set_content(&stack.pop());
            }
            Operator::I32Load { memarg } => {
                load!(memarg; i32);
            }
            Operator::I64Load { memarg } => {
                load!(memarg; i64);
            }
            Operator::F32Load { memarg } => {
                load!(memarg; f32);
            }
            Operator::F64Load { memarg } => {
                load!(memarg; f64);
            }
            Operator::I32Load8S { memarg } => {
                load!(memarg; i32 as i8);
            }
            Operator::I32Load8U { memarg } => {
                load!(memarg; i32 as u8);
            }
            Operator::I32Load16S { memarg } => {
                load!(memarg; i32 as i16);
            }
            Operator::I32Load16U { memarg } => {
                load!(memarg; i32 as u16);
            }
            Operator::I64Load8S { memarg } => {
                load!(memarg; i64 as i8);
            }
            Operator::I64Load8U { memarg } => {
                load!(memarg; i64 as u8);
            }
            Operator::I64Load16S { memarg } => {
                load!(memarg; i64 as i16);
            }
            Operator::I64Load16U { memarg } => {
                load!(memarg; i64 as u16);
            }
            Operator::I64Load32S { memarg } => {
                load!(memarg; i64 as i32);
            }
            Operator::I64Load32U { memarg } => {
                load!(memarg; i64 as u32);
            }
            Operator::I32Store { memarg } => {
                store!(memarg; i32);
            }
            Operator::I64Store { memarg } => {
                store!(memarg; i64);
            }
            Operator::F32Store { memarg } => {
                store!(memarg; f32);
            }
            Operator::F64Store { memarg } => {
                store!(memarg; f64);
            }
            Operator::I32Store8 { memarg } => {
                store!(memarg; i32 as u8);
            }
            Operator::I32Store16 { memarg } => {
                store!(memarg; i32 as u16);
            }
            Operator::I64Store8 { memarg } => {
                store!(memarg; i64 as u8);
            }
            Operator::I64Store16 { memarg } => {
                store!(memarg; i64 as u16);
            }
            Operator::I64Store32 { memarg } => {
                store!(memarg; i64 as u32);
            }
            Operator::MemorySize { .. } => {
                let current = context.get_memory().current();
                push!(current as i32; i32)
            }
            Operator::MemoryGrow { .. } => {
                let delta = pop!(i32) as u32;
                let current = context.get_memory().grow(delta);
                push!(current as i32; i32)
            }
            Operator::I32Const { value } => push!(*value; i32),
            Operator::I64Const { value } => push!(*value; i64),
            Operator::F32Const { value } => push!(value.bits(); f32),
            Operator::F64Const { value } => push!(value.bits(); f64),
            Operator::I32Eqz => step!(|a:i32| -> i32 if a == 0 { 1 } else { 0 }),
            Operator::I32Eq => step!(|a:i32, b:i32| -> i32 if a == b { 1 } else { 0 }),
            Operator::I32Ne => step!(|a:i32, b:i32| -> i32 if a == b { 0 } else { 1 }),
            Operator::I32LtS => step!(|a:i32, b:i32| -> i32 if a < b { 1 } else { 0 }),
            Operator::I32LtU => {
                step!(|a:i32, b:i32| -> i32 if (a as u32) < b as u32 { 1 } else { 0 })
            }
            Operator::I32GtS => step!(|a:i32, b:i32| -> i32 if a > b { 1 } else { 0 }),
            Operator::I32GtU => {
                step!(|a:i32, b:i32| -> i32 if (a as u32) > b as u32 { 1 } else { 0 })
            }
            Operator::I32LeS => step!(|a:i32, b:i32| -> i32 if a <= b { 1 } else { 0 }),
            Operator::I32LeU => {
                step!(|a:i32, b:i32| -> i32 if (a as u32) <= b as u32 { 1 } else { 0 })
            }
            Operator::I32GeS => step!(|a:i32, b:i32| -> i32 if a >= b { 1 } else { 0 }),
            Operator::I32GeU => {
                step!(|a:i32, b:i32| -> i32 if (a as u32) >= b as u32 { 1 } else { 0 })
            }
            Operator::I64Eqz => step!(|a:i64| -> i32 if a == 0 { 1 } else { 0 }),
            Operator::I64Eq => step!(|a:i64, b:i64| -> i32 if a == b { 1 } else { 0 }),
            Operator::I64Ne => step!(|a:i64, b:i64| -> i32 if a == b { 0 } else { 1 }),
            Operator::I64LtS => step!(|a:i64, b:i64| -> i32 if a < b { 1 } else { 0 }),
            Operator::I64LtU => {
                step!(|a:i64, b:i64| -> i32 if (a as u64) < b as u64 { 1 } else { 0 })
            }
            Operator::I64GtS => step!(|a:i64, b:i64| -> i32 if a > b { 1 } else { 0 }),
            Operator::I64GtU => {
                step!(|a:i64, b:i64| -> i32 if (a as u64) > b as u64 { 1 } else { 0 })
            }
            Operator::I64LeS => step!(|a:i64, b:i64| -> i32 if a <= b { 1 } else { 0 }),
            Operator::I64LeU => {
                step!(|a:i64, b:i64| -> i32 if (a as u64) <= b as u64 { 1 } else { 0 })
            }
            Operator::I64GeS => step!(|a:i64, b:i64| -> i32 if a >= b { 1 } else { 0 }),
            Operator::I64GeU => {
                step!(|a:i64, b:i64| -> i32 if (a as u64) >= b as u64 { 1 } else { 0 })
            }
            Operator::F32Eq => step!(|a:f32, b:f32| -> i32 wasm_f32::eq(a, b)),
            Operator::F32Ne => step!(|a:f32, b:f32| -> i32 wasm_f32::ne(a, b)),
            Operator::F32Lt => step!(|a:f32, b:f32| -> i32 wasm_f32::lt(a, b)),
            Operator::F32Gt => step!(|a:f32, b:f32| -> i32 wasm_f32::gt(a, b)),
            Operator::F32Le => step!(|a:f32, b:f32| -> i32 wasm_f32::le(a, b)),
            Operator::F32Ge => step!(|a:f32, b:f32| -> i32 wasm_f32::ge(a, b)),
            Operator::F64Eq => step!(|a:f64, b:f64| -> i32 wasm_f64::eq(a, b)),
            Operator::F64Ne => step!(|a:f64, b:f64| -> i32 wasm_f64::ne(a, b)),
            Operator::F64Lt => step!(|a:f64, b:f64| -> i32 wasm_f64::lt(a, b)),
            Operator::F64Gt => step!(|a:f64, b:f64| -> i32 wasm_f64::gt(a, b)),
            Operator::F64Le => step!(|a:f64, b:f64| -> i32 wasm_f64::le(a, b)),
            Operator::F64Ge => step!(|a:f64, b:f64| -> i32 wasm_f64::ge(a, b)),
            Operator::I32Clz => step!(|a:i32| -> i32 a.leading_zeros() as i32),
            Operator::I32Ctz => step!(|a:i32| -> i32 a.trailing_zeros() as i32),
            Operator::I32Popcnt => step!(|a:i32| -> i32 a.count_ones() as i32),
            Operator::I32Add => step!(|a:i32, b:i32| -> i32 a.wrapping_add(b)),
            Operator::I32Sub => step!(|a:i32, b:i32| -> i32 a.wrapping_sub(b)),
            Operator::I32Mul => step!(|a:i32, b:i32| -> i32 a.wrapping_mul(b)),
            Operator::I32DivS => step!(|a: i32, b: i32| -> i32 {
                if let Some(c) = a.checked_div(b) {
                    c
                } else {
                    trap!(if b == 0 {
                        TrapKind::DivisionByZero
                    } else {
                        TrapKind::Overflow
                    });
                }
            }),
            Operator::I32DivU => step!(|a: i32, b: i32| -> i32 {
                if let Some(c) = (a as u32).checked_div(b as u32) {
                    c as i32
                } else {
                    trap!(if b == 0 {
                        TrapKind::DivisionByZero
                    } else {
                        TrapKind::Overflow
                    });
                }
            }),
            Operator::I32RemS => step!(|a: i32, b: i32| -> i32 {
                if let Some(c) = a.checked_rem(b) {
                    c
                } else if b == 0 {
                    trap!(TrapKind::DivisionByZero);
                } else {
                    assert!(b == -1);
                    0
                }
            }),
            Operator::I32RemU => step!(|a: i32, b: i32| -> i32 {
                if let Some(c) = (a as u32).checked_rem(b as u32) {
                    c as i32
                } else {
                    trap!(if b == 0 {
                        TrapKind::DivisionByZero
                    } else {
                        TrapKind::Overflow
                    });
                }
            }),
            Operator::I32And => step!(|a:i32, b:i32| -> i32 a & b),
            Operator::I32Or => step!(|a:i32, b:i32| -> i32 a | b),
            Operator::I32Xor => step!(|a:i32, b:i32| -> i32 a ^ b),
            Operator::I32Shl => step!(|a:i32, b:i32| -> i32 a.wrapping_shl(b as u32)),
            Operator::I32ShrS => step!(|a:i32, b:i32| -> i32 a.wrapping_shr(b as u32)),
            Operator::I32ShrU => {
                step!(|a:i32, b:i32| -> i32 (a as u32).wrapping_shr(b as u32) as i32)
            }
            Operator::I32Rotl => step!(|a:i32, b:i32| -> i32 a.rotate_left(b as u32)),
            Operator::I32Rotr => step!(|a:i32, b:i32| -> i32 a.rotate_right(b as u32)),
            Operator::I64Clz => step!(|a:i64| -> i64 a.leading_zeros() as i64),
            Operator::I64Ctz => step!(|a:i64| -> i64 a.trailing_zeros() as i64),
            Operator::I64Popcnt => step!(|a:i64| -> i64 a.count_ones() as i64),
            Operator::I64Add => step!(|a:i64, b:i64| -> i64 a.wrapping_add(b)),
            Operator::I64Sub => step!(|a:i64, b:i64| -> i64 a.wrapping_sub(b)),
            Operator::I64Mul => step!(|a:i64, b:i64| -> i64 a.wrapping_mul(b)),
            Operator::I64DivS => step!(|a: i64, b: i64| -> i64 {
                if let Some(c) = a.checked_div(b) {
                    c
                } else {
                    trap!(if b == 0 {
                        TrapKind::DivisionByZero
                    } else {
                        TrapKind::Overflow
                    });
                }
            }),
            Operator::I64DivU => step!(|a: i64, b: i64| -> i64 {
                if let Some(c) = (a as u64).checked_div(b as u64) {
                    c as i64
                } else {
                    trap!(if b == 0 {
                        TrapKind::DivisionByZero
                    } else {
                        TrapKind::Overflow
                    });
                }
            }),
            Operator::I64RemS => step!(|a: i64, b: i64| -> i64 {
                if let Some(c) = a.checked_rem(b) {
                    c
                } else if b == 0 {
                    trap!(TrapKind::DivisionByZero);
                } else {
                    assert!(b == -1);
                    0
                }
            }),
            Operator::I64RemU => step!(|a: i64, b: i64| -> i64 {
                if let Some(c) = (a as u64).checked_rem(b as u64) {
                    c as i64
                } else {
                    trap!(if b == 0 {
                        TrapKind::DivisionByZero
                    } else {
                        TrapKind::Overflow
                    });
                }
            }),
            Operator::I64And => step!(|a: i64, b: i64| -> i64 a & b),
            Operator::I64Or => step!(|a: i64, b: i64| -> i64 a | b),
            Operator::I64Xor => step!(|a: i64, b: i64| -> i64 a ^ b),
            Operator::I64Shl => step!(|a: i64, b: i64| -> i64 a.wrapping_shl(b as u32)),
            Operator::I64ShrS => step!(|a: i64, b: i64| -> i64 a.wrapping_shr(b as u32)),
            Operator::I64ShrU => {
                step!(|a: i64, b: i64| -> i64 (a as u64).wrapping_shr(b as u32) as i64)
            }
            Operator::I64Rotl => step!(|a:i64, b:i64| -> i64 a.rotate_left(b as u32)),
            Operator::I64Rotr => step!(|a:i64, b:i64| -> i64 a.rotate_right(b as u32)),
            Operator::F32Abs => step!(|a:f32| -> f32 wasm_f32::abs(a)),
            Operator::F32Neg => step!(|a:f32| -> f32 wasm_f32::neg(a)),
            Operator::F32Ceil => step!(|a:f32| -> f32 wasm_f32::ceil(a)),
            Operator::F32Floor => step!(|a:f32| -> f32 wasm_f32::floor(a)),
            Operator::F32Trunc => step!(|a:f32| -> f32 wasm_f32::trunc(a)),
            Operator::F32Nearest => step!(|a:f32| -> f32 wasm_f32::nearby(a)),
            Operator::F32Sqrt => step!(|a:f32| -> f32 wasm_f32::sqrt(a)),
            Operator::F32Add => step!(|a:f32, b:f32| -> f32 wasm_f32::add(a, b)),
            Operator::F32Sub => step!(|a:f32, b:f32| -> f32 wasm_f32::sub(a, b)),
            Operator::F32Mul => step!(|a:f32, b:f32| -> f32 wasm_f32::mul(a, b)),
            Operator::F32Div => step!(|a:f32, b:f32| -> f32 wasm_f32::div(a, b)),
            Operator::F32Min => step!(|a:f32, b:f32| -> f32 wasm_f32::min(a, b)),
            Operator::F32Max => step!(|a:f32, b:f32| -> f32 wasm_f32::max(a, b)),
            Operator::F32Copysign => step!(|a:f32, b:f32| -> f32 wasm_f32::copysign(a, b)),
            Operator::F64Abs => step!(|a:f64| -> f64 wasm_f64::abs(a)),
            Operator::F64Neg => step!(|a:f64| -> f64 wasm_f64::neg(a)),
            Operator::F64Ceil => step!(|a:f64| -> f64 wasm_f64::ceil(a)),
            Operator::F64Floor => step!(|a:f64| -> f64 wasm_f64::floor(a)),
            Operator::F64Trunc => step!(|a:f64| -> f64 wasm_f64::trunc(a)),
            Operator::F64Nearest => step!(|a:f64| -> f64 wasm_f64::nearby(a)),
            Operator::F64Sqrt => step!(|a:f64| -> f64 wasm_f64::sqrt(a)),
            Operator::F64Add => step!(|a:f64, b:f64| -> f64 wasm_f64::add(a, b)),
            Operator::F64Sub => step!(|a:f64, b:f64| -> f64 wasm_f64::sub(a, b)),
            Operator::F64Mul => step!(|a:f64, b:f64| -> f64 wasm_f64::mul(a, b)),
            Operator::F64Div => step!(|a:f64, b:f64| -> f64 wasm_f64::div(a, b)),
            Operator::F64Min => step!(|a:f64, b:f64| -> f64 wasm_f64::min(a, b)),
            Operator::F64Max => step!(|a:f64, b:f64| -> f64 wasm_f64::max(a, b)),
            Operator::F64Copysign => step!(|a:f64, b:f64| -> f64 wasm_f64::copysign(a, b)),
            Operator::I32WrapI64 => step!(|a:i64| -> i32 a as i32),
            Operator::I32TruncF32S => step!(|a:f32| -> i32 match wasm_f32::trunc_i32(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I32TruncF32U => step!(|a:f32| -> i32 match wasm_f32::trunc_u32(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I32TruncF64S => step!(|a:f64| -> i32 match wasm_f64::trunc_i32(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I32TruncF64U => step!(|a:f64| -> i32 match wasm_f64::trunc_u32(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I64ExtendI32S => step!(|a:i32| -> i64 (a as i64)),
            Operator::I64ExtendI32U => step!(|a:i32| -> i64 (a as u32 as i64)),
            Operator::I64TruncF32S => step!(|a:f32| -> i64 match wasm_f32::trunc_i64(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I64TruncF32U => step!(|a:f32| -> i64 match wasm_f32::trunc_u64(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I64TruncF64S => step!(|a:f64| -> i64 match wasm_f64::trunc_i64(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I64TruncF64U => step!(|a:f64| -> i64 match wasm_f64::trunc_u64(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::F32ConvertI32S => step!(|a:i32| -> f32 wasm_f32::from_i32(a)),
            Operator::F32ConvertI32U => step!(|a:i32| -> f32 wasm_f32::from_u32(a)),
            Operator::F32ConvertI64S => step!(|a:i64| -> f32 wasm_f32::from_i64(a)),
            Operator::F32ConvertI64U => step!(|a:i64| -> f32 wasm_f32::from_u64(a)),
            Operator::F32DemoteF64 => step!(|a:f64| -> f32 wasm_f32::from_f64(a)),
            Operator::F64ConvertI32S => step!(|a:i32| -> f64 wasm_f64::from_i32(a)),
            Operator::F64ConvertI32U => step!(|a:i32| -> f64 wasm_f64::from_u32(a)),
            Operator::F64ConvertI64S => step!(|a:i64| -> f64 wasm_f64::from_i64(a)),
            Operator::F64ConvertI64U => step!(|a:i64| -> f64 wasm_f64::from_u64(a)),
            Operator::F64PromoteF32 => step!(|a:f32| -> f64 wasm_f64::from_f32(a)),
            Operator::I32ReinterpretF32 => step!(|a:f32| -> i32 a as i32),
            Operator::I64ReinterpretF64 => step!(|a:f64| -> i64 a as i64),
            Operator::F32ReinterpretI32 => step!(|a:i32| -> f32 a as u32),
            Operator::F64ReinterpretI64 => step!(|a:i64| -> f64 a as u64),
            Operator::I32TruncSatF32S => step!(|a:f32| -> i32 wasm_f32::trunc_i32_sat(a)),
            Operator::I32TruncSatF32U => step!(|a:f32| -> i32 wasm_f32::trunc_u32_sat(a)),
            Operator::I32TruncSatF64S => step!(|a:f64| -> i32 wasm_f64::trunc_i32_sat(a)),
            Operator::I32TruncSatF64U => step!(|a:f64| -> i32 wasm_f64::trunc_u32_sat(a)),
            Operator::I64TruncSatF32S => step!(|a:f32| -> i64 wasm_f32::trunc_i64_sat(a)),
            Operator::I64TruncSatF32U => step!(|a:f32| -> i64 wasm_f32::trunc_u64_sat(a)),
            Operator::I64TruncSatF64S => step!(|a:f64| -> i64 wasm_f64::trunc_i64_sat(a)),
            Operator::I64TruncSatF64U => step!(|a:f64| -> i64 wasm_f64::trunc_u64_sat(a)),
            Operator::I32Extend16S => step!(|a: i32| -> i32 (a as i16) as i32),
            Operator::I32Extend8S => step!(|a: i32| -> i32 (a as i8) as i32),
            Operator::I64Extend32S => step!(|a: i64| -> i64 (a as i32) as i64),
            Operator::I64Extend16S => step!(|a: i64| -> i64 (a as i16) as i64),
            Operator::I64Extend8S => step!(|a: i64| -> i64 (a as i8) as i64),
            Operator::I32AtomicLoad { .. }
            | Operator::I32AtomicLoad16U { .. }
            | Operator::I32AtomicLoad8U { .. }
            | Operator::I64AtomicLoad { .. }
            | Operator::I64AtomicLoad32U { .. }
            | Operator::I64AtomicLoad16U { .. }
            | Operator::I64AtomicLoad8U { .. }
            | Operator::I32AtomicStore { .. }
            | Operator::I32AtomicStore16 { .. }
            | Operator::I32AtomicStore8 { .. }
            | Operator::I64AtomicStore { .. }
            | Operator::I64AtomicStore32 { .. }
            | Operator::I64AtomicStore16 { .. }
            | Operator::I64AtomicStore8 { .. }
            | Operator::I32AtomicRmwAdd { .. }
            | Operator::I32AtomicRmwSub { .. }
            | Operator::I32AtomicRmwAnd { .. }
            | Operator::I32AtomicRmwOr { .. }
            | Operator::I32AtomicRmwXor { .. }
            | Operator::I32AtomicRmw16AddU { .. }
            | Operator::I32AtomicRmw16SubU { .. }
            | Operator::I32AtomicRmw16AndU { .. }
            | Operator::I32AtomicRmw16OrU { .. }
            | Operator::I32AtomicRmw16XorU { .. }
            | Operator::I32AtomicRmw8AddU { .. }
            | Operator::I32AtomicRmw8SubU { .. }
            | Operator::I32AtomicRmw8AndU { .. }
            | Operator::I32AtomicRmw8OrU { .. }
            | Operator::I32AtomicRmw8XorU { .. }
            | Operator::I64AtomicRmwAdd { .. }
            | Operator::I64AtomicRmwSub { .. }
            | Operator::I64AtomicRmwAnd { .. }
            | Operator::I64AtomicRmwOr { .. }
            | Operator::I64AtomicRmwXor { .. }
            | Operator::I64AtomicRmw32AddU { .. }
            | Operator::I64AtomicRmw32SubU { .. }
            | Operator::I64AtomicRmw32AndU { .. }
            | Operator::I64AtomicRmw32OrU { .. }
            | Operator::I64AtomicRmw32XorU { .. }
            | Operator::I64AtomicRmw16AddU { .. }
            | Operator::I64AtomicRmw16SubU { .. }
            | Operator::I64AtomicRmw16AndU { .. }
            | Operator::I64AtomicRmw16OrU { .. }
            | Operator::I64AtomicRmw16XorU { .. }
            | Operator::I64AtomicRmw8AddU { .. }
            | Operator::I64AtomicRmw8SubU { .. }
            | Operator::I64AtomicRmw8AndU { .. }
            | Operator::I64AtomicRmw8OrU { .. }
            | Operator::I64AtomicRmw8XorU { .. }
            | Operator::I32AtomicRmwXchg { .. }
            | Operator::I32AtomicRmw16XchgU { .. }
            | Operator::I32AtomicRmw8XchgU { .. }
            | Operator::I32AtomicRmwCmpxchg { .. }
            | Operator::I32AtomicRmw16CmpxchgU { .. }
            | Operator::I32AtomicRmw8CmpxchgU { .. }
            | Operator::I64AtomicRmwXchg { .. }
            | Operator::I64AtomicRmw32XchgU { .. }
            | Operator::I64AtomicRmw16XchgU { .. }
            | Operator::I64AtomicRmw8XchgU { .. }
            | Operator::I64AtomicRmwCmpxchg { .. }
            | Operator::I64AtomicRmw32CmpxchgU { .. }
            | Operator::I64AtomicRmw16CmpxchgU { .. }
            | Operator::I64AtomicRmw8CmpxchgU { .. }
            | Operator::MemoryAtomicNotify { .. }
            | Operator::MemoryAtomicWait32 { .. }
            | Operator::MemoryAtomicWait64 { .. } => op_notimpl!(),
            Operator::AtomicFence { ref flags } => op_notimpl!(),
            Operator::RefNull { .. } | Operator::RefIsNull | Operator::RefFunc { .. } => {
                op_notimpl!()
            }
            Operator::V128Load { .. } | Operator::V128Store { .. } => op_notimpl!(),
            Operator::V128Const { .. }
            | Operator::I8x16Splat
            | Operator::I16x8Splat
            | Operator::I32x4Splat
            | Operator::I64x2Splat
            | Operator::F32x4Splat
            | Operator::F64x2Splat => op_notimpl!(),
            Operator::I8x16ExtractLaneS { lane }
            | Operator::I8x16ExtractLaneU { lane }
            | Operator::I16x8ExtractLaneS { lane }
            | Operator::I16x8ExtractLaneU { lane }
            | Operator::I32x4ExtractLane { lane }
            | Operator::I8x16ReplaceLane { lane }
            | Operator::I16x8ReplaceLane { lane }
            | Operator::I32x4ReplaceLane { lane }
            | Operator::I64x2ExtractLane { lane }
            | Operator::I64x2ReplaceLane { lane }
            | Operator::F32x4ExtractLane { lane }
            | Operator::F32x4ReplaceLane { lane }
            | Operator::F64x2ExtractLane { lane }
            | Operator::F64x2ReplaceLane { lane } => op_notimpl!(),
            Operator::F32x4Eq
            | Operator::F32x4Ne
            | Operator::F32x4Lt
            | Operator::F32x4Gt
            | Operator::F32x4Le
            | Operator::F32x4Ge
            | Operator::F64x2Eq
            | Operator::F64x2Ne
            | Operator::F64x2Lt
            | Operator::F64x2Gt
            | Operator::F64x2Le
            | Operator::F64x2Ge
            | Operator::F32x4Add
            | Operator::F32x4Sub
            | Operator::F32x4Mul
            | Operator::F32x4Div
            | Operator::F32x4Min
            | Operator::F32x4Max
            | Operator::F64x2Add
            | Operator::F64x2Sub
            | Operator::F64x2Mul
            | Operator::F64x2Div
            | Operator::F64x2Min
            | Operator::F64x2Max
            | Operator::I8x16Eq
            | Operator::I8x16Ne
            | Operator::I8x16LtS
            | Operator::I8x16LtU
            | Operator::I8x16GtS
            | Operator::I8x16GtU
            | Operator::I8x16LeS
            | Operator::I8x16LeU
            | Operator::I8x16GeS
            | Operator::I8x16GeU
            | Operator::I16x8Eq
            | Operator::I16x8Ne
            | Operator::I16x8LtS
            | Operator::I16x8LtU
            | Operator::I16x8GtS
            | Operator::I16x8GtU
            | Operator::I16x8LeS
            | Operator::I16x8LeU
            | Operator::I16x8GeS
            | Operator::I16x8GeU
            | Operator::I32x4Eq
            | Operator::I32x4Ne
            | Operator::I32x4LtS
            | Operator::I32x4LtU
            | Operator::I32x4GtS
            | Operator::I32x4GtU
            | Operator::I32x4LeS
            | Operator::I32x4LeU
            | Operator::I32x4GeS
            | Operator::I32x4GeU
            | Operator::V128And
            | Operator::V128Or
            | Operator::V128Xor
            | Operator::I8x16Add
            | Operator::I8x16AddSaturateS
            | Operator::I8x16AddSaturateU
            | Operator::I8x16Sub
            | Operator::I8x16SubSaturateS
            | Operator::I8x16SubSaturateU
            | Operator::I16x8Add
            | Operator::I16x8AddSaturateS
            | Operator::I16x8AddSaturateU
            | Operator::I16x8Sub
            | Operator::I16x8SubSaturateS
            | Operator::I16x8SubSaturateU
            | Operator::I16x8Mul
            | Operator::I32x4Add
            | Operator::I32x4Sub
            | Operator::I32x4Mul
            | Operator::I64x2Add
            | Operator::I64x2Sub
            | Operator::F32x4Abs
            | Operator::F32x4Neg
            | Operator::F32x4Sqrt
            | Operator::F64x2Abs
            | Operator::F64x2Neg
            | Operator::F64x2Sqrt
            | Operator::F32x4ConvertI32x4S
            | Operator::F32x4ConvertI32x4U
            | Operator::V128Not
            | Operator::I8x16Neg
            | Operator::I8x16Abs
            | Operator::I16x8Neg
            | Operator::I16x8Abs
            | Operator::I32x4Neg
            | Operator::I32x4Abs
            | Operator::I64x2Neg
            | Operator::I32x4TruncSatF32x4S
            | Operator::I32x4TruncSatF32x4U
            | Operator::V128Bitselect
            | Operator::I8x16AnyTrue
            | Operator::I8x16AllTrue
            | Operator::I16x8AnyTrue
            | Operator::I16x8AllTrue
            | Operator::I32x4AnyTrue
            | Operator::I32x4AllTrue
            | Operator::I8x16Shl
            | Operator::I8x16ShrS
            | Operator::I8x16ShrU
            | Operator::I8x16Bitmask
            | Operator::I8x16MinS
            | Operator::I8x16MinU
            | Operator::I8x16MaxS
            | Operator::I8x16MaxU
            | Operator::I16x8Shl
            | Operator::I16x8ShrS
            | Operator::I16x8ShrU
            | Operator::I16x8Bitmask
            | Operator::I16x8MinS
            | Operator::I16x8MinU
            | Operator::I16x8MaxS
            | Operator::I16x8MaxU
            | Operator::I32x4Shl
            | Operator::I32x4ShrS
            | Operator::I32x4ShrU
            | Operator::I32x4Bitmask
            | Operator::I32x4MinS
            | Operator::I32x4MinU
            | Operator::I32x4MaxS
            | Operator::I32x4MaxU
            | Operator::I64x2Shl
            | Operator::I64x2ShrS
            | Operator::I64x2ShrU
            | Operator::V8x16Swizzle => op_notimpl!(),
            Operator::V8x16Shuffle { ref lanes } => op_notimpl!(),
            Operator::V8x16LoadSplat { .. }
            | Operator::V16x8LoadSplat { .. }
            | Operator::V32x4LoadSplat { .. }
            | Operator::V64x2LoadSplat { .. } => op_notimpl!(),
            Operator::MemoryCopy { .. } | Operator::MemoryFill { .. } => op_notimpl!(),
            Operator::MemoryInit { segment, .. }
            | Operator::DataDrop { segment }
            | Operator::ElemDrop { segment } => op_notimpl!(),
            Operator::TableInit { table, segment } => op_notimpl!(),
            Operator::TableCopy {
                dst_table,
                src_table,
            } => op_notimpl!(),
            Operator::TableGet { table }
            | Operator::TableSet { table }
            | Operator::TableGrow { table }
            | Operator::TableSize { table }
            | Operator::TableFill { table } => op_notimpl!(),
            Operator::V128AndNot
            | Operator::I64x2Mul
            | Operator::I8x16NarrowI16x8S
            | Operator::I8x16NarrowI16x8U
            | Operator::I16x8NarrowI32x4S
            | Operator::I16x8NarrowI32x4U
            | Operator::I16x8WidenLowI8x16S
            | Operator::I16x8WidenHighI8x16S
            | Operator::I16x8WidenLowI8x16U
            | Operator::I16x8WidenHighI8x16U
            | Operator::I32x4WidenLowI16x8S
            | Operator::I32x4WidenHighI16x8S
            | Operator::I32x4WidenLowI16x8U
            | Operator::I32x4WidenHighI16x8U
            | Operator::I16x8Load8x8S { .. }
            | Operator::I16x8Load8x8U { .. }
            | Operator::I32x4Load16x4S { .. }
            | Operator::I32x4Load16x4U { .. }
            | Operator::I64x2Load32x2S { .. }
            | Operator::I64x2Load32x2U { .. }
            | Operator::I8x16RoundingAverageU
            | Operator::I16x8RoundingAverageU => op_notimpl!(),
            Operator::ReturnCall { .. } | Operator::ReturnCallIndirect { .. } => op_notimpl!(),
        }
        i += 1;
    }
    stack.compress_stack_items(0, stack.len() - return_arity);
    Ok(())
}

pub(crate) fn eval_const<'a>(context: &'a (dyn EvalContext + 'a), source: &dyn EvalSource) -> Val {
    const MAX_CONST_EVAL_STACK: usize = 10;
    let mut stack = vec![Default::default(); MAX_CONST_EVAL_STACK];
    let result = eval(context, source, 1, &mut stack, 0);
    match result {
        Ok(()) => stack.into_iter().next().unwrap(),
        Err(_) => {
            panic!("trap duing eval_const");
        }
    }
}

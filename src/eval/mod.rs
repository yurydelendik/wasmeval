use crate::values::{Trap, TrapKind, Val};
use std::cell::RefCell;
use std::rc::Rc;
use std::slice;

pub(crate) use bytecode::{BreakDestination, BytecodeCache, EvalSource, Operator};
pub(crate) use context::{EvalContext, Frame};

mod bytecode;
mod context;
mod floats;

#[allow(dead_code)]
const STACK_LIMIT: u32 = 10;

fn get_br_table_entry(table: &wasmparser::BrTable, i: u32) -> u32 {
    let i = table.len().min(i as usize);
    let it = table.clone().into_iter();
    it.skip(i).next().expect("valid br_table entry")
}

struct EvalStack(Vec<Val>);

impl EvalStack {
    #[allow(dead_code)]
    pub fn new() -> Self {
        //EvalStack(Vec::new())
        EvalStack(Vec::with_capacity(100))
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn last(&self) -> &Val {
        self.0.last().unwrap()
    }
    pub fn last_mut(&mut self) -> &mut Val {
        self.0.last_mut().unwrap()
    }
    pub fn pop(&mut self) -> Val {
        self.0.pop().unwrap()
    }
    pub fn push(&mut self, val: Val) {
        self.0.push(val);
    }
    pub fn truncate(&mut self, at: usize) {
        self.0.truncate(at);
    }
    pub fn resize_with_default(&mut self, len: usize) {
        self.0.resize_with(len, Default::default);
    }
    pub fn remove_items(&mut self, index: usize, len: usize) {
        match len {
            0 => (),
            1 => drop(self.0.remove(index)),
            _ => {
                let vec = &mut self.0;
                if index + len < vec.len() {
                    vec[index + len..].reverse();
                    vec[index..].reverse();
                }
                vec.truncate(vec.len() - len);
            }
        }
    }
    pub fn tail(&self, len: usize) -> &[Val] {
        &self.0[self.0.len() - len..]
    }
    pub fn item_ptr(&self, index: usize) -> *const Val {
        &self.0[index]
    }
    pub fn item_mut_ptr(&mut self, index: usize) -> *mut Val {
        &mut self.0[index]
    }
}

#[allow(unused_variables)]
pub(crate) fn eval<'a>(
    frame: &'a mut Frame,
    source: &dyn EvalSource,
    returns: &mut [Val],
) -> Result<(), Trap> {
    let return_arity = returns.len();
    let bytecode = source.bytecode();
    let operators = bytecode.operators();
    let mut i = 0;
    let mut stack = EvalStack::new();
    let mut block_returns = Vec::with_capacity(bytecode.max_control_depth() + 1);
    let mut memory_cache: Option<Rc<RefCell<_>>> = None;
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
            stack.push(val_ty!($ty)($e))
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
            let ptr = frame.context()
                .get_memory()
                .borrow_mut()
                .content_ptr($memarg, offset, val_size!($ty));
            if ptr.is_null() {
                trap!(TrapKind::OutOfBounds);
            }
            let val = unsafe { *(ptr as *const rust_ty!($ty)) };
            push!(val; $ty);
        }};
        ($memarg:expr; $ty:ident as $tt:ident) => {{
            let offset = pop!(i32) as u32;
            let ptr = frame.context()
                .get_memory()
                .borrow_mut()
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
            memory_cache.get_or_insert_with(|| frame.context().get_memory().clone())
        };
    }
    macro_rules! store {
        ($memarg:expr; $ty:ident) => {{
            let val = pop!($ty);
            let offset = pop!(i32) as u32;
            let ptr = memory!()
                .borrow_mut()
                .content_ptr_mut($memarg, offset, val_size!($ty));
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
            let ptr = memory!().borrow_mut().content_ptr_mut(
                $memarg,
                offset,
                std::mem::size_of::<$tt>() as u32,
            );
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
                    stack.remove_items(leave, stack.len() - tail_len - leave);
                }
                BreakDestination::LoopStart(start) => {
                    i = start;
                    let leave = block_returns[target_depth];
                    block_returns.truncate(target_depth + 1);
                    stack.truncate(leave);
                }
            }
            continue;
        }};
    }
    macro_rules! call {
        ($f:expr) => {{
            // TODO better signature check
            let params_len = $f.borrow().params_arity();
            let results_len = $f.borrow().results_arity();
            let top = stack.len();
            stack.resize_with_default(top + results_len);
            let params = if params_len > 0 {
                unsafe { slice::from_raw_parts(stack.item_ptr(top - params_len), params_len) }
            } else {
                &[]
            };
            let results = if results_len > 0 {
                unsafe { slice::from_raw_parts_mut(stack.item_mut_ptr(top), results_len) }
            } else {
                &mut []
            };
            let result = $f.borrow().call(params, results);
            match result {
                Ok(()) => {
                    stack.remove_items(top - params_len, params_len);
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
                block_returns.push(stack.len());
            }
            Operator::If { ty } => {
                let c = pop!(i32);
                block_returns.push(stack.len());
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
                let f = frame.context().get_function(*function_index);
                call!(f)
            }
            Operator::CallIndirect { index, table_index } => {
                let func_index = pop!(i32) as u32;
                let table = frame.context().get_table(*table_index);
                let ty = frame.context().get_type(*index);
                let f = match table.borrow().get_func(func_index) {
                    Ok(Some(f)) => f,
                    Ok(None) => trap!(TrapKind::Uninitialized),
                    Err(_) => trap!(TrapKind::UndefinedElement),
                };
                // TODO detailed signature check
                if f.borrow().params_arity() != ty.ty().params.len()
                    || f.borrow().results_arity() != ty.ty().returns.len()
                {
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
                    *stack.last_mut() = stack.pop();
                }
            }
            Operator::GetLocal { local_index } => stack.push(frame.get_local(*local_index).clone()),
            Operator::SetLocal { local_index } => {
                *frame.get_local_mut(*local_index) = stack.pop();
            }
            Operator::TeeLocal { local_index } => {
                *frame.get_local_mut(*local_index) = stack.last().clone();
            }
            Operator::GetGlobal { global_index } => {
                let g = frame.context().get_global(*global_index);
                stack.push(g.borrow().content().clone());
            }
            Operator::SetGlobal { global_index } => {
                let g = frame.context().get_global(*global_index);
                *g.borrow_mut().content_mut() = stack.pop();
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
            Operator::MemorySize {
                reserved: memory_index,
            } => {
                let current = frame.context().get_memory().borrow().current();
                push!(current as i32; i32)
            }
            Operator::MemoryGrow {
                reserved: memory_index,
            } => {
                let delta = pop!(i32) as u32;
                let current = frame.context().get_memory().borrow_mut().grow(delta);
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
            Operator::I32LtU => step!(|a:i32, b:i32| -> i32 if (a as u32) < b as u32 { 1 } else { 0 }),
            Operator::I32GtS => step!(|a:i32, b:i32| -> i32 if a > b { 1 } else { 0 }),
            Operator::I32GtU => step!(|a:i32, b:i32| -> i32 if (a as u32) > b as u32 { 1 } else { 0 }),
            Operator::I32LeS => step!(|a:i32, b:i32| -> i32 if a <= b { 1 } else { 0 }),
            Operator::I32LeU => step!(|a:i32, b:i32| -> i32 if (a as u32) <= b as u32 { 1 } else { 0 }),
            Operator::I32GeS => step!(|a:i32, b:i32| -> i32 if a >= b { 1 } else { 0 }),
            Operator::I32GeU => step!(|a:i32, b:i32| -> i32 if (a as u32) >= b as u32 { 1 } else { 0 }),
            Operator::I64Eqz => step!(|a:i64| -> i32 if a == 0 { 1 } else { 0 }),
            Operator::I64Eq => step!(|a:i64, b:i64| -> i32 if a == b { 1 } else { 0 }),
            Operator::I64Ne => step!(|a:i64, b:i64| -> i32 if a == b { 0 } else { 1 }),
            Operator::I64LtS => step!(|a:i64, b:i64| -> i32 if a < b { 1 } else { 0 }),
            Operator::I64LtU => step!(|a:i64, b:i64| -> i32 if (a as u64) < b as u64 { 1 } else { 0 }),
            Operator::I64GtS => step!(|a:i64, b:i64| -> i32 if a > b { 1 } else { 0 }),
            Operator::I64GtU => step!(|a:i64, b:i64| -> i32 if (a as u64) > b as u64 { 1 } else { 0 }),
            Operator::I64LeS => step!(|a:i64, b:i64| -> i32 if a <= b { 1 } else { 0 }),
            Operator::I64LeU => step!(|a:i64, b:i64| -> i32 if (a as u64) <= b as u64 { 1 } else { 0 }),
            Operator::I64GeS => step!(|a:i64, b:i64| -> i32 if a >= b { 1 } else { 0 }),
            Operator::I64GeU => step!(|a:i64, b:i64| -> i32 if (a as u64) >= b as u64 { 1 } else { 0 }),
            Operator::F32Eq => step!(|a:f32, b:f32| -> i32 floats::eq_f32(a, b)),
            Operator::F32Ne => step!(|a:f32, b:f32| -> i32 floats::ne_f32(a, b)),
            Operator::F32Lt => step!(|a:f32, b:f32| -> i32 floats::lt_f32(a, b)),
            Operator::F32Gt => step!(|a:f32, b:f32| -> i32 floats::gt_f32(a, b)),
            Operator::F32Le => step!(|a:f32, b:f32| -> i32 floats::le_f32(a, b)),
            Operator::F32Ge => step!(|a:f32, b:f32| -> i32 floats::ge_f32(a, b)),
            Operator::F64Eq => step!(|a:f64, b:f64| -> i32 floats::eq_f64(a, b)),
            Operator::F64Ne => step!(|a:f64, b:f64| -> i32 floats::ne_f64(a, b)),
            Operator::F64Lt => step!(|a:f64, b:f64| -> i32 floats::lt_f64(a, b)),
            Operator::F64Gt => step!(|a:f64, b:f64| -> i32 floats::gt_f64(a, b)),
            Operator::F64Le => step!(|a:f64, b:f64| -> i32 floats::le_f64(a, b)),
            Operator::F64Ge => step!(|a:f64, b:f64| -> i32 floats::ge_f64(a, b)),
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
            Operator::F32Abs => step!(|a:f32| -> f32 floats::abs_f32(a)),
            Operator::F32Neg => step!(|a:f32| -> f32 floats::neg_f32(a)),
            Operator::F32Ceil => step!(|a:f32| -> f32 floats::ceil_f32(a)),
            Operator::F32Floor => step!(|a:f32| -> f32 floats::floor_f32(a)),
            Operator::F32Trunc => step!(|a:f32| -> f32 floats::trunc_f32(a)),
            Operator::F32Nearest => step!(|a:f32| -> f32 floats::nearby_f32(a)),
            Operator::F32Sqrt => step!(|a:f32| -> f32 floats::sqrt_f32(a)),
            Operator::F32Add => step!(|a:f32, b:f32| -> f32 floats::add_f32(a, b)),
            Operator::F32Sub => step!(|a:f32, b:f32| -> f32 floats::sub_f32(a, b)),
            Operator::F32Mul => step!(|a:f32, b:f32| -> f32 floats::mul_f32(a, b)),
            Operator::F32Div => step!(|a:f32, b:f32| -> f32 floats::div_f32(a, b)),
            Operator::F32Min => step!(|a:f32, b:f32| -> f32 floats::min_f32(a, b)),
            Operator::F32Max => step!(|a:f32, b:f32| -> f32 floats::max_f32(a, b)),
            Operator::F32Copysign => step!(|a:f32, b:f32| -> f32 floats::copysign_f32(a, b)),
            Operator::F64Abs => step!(|a:f64| -> f64 floats::abs_f64(a)),
            Operator::F64Neg => step!(|a:f64| -> f64 floats::neg_f64(a)),
            Operator::F64Ceil => step!(|a:f64| -> f64 floats::ceil_f64(a)),
            Operator::F64Floor => step!(|a:f64| -> f64 floats::floor_f64(a)),
            Operator::F64Trunc => step!(|a:f64| -> f64 floats::trunc_f64(a)),
            Operator::F64Nearest => step!(|a:f64| -> f64 floats::nearby_f64(a)),
            Operator::F64Sqrt => step!(|a:f64| -> f64 floats::sqrt_f64(a)),
            Operator::F64Add => step!(|a:f64, b:f64| -> f64 floats::add_f64(a, b)),
            Operator::F64Sub => step!(|a:f64, b:f64| -> f64 floats::sub_f64(a, b)),
            Operator::F64Mul => step!(|a:f64, b:f64| -> f64 floats::mul_f64(a, b)),
            Operator::F64Div => step!(|a:f64, b:f64| -> f64 floats::div_f64(a, b)),
            Operator::F64Min => step!(|a:f64, b:f64| -> f64 floats::min_f64(a, b)),
            Operator::F64Max => step!(|a:f64, b:f64| -> f64 floats::max_f64(a, b)),
            Operator::F64Copysign => step!(|a:f64, b:f64| -> f64 floats::copysign_f64(a, b)),
            Operator::I32WrapI64 => step!(|a:i64| -> i32 a as i32),
            Operator::I32TruncSF32 => step!(|a:f32| -> i32 match floats::f32_trunc_i32(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I32TruncUF32 => step!(|a:f32| -> i32 match floats::f32_trunc_u32(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I32TruncSF64 => step!(|a:f64| -> i32 match floats::f64_trunc_i32(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I32TruncUF64 => step!(|a:f64| -> i32 match floats::f64_trunc_u32(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I64ExtendSI32 => step!(|a:i32| -> i64 (a as i64)),
            Operator::I64ExtendUI32 => step!(|a:i32| -> i64 (a as u32 as i64)),
            Operator::I64TruncSF32 => step!(|a:f32| -> i64 match floats::f32_trunc_i64(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I64TruncUF32 => step!(|a:f32| -> i64 match floats::f32_trunc_u64(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I64TruncSF64 => step!(|a:f64| -> i64 match floats::f64_trunc_i64(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::I64TruncUF64 => step!(|a:f64| -> i64 match floats::f64_trunc_u64(a) {
                Ok(c) => c,
                Err(kind) => trap!(kind),
            }),
            Operator::F32ConvertSI32 => step!(|a:i32| -> f32 floats::i32_to_f32(a)),
            Operator::F32ConvertUI32 => step!(|a:i32| -> f32 floats::u32_to_f32(a)),
            Operator::F32ConvertSI64 => step!(|a:i64| -> f32 floats::i64_to_f32(a)),
            Operator::F32ConvertUI64 => step!(|a:i64| -> f32 floats::u64_to_f32(a)),
            Operator::F32DemoteF64 => step!(|a:f64| -> f32 floats::f64_to_f32(a)),
            Operator::F64ConvertSI32 => step!(|a:i32| -> f64 floats::i32_to_f64(a)),
            Operator::F64ConvertUI32 => step!(|a:i32| -> f64 floats::u32_to_f64(a)),
            Operator::F64ConvertSI64 => step!(|a:i64| -> f64 floats::i64_to_f64(a)),
            Operator::F64ConvertUI64 => step!(|a:i64| -> f64 floats::u64_to_f64(a)),
            Operator::F64PromoteF32 => step!(|a:f32| -> f64 floats::f32_to_f64(a)),
            Operator::I32ReinterpretF32 => step!(|a:f32| -> i32 a as i32),
            Operator::I64ReinterpretF64 => step!(|a:f64| -> i64 a as i64),
            Operator::F32ReinterpretI32 => step!(|a:i32| -> f32 a as u32),
            Operator::F64ReinterpretI64 => step!(|a:i64| -> f64 a as u64),
            Operator::I32TruncSSatF32
            | Operator::I32TruncUSatF32
            | Operator::I32TruncSSatF64
            | Operator::I32TruncUSatF64
            | Operator::I64TruncSSatF32
            | Operator::I64TruncUSatF32
            | Operator::I64TruncSSatF64
            | Operator::I64TruncUSatF64
            | Operator::I32Extend16S
            | Operator::I32Extend8S
            | Operator::I64Extend32S
            | Operator::I64Extend16S
            | Operator::I64Extend8S => op_notimpl!(),
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
            | Operator::I32AtomicRmw16UAdd { .. }
            | Operator::I32AtomicRmw16USub { .. }
            | Operator::I32AtomicRmw16UAnd { .. }
            | Operator::I32AtomicRmw16UOr { .. }
            | Operator::I32AtomicRmw16UXor { .. }
            | Operator::I32AtomicRmw8UAdd { .. }
            | Operator::I32AtomicRmw8USub { .. }
            | Operator::I32AtomicRmw8UAnd { .. }
            | Operator::I32AtomicRmw8UOr { .. }
            | Operator::I32AtomicRmw8UXor { .. }
            | Operator::I64AtomicRmwAdd { .. }
            | Operator::I64AtomicRmwSub { .. }
            | Operator::I64AtomicRmwAnd { .. }
            | Operator::I64AtomicRmwOr { .. }
            | Operator::I64AtomicRmwXor { .. }
            | Operator::I64AtomicRmw32UAdd { .. }
            | Operator::I64AtomicRmw32USub { .. }
            | Operator::I64AtomicRmw32UAnd { .. }
            | Operator::I64AtomicRmw32UOr { .. }
            | Operator::I64AtomicRmw32UXor { .. }
            | Operator::I64AtomicRmw16UAdd { .. }
            | Operator::I64AtomicRmw16USub { .. }
            | Operator::I64AtomicRmw16UAnd { .. }
            | Operator::I64AtomicRmw16UOr { .. }
            | Operator::I64AtomicRmw16UXor { .. }
            | Operator::I64AtomicRmw8UAdd { .. }
            | Operator::I64AtomicRmw8USub { .. }
            | Operator::I64AtomicRmw8UAnd { .. }
            | Operator::I64AtomicRmw8UOr { .. }
            | Operator::I64AtomicRmw8UXor { .. }
            | Operator::I32AtomicRmwXchg { .. }
            | Operator::I32AtomicRmw16UXchg { .. }
            | Operator::I32AtomicRmw8UXchg { .. }
            | Operator::I32AtomicRmwCmpxchg { .. }
            | Operator::I32AtomicRmw16UCmpxchg { .. }
            | Operator::I32AtomicRmw8UCmpxchg { .. }
            | Operator::I64AtomicRmwXchg { .. }
            | Operator::I64AtomicRmw32UXchg { .. }
            | Operator::I64AtomicRmw16UXchg { .. }
            | Operator::I64AtomicRmw8UXchg { .. }
            | Operator::I64AtomicRmwCmpxchg { .. }
            | Operator::I64AtomicRmw32UCmpxchg { .. }
            | Operator::I64AtomicRmw16UCmpxchg { .. }
            | Operator::I64AtomicRmw8UCmpxchg { .. }
            | Operator::Wake { .. }
            | Operator::I32Wait { .. }
            | Operator::I64Wait { .. } => op_notimpl!(),
            Operator::Fence { ref flags } => op_notimpl!(),
            Operator::RefNull | Operator::RefIsNull => op_notimpl!(),
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
            | Operator::I8x16Mul
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
            | Operator::F32x4ConvertSI32x4
            | Operator::F32x4ConvertUI32x4
            | Operator::F64x2ConvertSI64x2
            | Operator::F64x2ConvertUI64x2
            | Operator::V128Not
            | Operator::I8x16Neg
            | Operator::I16x8Neg
            | Operator::I32x4Neg
            | Operator::I64x2Neg
            | Operator::I32x4TruncSF32x4Sat
            | Operator::I32x4TruncUF32x4Sat
            | Operator::I64x2TruncSF64x2Sat
            | Operator::I64x2TruncUF64x2Sat
            | Operator::V128Bitselect
            | Operator::I8x16AnyTrue
            | Operator::I8x16AllTrue
            | Operator::I16x8AnyTrue
            | Operator::I16x8AllTrue
            | Operator::I32x4AnyTrue
            | Operator::I32x4AllTrue
            | Operator::I64x2AnyTrue
            | Operator::I64x2AllTrue
            | Operator::I8x16Shl
            | Operator::I8x16ShrS
            | Operator::I8x16ShrU
            | Operator::I16x8Shl
            | Operator::I16x8ShrS
            | Operator::I16x8ShrU
            | Operator::I32x4Shl
            | Operator::I32x4ShrS
            | Operator::I32x4ShrU
            | Operator::I64x2Shl
            | Operator::I64x2ShrS
            | Operator::I64x2ShrU
            | Operator::V8x16Swizzle => op_notimpl!(),
            Operator::V8x16Shuffle { ref lanes } => op_notimpl!(),
            Operator::I8x16LoadSplat { .. }
            | Operator::I16x8LoadSplat { .. }
            | Operator::I32x4LoadSplat { .. }
            | Operator::I64x2LoadSplat { .. } => op_notimpl!(),
            Operator::MemoryCopy | Operator::MemoryFill => op_notimpl!(),
            Operator::MemoryInit { segment }
            | Operator::DataDrop { segment }
            | Operator::TableInit { segment }
            | Operator::ElemDrop { segment } => op_notimpl!(),
            Operator::TableCopy => op_notimpl!(),
            Operator::TableGet { table }
            | Operator::TableSet { table }
            | Operator::TableGrow { table }
            | Operator::TableSize { table } => op_notimpl!(),
        }
        i += 1;
    }
    returns.clone_from_slice(stack.tail(return_arity));
    Ok(())
}

pub(crate) fn eval_const<'a>(context: &'a (dyn EvalContext + 'a), source: &dyn EvalSource) -> Val {
    let mut vals = vec![Default::default()];
    let mut frame = Frame::new(context, 0);
    let result = eval(&mut frame, source, &mut vals);
    match result {
        Ok(()) => vals.into_iter().next().unwrap(),
        Err(_) => {
            panic!("trap duing eval_const");
        }
    }
}

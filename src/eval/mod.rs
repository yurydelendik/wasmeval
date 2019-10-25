use std::cell::RefCell;
use std::rc::Rc;

use crate::externals::{Func, Global, Memory};
use crate::instance::InstanceData;
use crate::values::{Trap, Val};

pub(crate) use bytecode::{BytecodeCache, EvalSource, Operator};
pub(crate) use context::{EvalContext, Frame, Local};

mod bytecode;
mod context;

pub(crate) fn eval<'a>(
    context: &'a mut EvalContext,
    source: &dyn EvalSource,
    locals: Vec<Local>,
) -> Result<Box<[Val]>, Rc<Trap>> {
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
            stack.pop().unwrap().$ty().unwrap()
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
                .borrow_mut()
                .content_ptr($memarg, offset);
            let val = unsafe { *(ptr as *const rust_ty!($ty)) };
            push!(val; $ty);
        }};
        ($memarg:expr; $ty:ident as $tt:ident) => {{
            let offset = pop!(i32) as u32;
            let ptr = context
                .get_memory()
                .borrow_mut()
                .content_ptr($memarg, offset);
            let val = unsafe { *(ptr as *const $tt) } as rust_ty!($ty);
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
                *(ptr as *mut rust_ty!($ty)) = val;
            }
        }};
        ($memarg:expr; $ty:ident as $tt:ident) => {{
            let val = pop!($ty) as $tt;
            let offset = pop!(i32) as u32;
            let ptr = context
                .get_memory()
                .borrow_mut()
                .content_ptr_mut($memarg, offset);
            unsafe {
                *(ptr as *mut $tt) = val;
            }
        }};
    }
    macro_rules! break_to {
        ($depth:expr) => {{
            i = bytecode.break_to(i, $depth);
            continue;
        }};
    }

    // TODO validate stack state
    // TODO handle traps

    loop {
        match &operators[i] {
            Operator::Unreachable => {
                return Err(Rc::new(Trap));
            }
            Operator::Nop => (),
            Operator::Block { .. } | Operator::Loop { .. } => (),
            Operator::If { ty } => {
                let c = pop!(i32);
                if c == 0 {
                    i = bytecode.skip_to_else(i);
                    continue;
                }
            }
            Operator::Else => {
                i = bytecode.skip_to_end(i);
                continue;
            }
            Operator::End => {
                if i + 1 >= bytecode.len() {
                    break;
                }
            }
            Operator::Br { relative_depth } => break_to!(*relative_depth),
            Operator::BrIf { relative_depth } => {
                let c = pop!(i32);
                if c != 0 {
                    break_to!(*relative_depth);
                }
            }
            Operator::BrTable { table } => unimplemented!("{:?}", operators[i]),
            Operator::Return => {
                break;
            }
            Operator::Call { function_index } => {
                let f = context.get_function(*function_index);
                let params = stack.split_off(stack.len() - f.borrow().params_arity());
                let result = f.borrow().call(&params);
                match result {
                    Ok(returns) => stack.extend_from_slice(&returns),
                    Err(trap) => {
                        return Err(trap);
                    }
                }
            }
            Operator::CallIndirect { index, table_index } => unimplemented!("{:?}", operators[i]),
            Operator::Drop => {
                stack.pop().unwrap();
            }
            Operator::Select => {
                let c = pop!(i32);
                if c != 0 {
                    stack.pop().unwrap();
                } else {
                    *stack.last_mut().unwrap() = stack.pop().unwrap();
                }
            }
            Operator::GetLocal { local_index } => stack.push(frame.get_local(*local_index).clone()),
            Operator::SetLocal { local_index } => {
                *frame.get_local_mut(*local_index) = stack.pop().unwrap();
            }
            Operator::TeeLocal { local_index } => {
                *frame.get_local_mut(*local_index) = stack.last().unwrap().clone();
            }
            Operator::GetGlobal { global_index } => {
                let g = context.get_global(*global_index);
                stack.push(g.borrow().content().clone());
            }
            Operator::SetGlobal { global_index } => {
                let g = context.get_global(*global_index);
                *g.borrow_mut().content_mut() = stack.pop().unwrap();
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
                let current = context.get_memory().borrow().current();
                push!(current as i32; i32)
            }
            Operator::MemoryGrow {
                reserved: memory_index,
            } => {
                let delta = pop!(i32) as u32;
                let current = context.get_memory().borrow_mut().grow(delta);
                push!(current as i32; i32)
            }
            Operator::I32Const { value } => push!(*value; i32),
            Operator::I64Const { value } => push!(*value; i64),
            Operator::F32Const { value } => push!(value.bits(); f32),
            Operator::F64Const { value } => push!(value.bits(); f64),
            Operator::I32Eqz => step!(|a:i32| -> i32 if a != 0 { 1 } else { 0 }),
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
            Operator::I64Eqz
            | Operator::I64Eq
            | Operator::I64Ne
            | Operator::I64LtS
            | Operator::I64LtU
            | Operator::I64GtS
            | Operator::I64GtU
            | Operator::I64LeS
            | Operator::I64LeU
            | Operator::I64GeS
            | Operator::I64GeU
            | Operator::F32Eq
            | Operator::F32Ne
            | Operator::F32Lt
            | Operator::F32Gt
            | Operator::F32Le
            | Operator::F32Ge
            | Operator::F64Eq
            | Operator::F64Ne
            | Operator::F64Lt
            | Operator::F64Gt
            | Operator::F64Le
            | Operator::F64Ge
            | Operator::I32Clz
            | Operator::I32Ctz
            | Operator::I32Popcnt => unimplemented!("{:?}", operators[i]),
            Operator::I32Add => step!(|a:i32, b:i32| -> i32 a + b),
            Operator::I32Sub => step!(|a:i32, b:i32| -> i32 a - b),
            Operator::I32Mul | Operator::I32DivS | Operator::I32DivU | Operator::I32RemS => {
                unimplemented!()
            }
            Operator::I32RemU => {
                step!(|a: i32, b: i32| -> i32 { ((a as u32) % (b as u32)) as i32 })
            }
            Operator::I32And => step!(|a:i32, b:i32| -> i32 a & b),
            Operator::I32Or => step!(|a:i32, b:i32| -> i32 a | b),
            Operator::I32Xor
            | Operator::I32Shl
            | Operator::I32ShrS
            | Operator::I32ShrU
            | Operator::I32Rotl
            | Operator::I32Rotr
            | Operator::I64Clz
            | Operator::I64Ctz
            | Operator::I64Popcnt
            | Operator::I64Add
            | Operator::I64Sub
            | Operator::I64Mul
            | Operator::I64DivS
            | Operator::I64DivU
            | Operator::I64RemS
            | Operator::I64RemU
            | Operator::I64And
            | Operator::I64Or
            | Operator::I64Xor
            | Operator::I64Shl
            | Operator::I64ShrS
            | Operator::I64ShrU
            | Operator::I64Rotl
            | Operator::I64Rotr
            | Operator::F32Abs
            | Operator::F32Neg
            | Operator::F32Ceil
            | Operator::F32Floor
            | Operator::F32Trunc
            | Operator::F32Nearest
            | Operator::F32Sqrt
            | Operator::F32Add
            | Operator::F32Sub
            | Operator::F32Mul
            | Operator::F32Div
            | Operator::F32Min
            | Operator::F32Max
            | Operator::F32Copysign
            | Operator::F64Abs
            | Operator::F64Neg
            | Operator::F64Ceil
            | Operator::F64Floor
            | Operator::F64Trunc
            | Operator::F64Nearest
            | Operator::F64Sqrt
            | Operator::F64Add
            | Operator::F64Sub
            | Operator::F64Mul
            | Operator::F64Div
            | Operator::F64Min
            | Operator::F64Max
            | Operator::F64Copysign
            | Operator::I32WrapI64
            | Operator::I32TruncSF32
            | Operator::I32TruncUF32
            | Operator::I32TruncSF64
            | Operator::I32TruncUF64
            | Operator::I64ExtendSI32
            | Operator::I64ExtendUI32
            | Operator::I64TruncSF32
            | Operator::I64TruncUF32
            | Operator::I64TruncSF64
            | Operator::I64TruncUF64
            | Operator::F32ConvertSI32
            | Operator::F32ConvertUI32
            | Operator::F32ConvertSI64
            | Operator::F32ConvertUI64
            | Operator::F32DemoteF64
            | Operator::F64ConvertSI32
            | Operator::F64ConvertUI32
            | Operator::F64ConvertSI64
            | Operator::F64ConvertUI64
            | Operator::F64PromoteF32
            | Operator::I32ReinterpretF32
            | Operator::I64ReinterpretF64
            | Operator::F32ReinterpretI32
            | Operator::F64ReinterpretI64
            | Operator::I32TruncSSatF32
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
            | Operator::I64Extend8S => unimplemented!("{:?}", operators[i]),
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
            | Operator::I64Wait { .. } => unimplemented!("{:?}", operators[i]),
            Operator::Fence { ref flags } => unimplemented!("{:?}", operators[i]),
            Operator::RefNull | Operator::RefIsNull => unimplemented!("{:?}", operators[i]),
            Operator::V128Load { .. } | Operator::V128Store { .. } => {
                unimplemented!("{:?}", operators[i])
            }
            Operator::V128Const { .. }
            | Operator::I8x16Splat
            | Operator::I16x8Splat
            | Operator::I32x4Splat
            | Operator::I64x2Splat
            | Operator::F32x4Splat
            | Operator::F64x2Splat => unimplemented!("{:?}", operators[i]),
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
            | Operator::F64x2ReplaceLane { lane } => unimplemented!("{:?}", operators[i]),
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
            | Operator::V8x16Swizzle => unimplemented!("{:?}", operators[i]),
            Operator::V8x16Shuffle { ref lanes } => unimplemented!("{:?}", operators[i]),
            Operator::I8x16LoadSplat { .. }
            | Operator::I16x8LoadSplat { .. }
            | Operator::I32x4LoadSplat { .. }
            | Operator::I64x2LoadSplat { .. } => unimplemented!("{:?}", operators[i]),
            Operator::MemoryCopy | Operator::MemoryFill => unimplemented!("{:?}", operators[i]),
            Operator::MemoryInit { segment }
            | Operator::DataDrop { segment }
            | Operator::TableInit { segment }
            | Operator::ElemDrop { segment } => unimplemented!("{:?}", operators[i]),
            Operator::TableCopy => unimplemented!("{:?}", operators[i]),
            Operator::TableGet { table }
            | Operator::TableSet { table }
            | Operator::TableGrow { table }
            | Operator::TableSize { table } => unimplemented!("{:?}", operators[i]),
        }
        i += 1;
    }
    Ok(stack.into_boxed_slice())
}

pub(crate) fn eval_const<'a>(context: &'a mut EvalContext, source: &dyn EvalSource) -> Val {
    let result = eval(context, source, vec![]);
    match result {
        Ok(val) => {
            debug_assert!(val.len() == 1);
            val[0].clone()
        }
        Err(_) => {
            panic!("trap duing eval_const");
        }
    }
}

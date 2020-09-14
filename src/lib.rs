pub use crate::eval::EvalContext;
pub use crate::externals::{
    External, Func, FuncType, Global, Memory, MemoryImmediate, Table, TableOutOfBounds,
};
pub use crate::instance::Instance;
pub use crate::memory::InstanceMemory;
pub use crate::module::Module;
pub use crate::values::{Trap, Val, ValType};

pub mod data {
    pub use wasmparser::{FuncType, Type};
}

use crate::eval::{eval as eval_internal, BytecodeCache, EvalSource};
use crate::values::get_default_value;

mod eval;
mod externals;
mod func;
mod global;
mod instance;
mod memory;
mod module;
mod table;
mod values;

#[cfg(test)]
mod tests;

pub enum TrapOrParserError {
    Trap(Trap),
    ParserError(wasmparser::BinaryReaderError),
}

pub fn eval(
    ctx: &dyn EvalContext,
    params: &[Val],
    returns: &mut [Val],
    code: &[u8],
    eval_stack_size: usize,
) -> Result<(), TrapOrParserError> {
    use wasmparser::FunctionBody;
    let code: &'static [u8] = unsafe { std::slice::from_raw_parts(code.as_ptr(), code.len()) };
    let body = FunctionBody::new(0, code);
    let mut stack = vec![Default::default(); eval_stack_size];

    let mut non_params = Vec::new();
    for i in body
        .get_locals_reader()
        .map_err(|e| TrapOrParserError::ParserError(e))?
    {
        let (count, ty) = i.map_err(|e| TrapOrParserError::ParserError(e))?;
        let val = get_default_value(ValType::from(ty));
        for _ in 0..count {
            non_params.push(val.clone());
        }
    }

    struct S(BytecodeCache);
    impl EvalSource for S {
        fn bytecode(&self) -> &BytecodeCache {
            &self.0
        }
    }

    let code_reader = body
        .get_operators_reader()
        .map_err(|e| TrapOrParserError::ParserError(e))?;
    let bytecode_cache = BytecodeCache::new(code_reader, ctx, returns.len());
    let source = S(bytecode_cache);

    let locals_len = params.len() + non_params.len();
    stack[..params.len()].clone_from_slice(params);
    stack[params.len()..locals_len].clone_from_slice(&non_params);
    eval_internal(ctx, &source, returns.len(), &mut stack, locals_len)
        .map_err(|e| TrapOrParserError::Trap(e))?;

    returns.clone_from_slice(&stack[0..returns.len()]);
    Ok(())
}

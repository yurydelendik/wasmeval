#[derive(Clone)]
pub enum Val {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
    Func(Option<std::sync::Arc<dyn crate::externals::Func>>),
}

impl std::fmt::Debug for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Val::I32(i) => write!(f, "I32({})", i),
            Val::I64(i) => write!(f, "I64({})", i),
            Val::F32(u) => write!(f, "F32({:08x})", u),
            Val::F64(u) => write!(f, "F64({:016x})", u),
            Val::Func(_) => write!(f, "Func"),
        }
    }
}

impl Val {
    pub fn ty(&self) -> ValType {
        match self {
            Val::I32(_) => ValType::I32,
            Val::I64(_) => ValType::I64,
            Val::F32(_) => ValType::F32,
            Val::F64(_) => ValType::F64,
            Val::Func(_) => ValType::FuncRef,
        }
    }

    pub fn i32(self) -> Option<i32> {
        if let Val::I32(val) = self {
            Some(val)
        } else {
            None
        }
    }

    pub fn i64(self) -> Option<i64> {
        if let Val::I64(val) = self {
            Some(val)
        } else {
            None
        }
    }

    pub fn f32(self) -> Option<u32> {
        if let Val::F32(val) = self {
            Some(val)
        } else {
            None
        }
    }

    pub fn f64(self) -> Option<u64> {
        if let Val::F64(val) = self {
            Some(val)
        } else {
            None
        }
    }
}

impl Default for Val {
    fn default() -> Self {
        Val::I32(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValType {
    I32,
    I64,
    F32,
    F64,
    FuncRef,
}

impl From<wasmparser::Type> for ValType {
    fn from(ty: wasmparser::Type) -> ValType {
        use wasmparser::Type::*;
        match ty {
            I32 => ValType::I32,
            I64 => ValType::I64,
            F32 => ValType::F32,
            F64 => ValType::F64,
            FuncRef => ValType::FuncRef,
            _ => unimplemented!("From<wasmparser::Type>"),
        }
    }
}

#[derive(Debug)]
pub enum TrapKind {
    Unreachable,
    OutOfBounds,
    SignatureMismatch,
    DivisionByZero,
    Overflow,
    InvalidIntegerConversion,
    IntegerOverflow,
    Uninitialized,
    UndefinedElement,
    User(String),
}

#[derive(Debug)]
pub struct Trap {
    kind: TrapKind,
    position: usize,
}

impl Trap {
    pub fn new(kind: TrapKind, position: usize) -> Self {
        Trap { kind, position }
    }
}

impl std::fmt::Display for Trap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self.kind {
                TrapKind::Unreachable => "unreachable".to_string(),
                TrapKind::OutOfBounds => "out of bounds memory access".to_string(),
                TrapKind::SignatureMismatch => "indirect call type mismatch".to_string(),
                TrapKind::DivisionByZero => "integer divide by zero".to_string(),
                TrapKind::Overflow => "integer overflow".to_string(),
                TrapKind::InvalidIntegerConversion => "invalid conversion to integer".to_string(),
                TrapKind::IntegerOverflow => "integer overflow".to_string(),
                TrapKind::Uninitialized => "uninitialized element".to_string(),
                TrapKind::UndefinedElement => "undefined element".to_string(),
                TrapKind::User(ref msg) => format!("user trap: {}", msg),
            }
        )
    }
}

impl std::error::Error for Trap {}

pub fn get_default_value(ty: ValType) -> Val {
    match ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
        ValType::FuncRef => Val::Func(None),
    }
}

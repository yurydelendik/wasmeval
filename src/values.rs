#[derive(Debug, Clone)]
pub enum Val {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
}

impl Val {
    pub fn ty(&self) -> ValType {
        match self {
            Val::I32(_) => ValType::I32,
            Val::I64(_) => ValType::I64,
            Val::F32(_) => ValType::F32,
            Val::F64(_) => ValType::F64,
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

pub enum ValType {
    I32,
    I64,
    F32,
    F64,
}

impl From<wasmparser::Type> for ValType {
    fn from(ty: wasmparser::Type) -> ValType {
        use wasmparser::Type::*;
        match ty {
            I32 => ValType::I32,
            I64 => ValType::I64,
            F32 => ValType::F32,
            F64 => ValType::F64,
            _ => unimplemented!("From<wasmparser::Type>"),
        }
    }
}

#[derive(Debug)]
pub struct Trap;

pub fn get_default_value(ty: ValType) -> Val {
    match ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
    }
}

#[derive(Debug, Clone)]
pub enum Val {
    I32(i32),
}

impl Val {
    pub fn ty(&self) -> ValType {
        match self {
            Val::I32(_) => ValType::I32,
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

pub struct Trap;

pub fn get_default_value(ty: ValType) -> Val {
    match ty {
        ValType::I32 => Val::I32(0),
        _ => unimplemented!("get_default_value"),
    }
}

use anyhow::{bail, Error};
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use wasmeval::{External, Func, FuncType, Instance, Module, Trap, Val};

struct Callback(Arc<FuncType>);
impl Callback {
    fn new() -> Self {
        Self(Arc::new(FuncType {
            params: Box::new([]),
            returns: Box::new([]),
        }))
    }
}
impl Func for Callback {
    fn ty(&self) -> &Arc<FuncType> {
        &self.0
    }
    fn call(&self, _stack: &mut [Val]) -> Result<(), Trap> {
        println!("Hello, world!");
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    let bin = fs::read(Path::new("examples/hello.wasm")).expect("file data");
    let module = Module::new(bin.into_boxed_slice())?;
    let instance = Instance::new(&module, &[External::Func(Rc::new(Callback::new()))])?;
    let hello = &instance.exports()[0];
    if let Ok(()) = hello.func().unwrap().call_wrapped(&[], &mut []) {
        return Ok(());
    }
    bail!("some error")
}

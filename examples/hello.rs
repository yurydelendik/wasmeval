use failure::{bail, Error};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use wasmeval::{External, Func, Instance, Module, Trap, Val};

struct Callback;
impl Func for Callback {
    fn params_arity(&self) -> usize {
        0
    }

    fn results_arity(&self) -> usize {
        0
    }

    fn call(&self, _params: &[Val]) -> Result<Box<[Val]>, Trap> {
        println!("Hello, world!");
        Ok(Box::new([]))
    }
}

fn main() -> Result<(), Error> {
    let bin = fs::read(Path::new("examples/hello.wasm")).expect("file data");
    let module = Module::new(bin.into_boxed_slice())?;
    let instance = Instance::new(&module, &[External::Func(Rc::new(RefCell::new(Callback)))])?;
    let hello = &instance.exports()[0];
    if let Ok(result) = hello.func().unwrap().borrow().call(&[]) {
        println!("{:?}", result);
        return Ok(());
    }
    bail!("some error")
}

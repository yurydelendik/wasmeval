use failure::{bail, Error};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use crate::externals::{External, Func};
use crate::instance::Instance;
use crate::module::Module;
use crate::values::{Trap, Val};

mod eval;
mod externals;
mod func;
mod instance;
mod module;
mod values;

struct Callback;
impl Func for Callback {
    fn params_arity(&self) -> usize {
        0
    }

    fn results_arity(&self) -> usize {
        0
    }

    fn call(&self, params: &[Val]) -> Result<Box<[Val]>, Rc<RefCell<Trap>>> {
        println!("Hello, world!");
        Ok(Box::new([]))
    }
}

fn main() -> Result<(), Error> {
    let bin = fs::read(Path::new("hello.wasm")).expect("file data");
    let module = Module::new(bin.into_boxed_slice())?;
    let instance = Instance::new(&module, &[External::Func(Rc::new(RefCell::new(Callback)))])?;
    let hello = &instance.exports()[0];
    if let Ok(result) = hello.func().unwrap().borrow().call(&[]) {
        println!("{:?}", result);
        return Ok(());
    }
    bail!("some error")
}

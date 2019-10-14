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
mod global;
mod instance;
mod memory;
mod module;
mod values;

fn _hello() -> Result<(), Error> {
    struct Callback;
    impl Func for Callback {
        fn params_arity(&self) -> usize {
            0
        }

        fn results_arity(&self) -> usize {
            0
        }

        fn call(&self, _params: &[Val]) -> Result<Box<[Val]>, Rc<RefCell<Trap>>> {
            println!("Hello, world!");
            Ok(Box::new([]))
        }
    }

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

fn gcd() -> Result<(), Error> {
    let bin = fs::read(Path::new("gcd.wasm")).expect("file data");
    let module = Module::new(bin.into_boxed_slice())?;
    let (gcd_index, _) = module
        .exports()
        .iter()
        .enumerate()
        .find(|(_i, e)| *e == "gcd")
        .expect("gcd export");
    let instance = Instance::new(&module, &[])?;
    let gcd = &instance.exports()[gcd_index];
    if let Ok(result) = gcd
        .func()
        .unwrap()
        .borrow()
        .call(&[Val::I32(6), Val::I32(27)])
    {
        println!("{:?}", result);
        return Ok(());
    }
    bail!("some error")
}

fn main() -> Result<(), Error> {
    gcd()
}

use anyhow::{bail, Error};
use std::fs;
use std::path::Path;

use wasmeval::{Instance, Module, Val};

fn main() -> Result<(), Error> {
    let bin = fs::read(Path::new("examples/gcd.wasm")).expect("file data");
    let module = Module::new(bin.into_boxed_slice())?;
    let (gcd_index, _) = module
        .exports()
        .iter()
        .enumerate()
        .find(|(_i, e)| *e == "gcd")
        .expect("gcd export");
    let instance = Instance::new(&module, &[])?;
    let gcd = &instance.exports()[gcd_index];
    let mut result = vec![Default::default()];
    if let Ok(()) = gcd
        .func()
        .unwrap()
        .call_wrapped(&[Val::I32(6), Val::I32(27)], &mut result)
    {
        println!("{:?}", result);
        return Ok(());
    }
    bail!("some error")
}

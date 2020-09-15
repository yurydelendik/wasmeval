use anyhow::{bail, Error};
use std::env;
use std::fs;
use std::path::Path;

use wasmeval::{Instance, Module};

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        bail!("USAGE: eval <wasmfile> [<fn_name>]");
    }
    let wasmfile = &args[1];
    let fn_name = args.get(2);

    let bin = fs::read(Path::new(wasmfile)).expect("file data");
    let module = Module::new(bin.into_boxed_slice())?;

    let fn_index = fn_name.map(|name| {
        let (index, _) = module
            .exports()
            .iter()
            .enumerate()
            .find(|(_i, (e, _ty))| e == name)
            .expect("export");
        index
    });
    let instance = Instance::new(&module, &[])?;
    if let Some(fn_index) = fn_index {
        let f = &instance.exports()[fn_index];
        let mut result = vec![Default::default()];
        if let Ok(()) = f.func().unwrap().call_wrapped(&[], &mut result) {
            eprintln!("{:?}", result);
            return Ok(());
        }
        bail!("some error")
    }
    Ok(())
}

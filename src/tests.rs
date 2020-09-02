use anyhow::Error;
use std::collections::HashMap;
use std::fs::{read, read_dir};
use std::rc::Rc;
use wast::{
    parser::{self, ParseBuffer},
    Expression, Id, NanPattern, WastDirective, Wat,
};

use crate::{External, Func, Instance, Module, Trap, Val};

fn parse_module(module: Vec<u8>) -> Result<Module, Error> {
    let bin = module.into_boxed_slice();
    let module = Module::new(bin)?;
    Ok(module)
}

fn instantiate_module<'b>(
    context: &'b Context,
    module: Vec<u8>,
) -> Result<(Instance, Module), Error> {
    let module = parse_module(module)?;
    let mut imports = Vec::new();
    for (module_name, field) in module.imports().into_iter() {
        let (instance, m) = context.find_instance_by_name(Some(&module_name));
        let (i, _) = m
            .exports()
            .into_iter()
            .enumerate()
            .find(|(_, e)| *e == field)
            .unwrap();
        imports.push(instance.exports()[i].clone());
    }
    let instance = Instance::new(&module, &imports)?;
    Ok((instance, module))
}

fn call_func(f: Rc<dyn Func>, args: Vec<Expression>) -> Result<Box<[Val]>, Trap> {
    use wast::Instruction;
    let args = args
        .into_iter()
        .map(|a| {
            if a.instrs.len() != 1 {
                unimplemented!();
            }
            match &a.instrs[0] {
                Instruction::I32Const(i) => Val::I32(*i),
                Instruction::I64Const(i) => Val::I64(*i),
                Instruction::F32Const(f) => Val::F32(f.bits),
                Instruction::F64Const(f) => Val::F64(f.bits),
                _ => unimplemented!(),
            }
        })
        .collect::<Vec<_>>();
    let mut out = vec![Default::default(); f.results_arity()];
    f.call(&args, &mut out)
        .map(move |()| out.into_boxed_slice())
}

fn preform_action<'a, 'b>(
    context: &'b Context,
    exec: wast::WastExecute<'a>,
) -> Result<Box<[Val]>, Trap> {
    use wast::{WastExecute::*, WastInvoke};
    let get_export = |module: Option<Id>, field: &str| -> Option<&External> {
        let (instance, module) = context.find_instance(module);
        module
            .exports()
            .iter()
            .enumerate()
            .find(|(_, e)| **e == field)
            .map(|(i, _)| &instance.exports()[i])
    };

    match exec {
        Invoke(WastInvoke {
            module, name, args, ..
        }) => {
            let export = get_export(module, name).unwrap();
            let f = export.func().unwrap().clone();
            call_func(f, args)
        }
        Get { module, global } => {
            let result = get_export(module, global).unwrap();
            match result {
                External::Global(g) => {
                    let context = vec![g.content().clone()];
                    Ok(context.into_boxed_slice())
                }
                _ => unimplemented!("Action::Get result"),
            }
        }
        Module(mut module) => {
            let binary = module.encode().expect("valid module");
            match instantiate_module(&context, binary) {
                Ok(_) => Ok(Box::new([])),
                Err(e) => Err(e.downcast::<Trap>().unwrap()),
            }
        }
    }
}

fn assert_value(value: &Val, expected: &wast::AssertExpression) -> bool {
    use wast::AssertExpression::*;
    match expected {
        I32(i) => {
            if let Val::I32(j) = value {
                j == i
            } else {
                false
            }
        }
        I64(i) => {
            if let Val::I64(j) = value {
                j == i
            } else {
                false
            }
        }
        F32(f) => {
            if let Val::F32(j) = value {
                match f {
                    NanPattern::Value(f) => *j == f.bits,
                    NanPattern::ArithmeticNan => {
                        (*j & 0x7f80_0000) == 0x7f80_0000 && (*j & 0x7f_ffff) >= 0x40_0000u32
                    }
                    NanPattern::CanonicalNan => {
                        (*j & 0x7f80_0000) == 0x7f80_0000 && (*j & 0x7f_ffff) == 0x40_0000u32
                    }
                }
            } else {
                false
            }
        }
        F64(f) => {
            if let Val::F64(j) = value {
                match f {
                    NanPattern::Value(f) => *j == f.bits,
                    NanPattern::ArithmeticNan => (*j & 0xf_ffff_ffff_ffff) != 0,
                    NanPattern::CanonicalNan => (*j & 0xf_ffff_ffff_ffff) == 0,
                }
            } else {
                false
            }
        }
        _ => unimplemented!(),
    }
}

struct Context {
    instances: Vec<(Instance, Module)>,
    aliases: HashMap<String, usize>,
    last: usize,
}
impl Context {
    pub fn new() -> Self {
        let instances = vec![create_spectest()];
        let aliases: HashMap<String, usize> =
            [("spectest".to_owned(), 0)].iter().cloned().collect();
        Context {
            instances,
            aliases,
            last: !0,
        }
    }
    pub fn add_instance(&mut self, instance: Instance, module: Module) {
        let module_name = module.name();
        let last = self.instances.len();
        self.instances.push((instance, module));
        self.last = last;
        if let Some(name) = module_name {
            self.aliases.insert(name, last);
        }
    }
    pub fn find_instance<'b>(&'b self, name: Option<Id>) -> &'b (Instance, Module) {
        self.find_instance_by_name(name.map(|id| id.name()))
    }
    pub fn find_instance_by_name<'b>(&'b self, name: Option<&str>) -> &'b (Instance, Module) {
        if name.is_none() {
            return &self.instances[self.last];
        }
        if let Some(index) = self.aliases.get(name.unwrap()) {
            &self.instances[*index]
        } else {
            panic!("unable to resolve {} module", name.unwrap());
        }
    }
    pub fn add_alias(&mut self, name: Option<Id>, as_name: String) {
        self.aliases.insert(
            as_name,
            match name {
                Some(ref name) => {
                    if let Some(index) = self.aliases.get(name.name()) {
                        *index
                    } else {
                        panic!("unable to resolve {} module", name.name(),);
                    }
                }
                None => self.last,
            },
        );
    }
}

fn create_spectest() -> (Instance, Module) {
    let spectest_wasm: Box<[u8]> = Box::new([
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x1a, 0x06, 0x60, 0x00, 0x00, 0x60,
        0x01, 0x7f, 0x00, 0x60, 0x02, 0x7f, 0x7d, 0x00, 0x60, 0x02, 0x7c, 0x7c, 0x00, 0x60, 0x01,
        0x7d, 0x00, 0x60, 0x01, 0x7c, 0x00, 0x03, 0x07, 0x06, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
        0x04, 0x05, 0x01, 0x70, 0x01, 0x0a, 0x14, 0x05, 0x04, 0x01, 0x01, 0x01, 0x02, 0x06, 0x1b,
        0x03, 0x7f, 0x00, 0x41, 0x9a, 0x05, 0x0b, 0x7d, 0x00, 0x43, 0x00, 0x80, 0x26, 0x44, 0x0b,
        0x7c, 0x00, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0xd0, 0x84, 0x40, 0x0b, 0x07, 0x85, 0x01,
        0x0b, 0x0a, 0x67, 0x6c, 0x6f, 0x62, 0x61, 0x6c, 0x5f, 0x69, 0x33, 0x32, 0x03, 0x00, 0x0a,
        0x67, 0x6c, 0x6f, 0x62, 0x61, 0x6c, 0x5f, 0x66, 0x33, 0x32, 0x03, 0x01, 0x0a, 0x67, 0x6c,
        0x6f, 0x62, 0x61, 0x6c, 0x5f, 0x66, 0x36, 0x34, 0x03, 0x02, 0x05, 0x74, 0x61, 0x62, 0x6c,
        0x65, 0x01, 0x00, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, 0x05, 0x70, 0x72,
        0x69, 0x6e, 0x74, 0x00, 0x00, 0x09, 0x70, 0x72, 0x69, 0x6e, 0x74, 0x5f, 0x69, 0x33, 0x32,
        0x00, 0x01, 0x0d, 0x70, 0x72, 0x69, 0x6e, 0x74, 0x5f, 0x69, 0x33, 0x32, 0x5f, 0x66, 0x33,
        0x32, 0x00, 0x02, 0x0d, 0x70, 0x72, 0x69, 0x6e, 0x74, 0x5f, 0x66, 0x36, 0x34, 0x5f, 0x66,
        0x36, 0x34, 0x00, 0x03, 0x09, 0x70, 0x72, 0x69, 0x6e, 0x74, 0x5f, 0x66, 0x33, 0x32, 0x00,
        0x04, 0x09, 0x70, 0x72, 0x69, 0x6e, 0x74, 0x5f, 0x66, 0x36, 0x34, 0x00, 0x05, 0x0a, 0x19,
        0x06, 0x03, 0x00, 0x01, 0x0b, 0x03, 0x00, 0x01, 0x0b, 0x03, 0x00, 0x01, 0x0b, 0x03, 0x00,
        0x01, 0x0b, 0x03, 0x00, 0x01, 0x0b, 0x03, 0x00, 0x01, 0x0b,
    ]);
    let module = Module::new(spectest_wasm).expect("spectest module");
    let instance = Instance::new(&module, &[]).expect("spectest instance");
    (instance, module)
}

fn run_wabt_scripts<F>(filename: &str, wast: &[u8], skip_test: F) -> anyhow::Result<()>
where
    F: Fn(&str, usize) -> bool,
{
    println!("Parsing {:?}", filename);
    // Check if we need to skip entire wast file test/parsing.
    if skip_test(filename, /* line = */ 0) {
        println!("{}: skipping", filename);
        return Ok(());
    }

    let wast = std::str::from_utf8(wast).unwrap();

    let adjust_wast = |mut err: wast::Error| {
        err.set_path(filename.as_ref());
        err.set_text(wast);
        err
    };

    let buf = wast::parser::ParseBuffer::new(wast).map_err(adjust_wast)?;
    let ast = wast::parser::parse::<wast::Wast>(&buf).map_err(adjust_wast)?;

    let mut context = Context::new();
    for directive in ast.directives {
        let sp = directive.span();
        let (line, _col) = sp.linecol_in(wast);
        if skip_test(filename, line) {
            println!("{}:{}: skipping", filename, line);
            continue;
        }
        println!("line {}", line);

        match directive {
            WastDirective::Module(mut module) => {
                let binary = module.encode()?;
                let (instance, module) = instantiate_module(&context, binary).expect("module");
                context.add_instance(instance, module);
            }
            WastDirective::QuoteModule { source, .. } => {
                let mut module = String::new();
                for src in source {
                    module.push_str(std::str::from_utf8(src)?);
                    module.push_str(" ");
                }
                let buf = ParseBuffer::new(&module)?;
                let mut wat = parser::parse::<Wat>(&buf)?;
                let binary = wat.module.encode()?;

                let (instance, module) = instantiate_module(&context, binary).expect("module");
                context.add_instance(instance, module);
            }
            WastDirective::AssertUnlinkable { .. } => {
                println!("{}:{}: skipping TODO!!!", filename, line);
                // if let Err(err) = validate_module(module, ()) {
                //     panic!("{}:{}: invalid module: {:?}", filename, line, err);
                // }
            }
            WastDirective::AssertInvalid { .. } | WastDirective::AssertMalformed { .. } => {
                println!("{}:{}: skipping TODO!!!", filename, line);
                // // TODO diffentiate between assert_invalid and assert_malformed
                // if let Ok(_) = validate_module(module, ()) {
                //     panic!(
                //         "{}:{}: invalid module was successfully parsed",
                //         filename, line
                //     );
                // }
            }
            WastDirective::Register { module, name, .. } => {
                context.add_alias(module, name.to_string());
            }
            WastDirective::Invoke(i) => {
                let _result = preform_action(&context, wast::WastExecute::Invoke(i));
            }
            WastDirective::AssertReturn { exec, results, .. } => {
                let result = preform_action(&context, exec);
                if let Err(trap) = result {
                    panic!("{}:{}: trap was found {:?}", filename, line, trap);
                }
                let expected = results;
                let returns = result.ok().unwrap();
                assert!(
                    returns.len() == expected.len(),
                    "{}:{}: returns.len {} != {}",
                    filename,
                    line,
                    returns.len(),
                    expected.len()
                );
                for i in 0..returns.len() {
                    assert!(
                        assert_value(&returns[i], &expected[i]),
                        "{}:{}: {:?} != {:?} @{}",
                        filename,
                        line,
                        returns[i],
                        expected[i],
                        i
                    );
                }
            }
            WastDirective::AssertTrap { exec, message, .. } => {
                let result = preform_action(&context, exec);
                if let Ok(_) = result {
                    panic!("{}:{}: trap is expected: {}", filename, line, message);
                }
                let trap = result.err().unwrap();
                let trap_message = trap.to_string();
                if !trap_message.contains(message) {
                    panic!(
                        "{}:{}: trap message {} ~= {}",
                        filename, line, message, trap_message
                    );
                }
            }
            WastDirective::AssertExhaustion { .. } => (),
        }
    }
    Ok(())
}

const SPEC_TESTS_PATH: &str = "testsuite";

#[test]
fn run_spec_tests() {
    for entry in read_dir(SPEC_TESTS_PATH).unwrap() {
        let dir = entry.unwrap();
        if !dir.file_type().unwrap().is_file()
            || dir.path().extension().map(|s| s.to_str().unwrap()) != Some("wast")
        {
            continue;
        }

        let data = read(&dir.path()).expect("wast data");
        run_wabt_scripts(
            dir.file_name().to_str().expect("name"),
            &data,
            //|_, _| false,
            |name, line| match (name, line) {
                ("linking.wast", 387)
                | ("linking.wast", 386)
                | ("float_misc.wast", _)
                | ("float_exprs.wast", _)
                | ("conversions.wast", _)
                | ("f32.wast", _)
                | ("f64.wast", _)
                | ("i64.wast", 291)
                | ("i64.wast", 292)
                | ("i64.wast", 293)
                | ("i64.wast", 294)
                | ("fac.wast", 106)
                // type mismatch
                | ("call_indirect.wast", 498)
                | ("call_indirect.wast", 508)
                | ("call_indirect.wast", 515)
                | ("call_indirect.wast", 522)
                // stack "heavy"
                | ("call.wast", 329)
                | ("call.wast", 330)
                | ("call.wast", 333)
                | ("call.wast", 334)
                | ("call_indirect.wast", 577)
                | ("call_indirect.wast", 578)
                | ("call_indirect.wast", 581)
                | ("call_indirect.wast", 582) => true,
                _ => false,
            },
        )
        .expect("success");
    }
}

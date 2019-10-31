use failure::Error;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{read, read_dir};
use std::rc::Rc;
use wabt::script::{Action, Command, CommandKind, ModuleBinary, ScriptParser, Value};
use wabt::Features;

use crate::{External, Func, Instance, Module, Trap, Val};

fn parse_module<'a>(module: ModuleBinary) -> Result<Module<'a>, Error> {
    let bin = module.into_vec().into_boxed_slice();
    let module = Module::new(bin)?;
    Ok(module)
}

fn instantiate_module<'a, 'b>(
    context: &'b Context<'a>,
    module: ModuleBinary,
) -> Result<(Instance<'a>, Module<'a>), Error> {
    let module = parse_module(module)?;
    let mut imports = Vec::new();
    for (module_name, field) in module.imports().into_iter() {
        let (instance, m) = context.find_instance(Some(module_name));
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

fn call_func<'a>(
    f: Rc<RefCell<dyn Func + 'a>>,
    args: Vec<Value<f32, f64>>,
) -> Result<Box<[Val]>, Trap> {
    let args = args
        .into_iter()
        .map(|a| match a {
            Value::I32(i) => Val::I32(i),
            Value::I64(i) => Val::I64(i),
            Value::F32(f) => Val::F32(unsafe { std::mem::transmute(f) }),
            Value::F64(f) => Val::F64(unsafe { std::mem::transmute(f) }),
            _ => unimplemented!(),
        })
        .collect::<Vec<_>>();
    f.borrow().call(&args)
}

fn preform_action<'a, 'b>(
    context: &'b Context<'a>,
    action: Action<f32, f64>,
) -> Result<Box<[Val]>, Trap> {
    let get_export = |module: Option<String>, field: String| -> Option<&External<'a>> {
        let (instance, module) = context.find_instance(module);
        module
            .exports()
            .iter()
            .enumerate()
            .find(|(_, e)| **e == field)
            .map(|(i, _)| &instance.exports()[i])
    };

    match action {
        Action::Invoke {
            module,
            field,
            args,
        } => {
            let export = get_export(module, field).unwrap();
            let f = export.func().unwrap().clone();
            call_func(f, args)
        }
        Action::Get { module, field } => {
            let result = get_export(module, field).unwrap();
            match result {
                External::Global(g) => {
                    let context = vec![g.borrow().content().clone()];
                    Ok(context.into_boxed_slice())
                }
                _ => unimplemented!("Action::Get result"),
            }
        }
    }
}

fn assert_value(value: &Val, expected: &Value<f32, f64>) -> bool {
    match expected {
        Value::I32(i) => {
            if let Val::I32(j) = value {
                j == i
            } else {
                false
            }
        }
        Value::I64(i) => {
            if let Val::I64(j) = value {
                j == i
            } else {
                false
            }
        }
        Value::F32(f) => {
            if let Val::F32(j) = value {
                *j == unsafe { std::mem::transmute::<_, u32>(*f) }
            } else {
                false
            }
        }
        Value::F64(f) => {
            if let Val::F64(j) = value {
                *j == unsafe { std::mem::transmute::<_, u64>(*f) }
            } else {
                false
            }
        }
        _ => unimplemented!(),
    }
}

struct Context<'a> {
    instances: Vec<(Instance<'a>, Module<'a>)>,
    aliases: HashMap<String, usize>,
    last: usize,
}
impl<'a> Context<'a> {
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
    pub fn add_instance(
        &mut self,
        instance: Instance<'a>,
        module: Module<'a>,
        name: Option<String>,
    ) {
        let last = self.instances.len();
        self.instances.push((instance, module));
        self.last = last;
        if let Some(name) = name {
            self.aliases.insert(name, last);
        }
    }
    pub fn find_instance<'b>(&'b self, name: Option<String>) -> &'b (Instance<'a>, Module<'a>)
    where
        'a: 'b,
    {
        if name.is_none() {
            return &self.instances[self.last];
        }
        let name = name.as_ref().unwrap();
        if let Some(index) = self.aliases.get(name) {
            &self.instances[*index]
        } else {
            panic!("unable to resolve {} module", name,);
        }
    }
    pub fn add_alias(&mut self, name: Option<String>, as_name: String) {
        self.aliases.insert(
            as_name,
            match name {
                Some(ref name) => {
                    if let Some(index) = self.aliases.get(name) {
                        *index
                    } else {
                        panic!("unable to resolve {} module", name,);
                    }
                }
                None => self.last,
            },
        );
    }
}

fn create_spectest<'a>() -> (Instance<'a>, Module<'a>) {
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

fn run_wabt_scripts<F>(filename: &str, wast: &[u8], features: Features, skip_test: F)
where
    F: Fn(&str, u64) -> bool,
{
    println!("Parsing {:?}", filename);
    // Check if we need to skip entire wast file test/parsing.
    if skip_test(filename, /* line = */ 0) {
        println!("{}: skipping", filename);
        return;
    }

    let mut parser: ScriptParser<f32, f64> =
        ScriptParser::from_source_and_name_with_features(wast, filename, features)
            .expect("script parser");

    let mut context = Context::new();
    while let Some(Command { kind, line }) = parser.next().expect("parser") {
        if skip_test(filename, line) {
            println!("{}:{}: skipping", filename, line);
            continue;
        }
        println!("line {}", line);

        match kind {
            CommandKind::Module { module, name } => {
                let (instance, module) = instantiate_module(&context, module).expect("module");
                context.add_instance(instance, module, name);
            }
            CommandKind::AssertUninstantiable { .. } | CommandKind::AssertUnlinkable { .. } => {
                println!("{}:{}: skipping TODO!!!", filename, line);
                // if let Err(err) = validate_module(module, ()) {
                //     panic!("{}:{}: invalid module: {:?}", filename, line, err);
                // }
            }
            CommandKind::AssertInvalid { .. } | CommandKind::AssertMalformed { .. } => {
                println!("{}:{}: skipping TODO!!!", filename, line);
                // // TODO diffentiate between assert_invalid and assert_malformed
                // if let Ok(_) = validate_module(module, ()) {
                //     panic!(
                //         "{}:{}: invalid module was successfully parsed",
                //         filename, line
                //     );
                // }
            }
            CommandKind::Register { name, as_name } => {
                context.add_alias(name, as_name);
            }
            CommandKind::PerformAction(action) => {
                let _result = preform_action(&context, action);
            }
            CommandKind::AssertReturn { action, expected } => {
                let result = preform_action(&context, action);
                if let Err(trap) = result {
                    panic!("{}:{}: trap was found {:?}", filename, line, trap);
                }
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
            CommandKind::AssertTrap { action, message } => {
                let result = preform_action(&context, action);
                if let Ok(_) = result {
                    panic!("{}:{}: trap is expected: {}", filename, line, message);
                }
                let trap = result.err().unwrap();
                let trap_message = trap.to_string();
                if !trap_message.contains(message.as_str()) {
                    panic!(
                        "{}:{}: trap message {} ~= {}",
                        filename, line, message, trap_message
                    );
                }
            }
            CommandKind::AssertExhaustion { .. }
            | CommandKind::AssertReturnCanonicalNan { .. }
            | CommandKind::AssertReturnArithmeticNan { .. } => (),
        }
    }
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

        let wabt_features = Features::new();

        let data = read(&dir.path()).expect("wast data");
        run_wabt_scripts(
            dir.file_name().to_str().expect("name"),
            &data,
            wabt_features,
            //|_, _| false,
            |name, line| match (name, line) {
                ("linking.wast", 387)
                | ("linking.wast", 388)
                // -0.0
                | ("f32.wast", 1621)
                | ("f64.wast", 1621)
                | ("f32.wast", 2020)
                | ("f64.wast", 2020)
                // integer overflow
                | ("conversions.wast", 70)
                | ("conversions.wast", 92)
                | ("conversions.wast", 166)
                | ("conversions.wast", 186)
                | ("conversions.wast", 211)
                | ("conversions.wast", 235)
                // type mismatch
                | ("call_indirect.wast", 470)
                | ("call_indirect.wast", 480)
                | ("call_indirect.wast", 487)
                | ("call_indirect.wast", 494)
                // stack "heavy"
                | ("call.wast", 265)
                | ("call.wast", 266)
                | ("call.wast", 269)
                | ("call.wast", 270)
                | ("call_indirect.wast", 549)
                | ("call_indirect.wast", 550)
                | ("call_indirect.wast", 553)
                | ("call_indirect.wast", 554) => true,
                _ => false,
            },
        );
    }
}

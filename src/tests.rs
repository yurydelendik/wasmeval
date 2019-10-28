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
    _context: &'b Context<'a>,
    module: ModuleBinary,
) -> Result<(Instance<'a>, Module<'a>), Error> {
    let module = parse_module(module)?;
    // TODO dependencies
    let instance = Instance::new(&module, &[])?;
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
    pub instances: HashMap<String, (Instance<'a>, Module<'a>)>,
    pub last_name: String,
}
impl<'a> Context<'a> {
    pub fn new() -> Self {
        Context {
            instances: HashMap::new(),
            last_name: String::from(""),
        }
    }
    pub fn add_instance(
        &mut self,
        instance: Instance<'a>,
        module: Module<'a>,
        name: Option<String>,
    ) {
        let name = name.unwrap_or(String::from(""));
        self.instances.insert(name.clone(), (instance, module));
        self.last_name = name;
    }
    pub fn find_instance<'b>(&'b self, name: Option<String>) -> &'b (Instance<'a>, Module<'a>)
    where
        'a: 'b,
    {
        let name = name.unwrap_or_else(|| self.last_name.clone());
        self.instances.get(&name).unwrap()
    }
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
        println!("line {}", line);
        if skip_test(filename, line) {
            println!("{}:{}: skipping", filename, line);
            continue;
        }

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
            CommandKind::Register { .. } => {
                // TODO register for linking
            }
            CommandKind::PerformAction(action) => {
                let _result = preform_action(&context, action);
            }
            CommandKind::AssertReturn { action, expected } => {
                let result = preform_action(&context, action);
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
            CommandKind::AssertTrap { .. }
            | CommandKind::AssertExhaustion { .. }
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
                ("memory.wast", _)
                | ("imports.wast", _)
                | ("binary.wast", _)
                | ("linking.wast", _)
                | ("globals.wast", _)
                | ("comments.wast", _)
                | ("binary-leb128.wast", _)
                | ("elem.wast", _)
                | ("data.wast", _)
                | ("custom.wast", _)
                | ("start.wast", _)
                | ("names.wast", _)
                | ("func_ptrs.wast", _)
                | ("unwind.wast", _)
                // | ("table_get.wast", _)
                // | ("table_set.wast", _)
                // | ("table_size.wast", _)
                // | ("table_fill.wast", _)
                // | ("table_grow.wast", _)
                | ("call.wast", 269) // stack
                // br_table
                | ("call.wast", 284) 
                | ("call.wast", 285)
                | ("local_get.wast", 125)
                 // call_indirect
                | ("call.wast", 287)
                | ("call.wast", 288)
                | ("exports.wast", _) => true,
                _ => false,
            },
        );
    }
}

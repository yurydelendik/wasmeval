use anyhow::{format_err, Error};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use wasmeval::{eval, EvalContext, Func, FuncType, Global, Memory, MemoryImmediate, Table, Val};

fn read_gcd(data: &[u8]) -> Result<&[u8], Error> {
    use wasmparser::{ExternalKind, ImportSectionEntryType, Parser, Payload};

    let gcd_body = {
        let mut import_count = 0;
        let mut gcd_body = None;
        let mut gcd_body_index = None;
        let mut index = 0;
        for p in Parser::new(0).parse_all(data) {
            match p? {
                Payload::ImportSection(reader) => {
                    for i in reader.into_iter() {
                        if let ImportSectionEntryType::Function(_) = i?.ty {
                            import_count += 1;
                        }
                    }
                }
                Payload::CodeSectionEntry(body) => {
                    if Some(index + import_count) == gcd_body_index {
                        gcd_body = Some(body);
                        break;
                    }
                    index += 1;
                }
                Payload::ExportSection(reader) => {
                    for e in reader.into_iter() {
                        if let wasmparser::Export {
                            field: "gcd",
                            kind: ExternalKind::Function,
                            index,
                        } = e?
                        {
                            gcd_body_index = Some(index);
                            break;
                        }
                    }
                }
                _ => (),
            }
        }
        gcd_body.ok_or_else(|| format_err!("gcd body not found"))?
    };

    Ok(gcd_body.range().slice(data))
}

struct Ctx {
    global: Rc<dyn Global>,
    memory: Rc<dyn Memory>,
}

impl Ctx {
    pub fn new() -> Self {
        struct G(RefCell<Val>);
        impl Global for G {
            fn content(&self) -> Val {
                self.0.borrow().clone()
            }
            fn set_content(&self, val: &Val) {
                *self.0.borrow_mut() = val.clone();
            }
        }
        #[inline]
        fn combine_offsets(memarg: &MemoryImmediate, offset: u32) -> usize {
            memarg.offset as usize + offset as usize
        }
        struct M(RefCell<Vec<u8>>);
        impl Memory for M {
            fn current(&self) -> u32 {
                1
            }
            fn grow(&self, _delta: u32) -> u32 {
                panic!("M grow");
            }
            fn content_ptr(&self, memarg: &MemoryImmediate, offset: u32, size: u32) -> *const u8 {
                let offset = combine_offsets(memarg, offset);
                if offset + size as usize > self.0.borrow().len() {
                    return std::ptr::null();
                }
                &self.0.borrow()[offset]
            }
            fn content_ptr_mut(&self, memarg: &MemoryImmediate, offset: u32, size: u32) -> *mut u8 {
                let offset = combine_offsets(memarg, offset);
                if offset + size as usize > self.0.borrow().len() {
                    return std::ptr::null_mut();
                }
                &mut self.0.borrow_mut()[offset]
            }
            fn clone_from_slice(&self, offset: u32, chunk: &[u8]) {
                let offset = offset as usize;
                self.0.borrow_mut()[offset..(offset + chunk.len())].clone_from_slice(chunk);
            }
        }
        Self {
            global: Rc::new(G(RefCell::new(Val::I32(65336)))),
            memory: Rc::new(M(RefCell::new(vec![0u8; 65336]))),
        }
    }
}

impl EvalContext for Ctx {
    fn get_function(&self, index: u32) -> Rc<dyn Func> {
        panic!("func {}", index);
    }
    fn get_global(&self, index: u32) -> Rc<dyn Global> {
        match index {
            0 => self.global.clone(),
            _ => {
                panic!("global {}", index);
            }
        }
    }
    fn get_memory(&self) -> Rc<dyn Memory> {
        self.memory.clone()
    }
    fn get_table(&self, index: u32) -> Rc<dyn Table> {
        panic!("table {}", index);
    }
    fn get_type(&self, index: u32) -> Rc<dyn FuncType> {
        panic!("type {}", index);
    }
}

fn main() -> Result<(), Error> {
    let bin = fs::read(Path::new("examples/gcd.wasm"))?;
    let gcd_data = read_gcd(&bin)?;
    let params = vec![Val::I32(24), Val::I32(18)];
    let mut returns = vec![Val::I32(0)];
    let ctx = Ctx::new();
    eval(&ctx, &params, &mut returns, gcd_data).map_err(|_| format_err!("gcd failed"))?;
    println!("{:?}", returns);
    Ok(())
}

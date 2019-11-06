use failure::{format_err, Error};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use wasmeval::{eval, EvalContext, Func, FuncType, Global, Memory, MemoryImmediate, Table, Val};

fn read_gcd(data: &[u8]) -> Result<&[u8], Error> {
    use wasmparser::{ExternalKind, ImportSectionEntryType, ModuleReader, SectionContent};

    let mut reader = ModuleReader::new(data)?;

    let gcd_body = {
        let mut import_count = 0;
        let mut gcd_body = None;
        let mut gcd_body_index = None;
        while !reader.eof() {
            match reader.read()?.content()? {
                SectionContent::Import(reader) => {
                    for i in reader.into_iter() {
                        if let ImportSectionEntryType::Function(_) = i?.ty {
                            import_count += 1;
                        }
                    }
                }
                SectionContent::Code(reader) => {
                    let index =
                        gcd_body_index.ok_or_else(|| format_err!("gcd export not found"))?;
                    gcd_body = reader
                        .into_iter()
                        .skip(index as usize - import_count)
                        .next();
                    break;
                }
                SectionContent::Export(reader) => {
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

    Ok(gcd_body?.range().slice(data))
}

struct Ctx {
    global: Rc<RefCell<dyn Global>>,
    memory: Rc<RefCell<dyn Memory>>,
}

impl Ctx {
    pub fn new() -> Self {
        struct G(Val);
        impl Global for G {
            fn content(&self) -> &Val {
                &self.0
            }
            fn content_mut(&mut self) -> &mut Val {
                &mut self.0
            }
        }
        #[inline]
        fn combine_offsets(memarg: &MemoryImmediate, offset: u32) -> usize {
            memarg.offset as usize + offset as usize
        }
        struct M(Vec<u8>);
        impl Memory for M {
            fn current(&self) -> u32 {
                1
            }
            fn grow(&mut self, _delta: u32) -> u32 {
                panic!("M grow");
            }
            fn content_ptr(&self, memarg: &MemoryImmediate, offset: u32, size: u32) -> *const u8 {
                let offset = combine_offsets(memarg, offset);
                if offset + size as usize > self.0.len() {
                    return std::ptr::null();
                }
                &self.0[offset]
            }
            fn content_ptr_mut(
                &mut self,
                memarg: &MemoryImmediate,
                offset: u32,
                size: u32,
            ) -> *mut u8 {
                let offset = combine_offsets(memarg, offset);
                if offset + size as usize > self.0.len() {
                    return std::ptr::null_mut();
                }
                &mut self.0[offset]
            }
            fn clone_from_slice(&mut self, offset: u32, chunk: &[u8]) {
                let offset = offset as usize;
                self.0[offset..(offset + chunk.len())].clone_from_slice(chunk);
            }
        }
        Self {
            global: Rc::new(RefCell::new(G(Val::I32(65336)))),
            memory: Rc::new(RefCell::new(M(vec![0u8; 65336]))),
        }
    }
}

impl EvalContext for Ctx {
    fn get_function(&self, index: u32) -> Rc<RefCell<dyn Func>> {
        panic!("func {}", index);
    }
    fn get_global(&self, index: u32) -> Rc<RefCell<dyn Global>> {
        match index {
            0 => self.global.clone(),
            _ => {
                panic!("global {}", index);
            }
        }
    }
    fn get_memory(&self) -> Rc<RefCell<dyn Memory>> {
        self.memory.clone()
    }
    fn get_table(&self, index: u32) -> Rc<RefCell<dyn Table>> {
        panic!("table {}", index);
    }
    fn get_type(&self, index: u32) -> Rc<RefCell<dyn FuncType>> {
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

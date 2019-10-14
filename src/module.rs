use failure::Error;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use wasmparser::{
    Export, FuncType, FunctionBody, Global, Import, MemoryType, ModuleReader, SectionCode,
};

pub(crate) struct ModuleData<'a> {
    pub buf: Pin<Box<[u8]>>,
    pub types: Box<[FuncType]>,
    pub imports: Box<[Import<'a>]>,
    pub exports: Box<[Export<'a>]>,
    pub memories: Box<[MemoryType]>,
    pub globals: Box<[Global<'a>]>,
    pub func_types: Box<[u32]>,
    pub func_bodies: Box<[FunctionBody<'a>]>,
}

pub struct Module<'a> {
    data: Rc<RefCell<ModuleData<'a>>>,
}

fn read_module_data<'a>(buf: Pin<Box<[u8]>>) -> Result<ModuleData<'a>, Error> {
    let mut reader = {
        let buf = unsafe { &std::slice::from_raw_parts(buf.as_ptr(), buf.len()) };
        ModuleReader::new(buf)?
    };
    let mut types = None;
    let mut imports = None;
    let mut exports = None;
    let mut memories = None;
    let mut globals = None;
    let mut func_types = None;
    let mut func_bodies = None;
    while !reader.eof() {
        let section = reader.read()?;
        match section.code {
            SectionCode::Type => {
                types = Some(
                    section
                        .get_type_section_reader()?
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            SectionCode::Import => {
                imports = Some(
                    section
                        .get_import_section_reader()?
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            SectionCode::Export => {
                exports = Some(
                    section
                        .get_export_section_reader()?
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            SectionCode::Memory => {
                memories = Some(
                    section
                        .get_memory_section_reader()?
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            SectionCode::Global => {
                globals = Some(
                    section
                        .get_global_section_reader()?
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            SectionCode::Function => {
                func_types = Some(
                    section
                        .get_function_section_reader()?
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            SectionCode::Code => {
                func_bodies = Some(
                    section
                        .get_code_section_reader()?
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            _ => (),
        }
    }
    let types = types.expect("types").into_boxed_slice();
    let imports = imports.unwrap_or_else(|| vec![]).into_boxed_slice();
    let exports = exports.unwrap_or_else(|| vec![]).into_boxed_slice();
    let memories = memories.unwrap_or_else(|| vec![]).into_boxed_slice();
    let globals = globals.unwrap_or_else(|| vec![]).into_boxed_slice();
    let func_types = func_types.unwrap_or_else(|| vec![]).into_boxed_slice();
    let func_bodies = func_bodies.unwrap_or_else(|| vec![]).into_boxed_slice();
    Ok(ModuleData {
        buf,
        types,
        imports,
        exports,
        memories,
        globals,
        func_types,
        func_bodies,
    })
}

impl<'a> Module<'a> {
    pub fn new(buf: Box<[u8]>) -> Result<Module<'a>, Error> {
        Ok(Module {
            data: Rc::new(RefCell::new(read_module_data(Pin::new(buf))?)),
        })
    }

    pub(crate) fn data(&self) -> &Rc<RefCell<ModuleData<'a>>> {
        &self.data
    }

    pub fn imports(&self) -> Vec<(String, String)> {
        vec![]
    }

    pub fn exports(&self) -> Vec<String> {
        self.data
            .borrow()
            .exports
            .iter()
            .map(|e| e.field.to_string())
            .collect::<Vec<_>>()
    }
}

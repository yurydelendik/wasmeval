use anyhow::{bail, Error};
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use wasmparser::{
    Data, Element, Export, FuncType, FunctionBody, Global, Import, MemoryType, Parser, Payload,
    TableType, TypeDef,
};

pub(crate) struct ModuleData {
    pub buf: Pin<Box<[u8]>>,
    pub types: Box<[FuncType]>,
    pub imports: Box<[Import<'static>]>,
    pub exports: Box<[Export<'static>]>,
    pub memories: Box<[MemoryType]>,
    pub data: Box<[Data<'static>]>,
    pub tables: Box<[TableType]>,
    pub elements: Box<[Element<'static>]>,
    pub globals: Box<[Global<'static>]>,
    pub func_types: Box<[u32]>,
    pub func_bodies: Box<[FunctionBody<'static>]>,
    pub start_func: Option<u32>,
}

pub struct Module {
    data: Rc<RefCell<ModuleData>>,
}

fn read_module_data(buf: Pin<Box<[u8]>>) -> Result<ModuleData, Error> {
    let it = {
        let buf = unsafe { &std::slice::from_raw_parts(buf.as_ptr(), buf.len()) };
        Parser::new(0).parse_all(buf)
    };
    let mut types = None;
    let mut imports = None;
    let mut exports = None;
    let mut memories = None;
    let mut data = None;
    let mut tables = None;
    let mut elements = None;
    let mut globals = None;
    let mut func_types = None;
    let mut func_bodies = vec![];
    let mut start_func = None;
    for r in it {
        let payload = r?;
        match payload {
            Payload::TypeSection(section) => {
                types = Some(
                    section
                        .into_iter()
                        .map(|ty| match ty {
                            Ok(TypeDef::Func(f)) => Ok(f),
                            Err(e) => bail!("type error: {:?}", e),
                            _ => {
                                bail!("unsupported typedef");
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            Payload::ImportSection(section) => {
                imports = Some(section.into_iter().collect::<Result<Vec<_>, _>>()?);
            }
            Payload::ExportSection(section) => {
                exports = Some(section.into_iter().collect::<Result<Vec<_>, _>>()?);
            }
            Payload::MemorySection(section) => {
                memories = Some(section.into_iter().collect::<Result<Vec<_>, _>>()?);
            }
            Payload::TableSection(section) => {
                tables = Some(section.into_iter().collect::<Result<Vec<_>, _>>()?);
            }
            Payload::GlobalSection(section) => {
                globals = Some(section.into_iter().collect::<Result<Vec<_>, _>>()?);
            }
            Payload::FunctionSection(section) => {
                func_types = Some(section.into_iter().collect::<Result<Vec<_>, _>>()?);
            }
            Payload::CodeSectionEntry(body) => {
                func_bodies.push(body);
            }
            Payload::DataSection(section) => {
                data = Some(section.into_iter().collect::<Result<Vec<_>, _>>()?);
            }
            Payload::ElementSection(section) => {
                elements = Some(section.into_iter().collect::<Result<Vec<_>, _>>()?);
            }
            Payload::StartSection { func, .. } => {
                start_func = Some(func);
            }
            _ => (),
        }
    }
    let types = types.unwrap_or_else(|| vec![]).into_boxed_slice();
    let imports = imports.unwrap_or_else(|| vec![]).into_boxed_slice();
    let exports = exports.unwrap_or_else(|| vec![]).into_boxed_slice();
    let memories = memories.unwrap_or_else(|| vec![]).into_boxed_slice();
    let data = data.unwrap_or_else(|| vec![]).into_boxed_slice();
    let tables = tables.unwrap_or_else(|| vec![]).into_boxed_slice();
    let elements = elements.unwrap_or_else(|| vec![]).into_boxed_slice();
    let globals = globals.unwrap_or_else(|| vec![]).into_boxed_slice();
    let func_types = func_types.unwrap_or_else(|| vec![]).into_boxed_slice();
    let func_bodies = func_bodies.into_boxed_slice();
    Ok(ModuleData {
        buf,
        types,
        imports,
        exports,
        memories,
        data,
        tables,
        elements,
        globals,
        func_types,
        func_bodies,
        start_func,
    })
}

impl Module {
    pub fn new(buf: Box<[u8]>) -> Result<Module, Error> {
        Ok(Module {
            data: Rc::new(RefCell::new(read_module_data(Pin::new(buf))?)),
        })
    }

    pub(crate) fn data(&self) -> &Rc<RefCell<ModuleData>> {
        &self.data
    }

    pub fn imports(&self) -> Vec<(String, String)> {
        self.data
            .borrow()
            .imports
            .iter()
            .map(|e| {
                (
                    e.module.to_string(),
                    e.field.expect("TODO module").to_string(),
                )
            })
            .collect::<Vec<_>>()
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

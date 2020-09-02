use anyhow::{bail, Error};
use std::pin::Pin;
use std::rc::Rc;
use wasmparser::{
    Data, Element, Export, FuncType, FunctionBody, Global, Import, ImportSectionEntryType,
    MemoryType, Name, NameSectionReader, Parser, Payload, TableType, TypeDef,
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
    pub module_name: Option<String>,
}

pub struct Module {
    data: Rc<ModuleData>,
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
    let mut module_name = None;
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
                            _ => bail!("unsupported typedef"),
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            Payload::ImportSection(section) => {
                imports = Some(
                    section
                        .into_iter()
                        .map(|i| match i {
                            Ok(
                                i
                                @
                                Import {
                                    ty: ImportSectionEntryType::Function(_),
                                    ..
                                },
                            )
                            | Ok(
                                i
                                @
                                Import {
                                    ty: ImportSectionEntryType::Memory(_),
                                    ..
                                },
                            )
                            | Ok(
                                i
                                @
                                Import {
                                    ty: ImportSectionEntryType::Table(_),
                                    ..
                                },
                            )
                            | Ok(
                                i
                                @
                                Import {
                                    ty: ImportSectionEntryType::Global(_),
                                    ..
                                },
                            ) => Ok(i),
                            Err(e) => bail!("import error: {:?}", e),
                            _ => bail!("unsupported import"),
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                );
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
            Payload::CustomSection {
                name,
                data,
                data_offset,
            } => {
                if name == "name" {
                    let mut iter = NameSectionReader::new(data, data_offset)?;
                    while !iter.eof() {
                        match iter.read()? {
                            Name::Module(name) => {
                                module_name = Some(name.get_name()?.to_string());
                            }
                            _ => (),
                        }
                    }
                }
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
        module_name,
    })
}

impl Module {
    pub fn new(buf: Box<[u8]>) -> Result<Module, Error> {
        Ok(Module {
            data: Rc::new(read_module_data(Pin::new(buf))?),
        })
    }

    pub(crate) fn data(&self) -> &Rc<ModuleData> {
        &self.data
    }

    pub fn imports(&self) -> Vec<(String, String)> {
        self.data
            .imports
            .iter()
            .map(|e| (e.module.to_string(), e.field.unwrap().to_string()))
            .collect::<Vec<_>>()
    }

    pub fn exports(&self) -> Vec<String> {
        self.data
            .exports
            .iter()
            .map(|e| e.field.to_string())
            .collect::<Vec<_>>()
    }

    pub fn name(&self) -> Option<String> {
        self.data.module_name.clone()
    }
}

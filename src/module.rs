use anyhow::{bail, Error};
use std::pin::Pin;
use std::sync::Arc;
use wasmparser::{
    Data, Element, Export, FunctionBody, Global, Import, ImportSectionEntryType, MemoryType, Name,
    NameSectionReader, Parser, Payload, TableType, TypeDef,
};

use crate::externals::{self, ExternType, FuncType};

pub(crate) struct ModuleData {
    pub buf: Pin<Box<[u8]>>,
    pub types: Box<[Arc<FuncType>]>,
    pub imports: Box<[Import<'static>]>,
    pub exports: Box<[Export<'static>]>,
    pub imported_memories_map: Box<[usize]>,
    pub memories: Box<[MemoryType]>,
    pub data: Box<[Data<'static>]>,
    pub imported_tables_map: Box<[usize]>,
    pub tables: Box<[TableType]>,
    pub elements: Box<[Element<'static>]>,
    pub imported_globals_map: Box<[usize]>,
    pub globals: Box<[Global<'static>]>,
    pub imported_func_map: Box<[usize]>,
    pub func_types: Box<[u32]>,
    pub func_bodies: Box<[FunctionBody<'static>]>,
    pub start_func: Option<u32>,
    pub module_name: Option<String>,
}

pub struct Module {
    data: Arc<ModuleData>,
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
    let mut imported_func_map = vec![];
    let mut imported_memories_map = vec![];
    let mut imported_tables_map = vec![];
    let mut imported_globals_map = vec![];
    for r in it {
        let payload = r?;
        match payload {
            Payload::TypeSection(section) => {
                types = Some(
                    section
                        .into_iter()
                        .map(|ty| match ty {
                            Ok(TypeDef::Func(f)) => Ok(Arc::new(f.into())),
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
                        .enumerate()
                        .map(|(index, i)| match i {
                            Ok(
                                i
                                @
                                Import {
                                    ty: ImportSectionEntryType::Function(_),
                                    ..
                                },
                            ) => {
                                imported_func_map.push(index);
                                Ok(i)
                            }
                            Ok(
                                i
                                @
                                Import {
                                    ty: ImportSectionEntryType::Memory(_),
                                    ..
                                },
                            ) => {
                                imported_memories_map.push(index);
                                Ok(i)
                            }
                            Ok(
                                i
                                @
                                Import {
                                    ty: ImportSectionEntryType::Table(_),
                                    ..
                                },
                            ) => {
                                imported_tables_map.push(index);
                                Ok(i)
                            }
                            Ok(
                                i
                                @
                                Import {
                                    ty: ImportSectionEntryType::Global(_),
                                    ..
                                },
                            ) => {
                                imported_globals_map.push(index);
                                Ok(i)
                            }
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
    let imported_memories_map = imported_memories_map.into_boxed_slice();
    let imported_tables_map = imported_tables_map.into_boxed_slice();
    let imported_globals_map = imported_globals_map.into_boxed_slice();
    let imported_func_map = imported_func_map.into_boxed_slice();
    Ok(ModuleData {
        buf,
        types,
        imports,
        exports,
        imported_memories_map,
        memories,
        data,
        imported_tables_map,
        tables,
        elements,
        imported_globals_map,
        globals,
        imported_func_map,
        func_types,
        func_bodies,
        start_func,
        module_name,
    })
}

impl Module {
    pub fn new(buf: Box<[u8]>) -> Result<Module, Error> {
        Ok(Module {
            data: Arc::new(read_module_data(Pin::new(buf))?),
        })
    }

    pub(crate) fn data(&self) -> &Arc<ModuleData> {
        &self.data
    }

    pub fn imports(&self) -> Vec<(String, String, ExternType)> {
        self.data
            .imports
            .iter()
            .map(|e| {
                (
                    e.module.to_string(),
                    e.field.unwrap().to_string(),
                    self.from_import_type(&e.ty),
                )
            })
            .collect::<Vec<_>>()
    }

    pub fn exports(&self) -> Vec<(String, ExternType)> {
        self.data
            .exports
            .iter()
            .map(|e| (e.field.to_string(), self.from_export_type(e)))
            .collect::<Vec<_>>()
    }

    pub fn types<'a>(&'a self) -> impl Iterator<Item = Arc<FuncType>> + 'a {
        self.data.types.iter().map(|ft| ft.clone())
    }

    pub fn memories<'a>(&'a self) -> impl Iterator<Item = externals::MemoryType> + 'a {
        self.data.memories.iter().map(|m| externals::MemoryType {
            limits: match m {
                MemoryType::M32 { limits, .. } => limits.clone().into(),
                _ => panic!(),
            },
        })
    }

    pub fn tables<'a>(&'a self) -> impl Iterator<Item = externals::TableType> + 'a {
        self.data.tables.iter().map(|t| externals::TableType {
            limits: t.limits.clone().into(),
            element: t.element_type.into(),
        })
    }

    pub fn globals<'a>(&'a self) -> impl Iterator<Item = externals::GlobalType> + 'a {
        self.data.globals.iter().map(|g| externals::GlobalType {
            ty: g.ty.content_type.into(),
        })
    }

    pub fn name(&self) -> Option<String> {
        self.data.module_name.clone()
    }

    fn from_import_type(&self, import: &ImportSectionEntryType) -> ExternType {
        match import {
            ImportSectionEntryType::Function(index) => {
                ExternType::Func((*self.data.types[*index as usize]).clone())
            }
            ImportSectionEntryType::Memory(m) => ExternType::Memory(externals::MemoryType {
                limits: match m {
                    MemoryType::M32 { limits, .. } => limits.clone().into(),
                    _ => panic!(),
                },
            }),
            ImportSectionEntryType::Global(g) => ExternType::Global(externals::GlobalType {
                ty: g.content_type.into(),
            }),
            ImportSectionEntryType::Table(t) => ExternType::Table(externals::TableType {
                limits: t.limits.clone().into(),
                element: t.element_type.into(),
            }),
            _ => panic!(),
        }
    }

    fn from_export_type(&self, export: &Export) -> ExternType {
        use crate::externals as ext;
        use wasmparser::ExternalKind::*;
        match export.kind {
            Function => {
                if (export.index as usize) < self.data.imported_func_map.len() {
                    self.from_import_type(
                        &self.data.imports[self.data.imported_func_map[export.index as usize]].ty,
                    )
                } else {
                    let ty = self.data.func_types
                        [export.index as usize - self.data.imported_func_map.len()];
                    ExternType::Func((*self.data.types[ty as usize]).clone())
                }
            }
            Global => {
                if (export.index as usize) < self.data.imported_globals_map.len() {
                    self.from_import_type(
                        &self.data.imports[self.data.imported_globals_map[export.index as usize]]
                            .ty,
                    )
                } else {
                    let g = &self.data.globals
                        [export.index as usize - self.data.imported_globals_map.len()];
                    ExternType::Global(ext::GlobalType {
                        ty: g.ty.content_type.into(),
                    })
                }
            }
            Table => {
                if (export.index as usize) < self.data.imported_tables_map.len() {
                    self.from_import_type(
                        &self.data.imports[self.data.imported_tables_map[export.index as usize]].ty,
                    )
                } else {
                    let t = &self.data.tables
                        [export.index as usize - self.data.imported_tables_map.len()];
                    ExternType::Table(ext::TableType {
                        limits: t.limits.clone().into(),
                        element: t.element_type.into(),
                    })
                }
            }
            Memory => {
                if (export.index as usize) < self.data.imported_memories_map.len() {
                    self.from_import_type(
                        &self.data.imports[self.data.imported_memories_map[export.index as usize]]
                            .ty,
                    )
                } else {
                    let m = &self.data.memories
                        [export.index as usize - self.data.imported_memories_map.len()];
                    ExternType::Memory(ext::MemoryType {
                        limits: match m {
                            MemoryType::M32 { limits, .. } => limits.clone().into(),
                            _ => panic!(),
                        },
                    })
                }
            }
            _ => panic!(),
        }
    }
}

use failure::Error;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use wasmparser::{
    Data, Element, Export, FuncType, FunctionBody, Global, Import, MemoryType, ModuleReader,
    SectionCode, TableType,
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
    let mut reader = {
        let buf = unsafe { &std::slice::from_raw_parts(buf.as_ptr(), buf.len()) };
        ModuleReader::new(buf)?
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
    let mut func_bodies = None;
    let mut start_func = None;
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
            SectionCode::Table => {
                tables = Some(
                    section
                        .get_table_section_reader()?
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
            SectionCode::Data => {
                data = Some(
                    section
                        .get_data_section_reader()?
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            SectionCode::Element => {
                elements = Some(
                    section
                        .get_element_section_reader()?
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            SectionCode::Start => {
                start_func = Some(section.get_start_section_content()?);
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
    let func_bodies = func_bodies.unwrap_or_else(|| vec![]).into_boxed_slice();
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
            .map(|e| (e.module.to_string(), e.field.to_string()))
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

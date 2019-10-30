use crate::externals::{Func, Table, TableOutOfBounds};
use std::cell::RefCell;
use std::rc::Rc;

pub struct InstanceTable<'a> {
    entries: Vec<Option<Rc<RefCell<dyn Func + 'a>>>>,
    #[allow(dead_code)]
    max: usize,
}

impl<'a> InstanceTable<'a> {
    pub fn new(min: usize, max: usize) -> InstanceTable<'a> {
        InstanceTable {
            entries: vec![None; min],
            max,
        }
    }
}

impl<'a> Table<'a> for InstanceTable<'a> {
    fn get_func(&self, index: u32) -> Result<Rc<RefCell<dyn Func + 'a>>, TableOutOfBounds> {
        if (index as usize) < self.entries.len() {
            Ok(self.entries[index as usize].as_ref().unwrap().clone())
        } else {
            Err(TableOutOfBounds)
        }
    }

    fn set_func(
        &mut self,
        index: u32,
        f: Rc<RefCell<dyn Func + 'a>>,
    ) -> Result<(), TableOutOfBounds> {
        if (index as usize) < self.entries.len() {
            self.entries[index as usize] = Some(f);
            Ok(())
        } else {
            Err(TableOutOfBounds)
        }
    }
}

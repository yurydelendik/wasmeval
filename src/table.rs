use crate::externals::{Func, Table, TableOutOfBounds};
use std::cell::RefCell;
use std::rc::Rc;

pub struct InstanceTable {
    entries: Vec<Option<Rc<RefCell<dyn Func>>>>,
    #[allow(dead_code)]
    max: usize,
}

impl InstanceTable {
    pub fn new(min: usize, max: usize) -> InstanceTable {
        InstanceTable {
            entries: vec![None; min],
            max,
        }
    }
}

impl Table for InstanceTable {
    fn get_func(&self, index: u32) -> Result<Option<Rc<RefCell<dyn Func>>>, TableOutOfBounds> {
        if (index as usize) < self.entries.len() {
            Ok(self.entries[index as usize].clone())
        } else {
            Err(TableOutOfBounds)
        }
    }

    fn set_func(
        &mut self,
        index: u32,
        f: Option<Rc<RefCell<dyn Func>>>,
    ) -> Result<(), TableOutOfBounds> {
        if (index as usize) < self.entries.len() {
            self.entries[index as usize] = f;
            Ok(())
        } else {
            Err(TableOutOfBounds)
        }
    }
}

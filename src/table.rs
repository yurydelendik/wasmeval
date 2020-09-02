use crate::externals::{Func, Table, TableOutOfBounds};
use std::cell::RefCell;
use std::rc::Rc;

pub struct InstanceTable {
    entries: RefCell<Vec<Option<Rc<dyn Func>>>>,
    #[allow(dead_code)]
    max: usize,
}

impl InstanceTable {
    pub fn new(min: usize, max: usize) -> InstanceTable {
        InstanceTable {
            entries: RefCell::new(vec![None; min]),
            max,
        }
    }
}

impl Table for InstanceTable {
    fn get_func(&self, index: u32) -> Result<Option<Rc<dyn Func>>, TableOutOfBounds> {
        if (index as usize) < self.entries.borrow().len() {
            Ok(self.entries.borrow()[index as usize].clone())
        } else {
            Err(TableOutOfBounds)
        }
    }

    fn set_func(&self, index: u32, f: Option<Rc<dyn Func>>) -> Result<(), TableOutOfBounds> {
        if (index as usize) < self.entries.borrow().len() {
            self.entries.borrow_mut()[index as usize] = f;
            Ok(())
        } else {
            Err(TableOutOfBounds)
        }
    }
}

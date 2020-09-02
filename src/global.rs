use crate::externals::Global;
use crate::values::Val;
use std::cell::RefCell;

pub struct InstanceGlobal(RefCell<Val>);

impl InstanceGlobal {
    pub fn new(val: Val) -> InstanceGlobal {
        InstanceGlobal(RefCell::new(val))
    }
}

impl Global for InstanceGlobal {
    fn content(&self) -> Val {
        self.0.borrow().clone()
    }
    fn set_content(&self, val: &Val) {
        *self.0.borrow_mut() = val.clone();
    }
}

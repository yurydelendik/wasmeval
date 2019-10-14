use crate::externals::Global;
use crate::values::Val;

pub struct InstanceGlobal(Val);

impl InstanceGlobal {
    pub fn new(val: Val) -> InstanceGlobal {
        InstanceGlobal(val)
    }
}

impl Global for InstanceGlobal {
    fn content(&self) -> &Val {
        &self.0
    }
    fn content_mut(&mut self) -> &mut Val {
        &mut self.0
    }
}

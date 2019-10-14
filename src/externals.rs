use std::cell::RefCell;
use std::rc::Rc;

use crate::values::{Trap, Val};

pub trait Func {
    fn params_arity(&self) -> usize;
    fn results_arity(&self) -> usize;
    fn call(&self, params: &[Val]) -> Result<Box<[Val]>, Rc<RefCell<Trap>>>;
}

pub trait Memory {}

#[derive(Clone)]
pub enum External<'a> {
    Func(Rc<RefCell<dyn Func + 'a>>),
    Memory(Rc<RefCell<dyn Memory>>),
}

impl<'a> External<'a> {
    pub fn func(&self) -> Option<&Rc<RefCell<dyn Func + 'a>>> {
        if let External::Func(f) = self {
            Some(f)
        } else {
            None
        }
    }
}

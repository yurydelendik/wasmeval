use std::cell::{Ref, RefCell};
use std::rc::Rc;
use wasmparser::{Operator, OperatorsReader};

use crate::externals::Func;
use crate::instance::InstanceData;
use crate::values::Val;

pub struct Local(pub Val);

pub(crate) struct EvalContext<'a> {
    pub instance_data: Rc<RefCell<InstanceData<'a>>>,
    pub locals: Vec<Local>,
    pub stack: Vec<Val>,
}

impl<'a> EvalContext<'a> {
    fn get_function(&self, index: u32) -> Rc<RefCell<dyn Func + 'a>> {
        self.instance_data.borrow().funcs[index as usize].clone()
    }
}

pub(crate) trait EvalSource {
    fn create_reader(&self) -> OperatorsReader;
}

pub(crate) fn eval<'a>(context: &'a mut EvalContext, source: &dyn EvalSource) {
    let operators = source
        .create_reader()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("ops");
    let mut i = 0;
    let mut control: Vec<Option<usize>> = Vec::new();
    let mut stack: Vec<Val> = Vec::new();
    loop {
        let op = &operators[i];
        match op {
            Operator::End => {
                if control.is_empty() {
                    break;
                }
                control.pop();
            }
            Operator::Call { function_index } => {
                let f = context.get_function(*function_index);
                let params = stack.split_off(stack.len() - f.borrow().params_arity());
                let result = f.borrow().call(&params);
                match result {
                    Ok(returns) => stack.extend_from_slice(&returns),
                    Err(_) => unimplemented!("call trap"),
                }
            }
            x => unimplemented!("{:?}", x),
        }
        i += 1;
    }
}

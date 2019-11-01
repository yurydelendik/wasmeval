use crate::module::ModuleData;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::vec::Vec;
use wasmparser::OperatorsReader;

pub use wasmparser::Operator;

pub(crate) struct BytecodeCache {
    _module_data: Rc<RefCell<ModuleData>>,
    operators: Vec<Operator<'static>>,
    parents: HashMap<usize, usize>,
    ends: Vec<(usize, usize)>,
    loops: HashMap<usize, usize>,
    elses: HashMap<usize, usize>,
}

pub enum BreakDestination {
    BlockEnd(usize),
    LoopStart(usize),
}

impl BytecodeCache {
    pub fn new(_module_data: Rc<RefCell<ModuleData>>, reader: OperatorsReader<'static>) -> Self {
        let operators = reader
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("ops");
        let mut parents = HashMap::new();
        let mut ends = Vec::new();
        let mut loops = HashMap::new();
        let mut elses = HashMap::new();

        let mut control = Vec::new();
        for i in (0..operators.len()).rev() {
            match operators[i] {
                Operator::End => {
                    if let Some(&(last, _)) = control.last() {
                        parents.insert(i, last);
                        ends.push((i, last));
                    }
                    control.push((i, None));
                }
                Operator::Loop { .. } => {
                    let (end, _) = control.pop().unwrap();
                    ends.push((i, end));
                    loops.insert(end, i);
                }
                Operator::Block { .. } => {
                    let (end, _) = control.pop().unwrap();
                    ends.push((i, end));
                }
                Operator::If { .. } => {
                    let (end, maybe_else) = control.pop().unwrap();
                    if let Some(el) = maybe_else {
                        elses.insert(i, el);
                    }
                    ends.push((i, end));
                }
                Operator::Else => {
                    control.last_mut().unwrap().1 = Some(i);
                }
                _ => (),
            }
        }

        assert!(control.len() == 1);
        ends.push((0, control[0].0));
        ends.reverse();

        BytecodeCache {
            _module_data,
            operators,
            parents,
            ends,
            loops,
            elses,
        }
    }

    pub fn break_to(&self, from: usize, depth: u32) -> BreakDestination {
        let mut end = match self.ends.binary_search_by_key(&from, |&(i, _)| i) {
            Ok(i) => self.ends[i].1,
            Err(i) => self.ends[i - 1].1,
        };
        for _ in 0..depth {
            end = self.parents[&end];
        }
        if let Some(i) = self.loops.get(&end) {
            BreakDestination::LoopStart(*i + 1)
        } else {
            BreakDestination::BlockEnd(end + 1)
        }
    }

    pub fn skip_to_else(&self, from_if: usize) -> usize {
        // TODO assert from_if for if
        if let Some(el) = self.elses.get(&from_if) {
            return *el + 1;
        }
        // No else, skipping
        self.skip_to_end(from_if)
    }

    pub fn skip_to_end(&self, from: usize) -> usize {
        let end = match self.ends.binary_search_by_key(&from, |&(i, _)| i) {
            Ok(i) => self.ends[i].1,
            Err(i) => self.ends[i - 1].1,
        };
        end + 1
    }

    pub fn len(&self) -> usize {
        self.operators.len()
    }

    pub fn operators(&self) -> &[Operator] {
        &self.operators
    }

    pub fn position(&self, i: usize) -> usize {
        // TODO real bytecode position
        i
    }
}

pub(crate) trait EvalSource {
    fn bytecode(&self) -> &BytecodeCache;
}

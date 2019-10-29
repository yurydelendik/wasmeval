pub use crate::externals::{External, Func, Memory, Table};
pub use crate::instance::Instance;
pub use crate::memory::InstanceMemory;
pub use crate::module::Module;
pub use crate::values::{Trap, Val};

mod eval;
mod externals;
mod func;
mod global;
mod instance;
mod memory;
mod module;
mod table;
mod values;

#[cfg(test)]
mod tests;

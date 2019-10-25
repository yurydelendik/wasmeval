pub use crate::externals::{External, Func};
pub use crate::instance::Instance;
pub use crate::module::Module;
pub use crate::values::{Trap, Val};

mod eval;
mod externals;
mod func;
mod global;
mod instance;
mod memory;
mod module;
mod values;

#[cfg(test)]
mod tests;

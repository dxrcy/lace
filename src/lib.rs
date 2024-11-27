// TODO: Make some modules private

// Parsing
mod parser;
pub use parser::AsmParser;
mod air;
pub use air::Air;

// Running
mod runtime;
pub use runtime::RunState;

// Reset global state for watch
mod symbol;
pub use symbol::{reset_state, StaticSource};

mod error;
mod lexer;

mod bin;

pub use bin::main;
pub use traps::Traps;

pub mod traps;

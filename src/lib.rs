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

pub struct Traps {
    // TODO(opt): Use function pointer instead of Option
    array: [Option<TrapFn>; 8],
}

type TrapFn = fn(&mut RunState) -> ();

impl Default for Traps {
    fn default() -> Self {
        Self {
            array: [Default::default(); 8],
        }
    }
}

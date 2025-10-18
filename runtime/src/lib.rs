/*!
Cem Runtime - Memory-safe runtime using May coroutines

This runtime provides:
- Stack operations (dup, swap, drop, etc.)
- Erlang-style green threads via May coroutines
- Pattern matching on sum types
- Async I/O with automatic yielding

All functions are exposed via C FFI for LLVM IR to call.
*/

pub mod stack;
pub mod io;
pub mod scheduler;
pub mod pattern;

// Re-export main types
pub use stack::{StackCell, CellType};

// Initialize May runtime when library loads
// May automatically sets up its work-stealing scheduler
use std::sync::Once;
static INIT: Once = Once::new();

/// Initialize the May runtime (called automatically)
#[no_mangle]
pub extern "C" fn cem_runtime_init() {
    INIT.call_once(|| {
        // May runtime initializes automatically
        // No explicit setup needed unlike your C scheduler!
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_init() {
        cem_runtime_init();
    }
}

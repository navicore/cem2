/*!
Cem Runtime - Memory-safe runtime using May coroutines

Edition 2024 compliant with proper unsafe annotations.
*/

pub mod io;
pub mod pattern;
pub mod scheduler;
pub mod stack;

// Re-export main types
pub use stack::{CellType, StackCell};

/// Initialize the May runtime (called automatically)
///
/// # Safety
/// This function is safe to call multiple times (idempotent).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cem_runtime_init() {
    // May runtime initializes automatically
    // This is a no-op but provided for explicit initialization if needed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_init() {
        unsafe {
            cem_runtime_init();
        }
    }
}

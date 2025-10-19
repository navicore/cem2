/*!
Cem Runtime - Memory-safe runtime using May coroutines

Edition 2024 compliant with proper unsafe annotations.
*/

pub mod conversions;
pub mod io;
pub mod pattern;
pub mod scheduler;
pub mod stack;

// Re-export main types
pub use stack::{CellDataUnion, CellType, StackCell, VariantData};

/// Initialize the May runtime (called automatically)
///
/// # Safety
/// This function is safe to call multiple times (idempotent).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cem_runtime_init() {
    // May runtime initializes automatically
    // This is a no-op but provided for explicit initialization if needed
}

/// Runtime error handler - prints error message and exits
///
/// # Safety
/// - `msg` must be a valid null-terminated C string pointer
/// - This function never returns (calls exit)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_error(msg: *const i8) -> ! {
    if !msg.is_null() {
        let error_msg = unsafe { std::ffi::CStr::from_ptr(msg).to_string_lossy().into_owned() };
        eprintln!("Runtime error: {}", error_msg);
    } else {
        eprintln!("Runtime error: (null message)");
    }
    std::process::exit(1);
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

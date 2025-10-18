/*!
Scheduler - Green Thread Management with May - Edition 2024 compliant
*/

use crate::stack::StackCell;
use may::coroutine;
use std::sync::atomic::{AtomicBool, Ordering};

static SCHEDULER_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize the scheduler
/// # Safety
/// Safe to call multiple times (idempotent).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_init() {
    SCHEDULER_INITIALIZED.store(true, Ordering::SeqCst);
    // May coroutines auto-initialize, no explicit init needed
}

/// Run the scheduler and wait for all coroutines to complete
/// # Safety
/// Returns the final stack (always null for now).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_run() -> *mut StackCell {
    // May's scheduler runs automatically, we just need to wait for completion
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::ptr::null_mut()
}

/// Shutdown the scheduler
/// # Safety
/// Safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_shutdown() {
    SCHEDULER_INITIALIZED.store(false, Ordering::SeqCst);
    // May doesn't require explicit shutdown
}

/// Spawn a strand (coroutine) with initial stack
/// # Safety
/// Entry function must be a valid function pointer.
/// initial_stack must be safe to send to another thread.
/// Returns strand ID (always 0 for now).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strand_spawn(
    entry: extern "C" fn(*mut StackCell) -> *mut StackCell,
    initial_stack: *mut StackCell,
) -> i64 {
    // SAFETY: We're in unsafe FFI land. The caller guarantees the stack is valid
    // for the lifetime of the coroutine. We wrap everything in types that impl Send.
    struct SendableFn(extern "C" fn(*mut StackCell) -> *mut StackCell);
    unsafe impl Send for SendableFn {}

    let entry_fn = SendableFn(entry);
    let stack_addr = initial_stack as usize; // Convert pointer to usize (which is Send)

    unsafe {
        coroutine::spawn(move || {
            let stack_ptr = stack_addr as *mut StackCell;
            let _final_stack = (entry_fn.0)(stack_ptr);
        });
    }
    0 // Return strand ID (0 for now, May doesn't expose IDs)
}

/// Free a stack
/// # Safety
/// Stack pointer must be valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_stack(stack: *mut StackCell) {
    if !stack.is_null() {
        unsafe {
            // Convert to Box and drop to free memory
            let _ = Box::from_raw(stack);
        }
    }
}

/// Legacy spawn_strand function (kept for compatibility)
/// # Safety
/// Entry function must be a valid function pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spawn_strand(entry: extern "C" fn(*mut StackCell) -> *mut StackCell) {
    unsafe {
        strand_spawn(entry, std::ptr::null_mut());
    }
}

/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yield_strand() {
    coroutine::yield_now();
}

/// # Safety
/// Always safe to call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wait_all_strands() {
    std::thread::sleep(std::time::Duration::from_millis(100));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_spawn_strand() {
        unsafe {
            static COUNTER: AtomicU32 = AtomicU32::new(0);

            extern "C" fn test_entry(_stack: *mut StackCell) -> *mut StackCell {
                COUNTER.fetch_add(1, Ordering::SeqCst);
                std::ptr::null_mut()
            }

            for _ in 0..100 {
                spawn_strand(test_entry);
            }

            std::thread::sleep(std::time::Duration::from_millis(200));
            assert_eq!(COUNTER.load(Ordering::SeqCst), 100);
        }
    }
}

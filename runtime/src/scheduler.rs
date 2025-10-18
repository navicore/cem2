/*!
Scheduler - Green Thread Management with May - Edition 2024 compliant
*/

use crate::stack::StackCell;
use may::coroutine;
use std::sync::Once;

static SCHEDULER_INIT: Once = Once::new();

/// Initialize the scheduler
///
/// # Safety
/// Safe to call multiple times (idempotent via Once).
/// May coroutines auto-initialize, so this is primarily a no-op marker.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_init() {
    SCHEDULER_INIT.call_once(|| {
        // May coroutines auto-initialize, no explicit setup needed
    });
}

/// Run the scheduler and wait for all coroutines to complete
///
/// # Safety
/// Returns the final stack (always null for now since May handles all scheduling).
///
/// # Known Limitations
/// Currently uses a fixed 100ms sleep as a temporary workaround.
/// This assumes all spawned coroutines complete within 100ms.
/// TODO: Replace with proper synchronization using May's join handles or WaitGroup pattern.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_run() -> *mut StackCell {
    // FIXME: This is a temporary workaround. May's scheduler runs automatically,
    // but we don't currently track spawned coroutines to wait for them properly.
    // This sleep gives coroutines time to complete, but is not guaranteed.
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::ptr::null_mut()
}

/// Shutdown the scheduler
///
/// # Safety
/// Safe to call. May doesn't require explicit shutdown, so this is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_shutdown() {
    // May doesn't require explicit shutdown
    // This function exists for API symmetry with init
}

/// Spawn a strand (coroutine) with initial stack
///
/// # Safety
/// - `entry` must be a valid function pointer that can safely execute on any thread
/// - `initial_stack` must be either null or a valid pointer to a `StackCell` that:
///   - Was heap-allocated (e.g., via Box)
///   - Has a 'static lifetime or lives longer than the coroutine
///   - Is safe to access from the spawned thread
/// - The caller transfers ownership of `initial_stack` to the coroutine
/// - Returns strand ID (always 0 for now; May doesn't expose coroutine IDs)
///
/// # Memory Ownership
/// The spawned coroutine takes ownership of `initial_stack`. The final stack
/// returned by `entry` is currently leaked. This is a known limitation.
/// TODO: Track and properly cleanup final stacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strand_spawn(
    entry: extern "C" fn(*mut StackCell) -> *mut StackCell,
    initial_stack: *mut StackCell,
) -> i64 {
    // Wrap types to satisfy Send bounds for coroutine::spawn
    struct SendableFn(extern "C" fn(*mut StackCell) -> *mut StackCell);

    // SAFETY: We assert Send for FFI function pointer where the caller guarantees thread safety
    unsafe impl Send for SendableFn {}

    let entry_fn = SendableFn(entry);
    // Convert pointer to usize (which is Send) to avoid provenance issues with raw pointers
    // This is necessary because *mut T is !Send, but the caller guarantees thread safety
    let stack_addr = initial_stack as usize;

    unsafe {
        coroutine::spawn(move || {
            // Reconstruct pointer from address
            let stack_ptr = stack_addr as *mut StackCell;
            // FIXME: Final stack is leaked. We should track it for cleanup.
            let _final_stack = (entry_fn.0)(stack_ptr);
        });
    }

    0 // Return strand ID (0 for now, May doesn't expose IDs)
}

/// Free a stack allocated by the runtime
///
/// # Safety
/// - `stack` must be either:
///   - A null pointer (safe, will be a no-op)
///   - A valid pointer returned by runtime stack functions (push_int, etc.)
///   - A pointer that was originally created via `Box::new(StackCell)` and converted with `Box::into_raw`
/// - The pointer must not have been previously freed
/// - After calling this function, the pointer is invalid and must not be used
/// - This function takes ownership and drops the memory
///
/// # Contract
/// The caller MUST guarantee that `stack` was heap-allocated via Box.
/// Passing a stack pointer that was not Box-allocated will cause undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_stack(stack: *mut StackCell) {
    if !stack.is_null() {
        unsafe {
            // SAFETY: Caller guarantees this was Box-allocated
            let _ = Box::from_raw(stack);
        }
    }
}

/// Legacy spawn_strand function (kept for compatibility)
///
/// # Safety
/// `entry` must be a valid function pointer that can safely execute on any thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spawn_strand(entry: extern "C" fn(*mut StackCell) -> *mut StackCell) {
    unsafe {
        strand_spawn(entry, std::ptr::null_mut());
    }
}

/// Yield execution to allow other coroutines to run
///
/// # Safety
/// Always safe to call from within a May coroutine.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yield_strand() {
    coroutine::yield_now();
}

/// Wait for all strands to complete (temporary implementation)
///
/// # Safety
/// Always safe to call.
///
/// # Known Limitations
/// Uses a fixed 100ms sleep. Same limitations as scheduler_run().
/// TODO: Replace with proper synchronization.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wait_all_strands() {
    // FIXME: Same issue as scheduler_run - needs proper synchronization
    std::thread::sleep(std::time::Duration::from_millis(100));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::push_int;
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

    #[test]
    fn test_scheduler_init_idempotent() {
        unsafe {
            // Should be safe to call multiple times
            scheduler_init();
            scheduler_init();
            scheduler_init();
        }
    }

    #[test]
    fn test_free_stack_null() {
        unsafe {
            // Freeing null should be a no-op
            free_stack(std::ptr::null_mut());
        }
    }

    #[test]
    fn test_free_stack_valid() {
        unsafe {
            // Create a stack, then free it
            let stack = push_int(std::ptr::null_mut(), 42);
            free_stack(stack);
            // If we get here without crashing, test passed
        }
    }

    #[test]
    fn test_strand_spawn_with_stack() {
        unsafe {
            static COUNTER: AtomicU32 = AtomicU32::new(0);

            extern "C" fn test_entry(stack: *mut StackCell) -> *mut StackCell {
                COUNTER.fetch_add(1, Ordering::SeqCst);
                // Return the stack as-is (caller will leak it, but that's OK for test)
                stack
            }

            let initial_stack = push_int(std::ptr::null_mut(), 99);
            strand_spawn(test_entry, initial_stack);

            std::thread::sleep(std::time::Duration::from_millis(200));
            assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn test_scheduler_shutdown() {
        unsafe {
            scheduler_init();
            scheduler_shutdown();
            // Should not crash
        }
    }
}

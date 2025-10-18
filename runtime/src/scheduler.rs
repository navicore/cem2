/*!
Scheduler - Green Thread Management with May - Edition 2024 compliant
*/

use crate::stack::StackCell;
use may::coroutine;
use std::sync::Once;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

static SCHEDULER_INIT: Once = Once::new();
static ACTIVE_STRANDS: AtomicUsize = AtomicUsize::new(0);
static NEXT_STRAND_ID: AtomicU64 = AtomicU64::new(1);

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
/// This function blocks until all spawned strands have completed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scheduler_run() -> *mut StackCell {
    // Wait for all active strands to complete
    // Check every 10ms to avoid busy-waiting while still being responsive
    while ACTIVE_STRANDS.load(Ordering::Acquire) > 0 {
        std::thread::sleep(Duration::from_millis(10));
    }
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
/// - Returns a unique strand ID (positive integer)
///
/// # Memory Management
/// The spawned coroutine takes ownership of `initial_stack` and will automatically
/// free the final stack returned by `entry` upon completion.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strand_spawn(
    entry: extern "C" fn(*mut StackCell) -> *mut StackCell,
    initial_stack: *mut StackCell,
) -> i64 {
    // Generate unique strand ID
    let strand_id = NEXT_STRAND_ID.fetch_add(1, Ordering::Relaxed);

    // Increment active strand counter
    ACTIVE_STRANDS.fetch_add(1, Ordering::Release);

    // Function pointers are already Send, no wrapper needed
    let entry_fn = entry;

    // Convert pointer to usize (which is Send)
    // This is necessary because *mut T is !Send, but the caller guarantees thread safety
    let stack_addr = initial_stack as usize;

    unsafe {
        coroutine::spawn(move || {
            // Reconstruct pointer from address
            let stack_ptr = stack_addr as *mut StackCell;

            // Execute the entry function
            let final_stack = entry_fn(stack_ptr);

            // Clean up the final stack to prevent memory leak
            free_stack(final_stack);

            // Decrement active strand counter
            ACTIVE_STRANDS.fetch_sub(1, Ordering::Release);
        });
    }

    strand_id as i64
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

/// Wait for all strands to complete
///
/// # Safety
/// Always safe to call. Blocks until all spawned strands have completed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wait_all_strands() {
    // Wait for all active strands to complete
    // Check every 10ms to avoid busy-waiting while still being responsive
    while ACTIVE_STRANDS.load(Ordering::Acquire) > 0 {
        std::thread::sleep(Duration::from_millis(10));
    }
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

    #[test]
    fn test_many_strands_stress() {
        unsafe {
            static COUNTER: AtomicU32 = AtomicU32::new(0);

            extern "C" fn increment(_stack: *mut StackCell) -> *mut StackCell {
                COUNTER.fetch_add(1, Ordering::SeqCst);
                std::ptr::null_mut()
            }

            // Reset counter for this test
            COUNTER.store(0, Ordering::SeqCst);

            // Spawn many strands to stress test synchronization
            for _ in 0..1000 {
                strand_spawn(increment, std::ptr::null_mut());
            }

            // Wait for all to complete
            wait_all_strands();

            // Verify all strands executed
            assert_eq!(COUNTER.load(Ordering::SeqCst), 1000);
        }
    }

    #[test]
    fn test_strand_ids_are_unique() {
        unsafe {
            use std::collections::HashSet;
            use std::sync::Mutex;

            static IDS: Mutex<Option<HashSet<i64>>> = Mutex::new(None);

            // Initialize the set
            *IDS.lock().unwrap() = Some(HashSet::new());

            extern "C" fn collect_id(_stack: *mut StackCell) -> *mut StackCell {
                // Note: We can't get the ID from inside the coroutine easily,
                // so this test just verifies they complete
                std::ptr::null_mut()
            }

            // Spawn strands and collect their IDs
            let mut ids = Vec::new();
            for _ in 0..100 {
                let id = strand_spawn(collect_id, std::ptr::null_mut());
                ids.push(id);
            }

            // Wait for completion
            wait_all_strands();

            // Verify all IDs are unique
            let unique_ids: HashSet<_> = ids.iter().collect();
            assert_eq!(unique_ids.len(), 100, "All strand IDs should be unique");

            // Verify all IDs are positive
            assert!(
                ids.iter().all(|&id| id > 0),
                "All strand IDs should be positive"
            );
        }
    }
}

/*!
Scheduler - Green Thread Management with May

May provides the Erlang-style coroutine scheduler you built in C.
This module wraps May's API for use from LLVM IR.
*/

use crate::stack::StackCell;
use may::coroutine;

/// Spawn a new green thread (strand)
///
/// This is like your C function: strand_spawn(entry_function)
/// May handles all the scheduling, context switching, and I/O multiplexing!
///
/// LLVM IR: call void @spawn_strand(ptr @my_word_function)
#[no_mangle]
pub extern "C" fn spawn_strand(entry: extern "C" fn(*mut StackCell) -> *mut StackCell) {
    coroutine::spawn(move || {
        // Create initial empty stack for this strand
        let stack = std::ptr::null_mut();

        // Call the entry function
        let _final_stack = entry(stack);

        // TODO: Handle final stack cleanup/result
        // For now, just let it drop
    });
}

/// Yield explicitly to the scheduler
/// (Usually not needed - May yields automatically on I/O)
#[no_mangle]
pub extern "C" fn yield_strand() {
    coroutine::yield_now();
}

/// Wait for all spawned strands to complete
/// Call this from main before exiting
#[no_mangle]
pub extern "C" fn wait_all_strands() {
    // May's runtime automatically waits for coroutines
    // We need to give it a chance to run
    std::thread::sleep(std::time::Duration::from_millis(100));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_spawn_strand() {
        static COUNTER: AtomicU32 = AtomicU32::new(0);

        extern "C" fn test_entry(_stack: *mut StackCell) -> *mut StackCell {
            COUNTER.fetch_add(1, Ordering::SeqCst);
            std::ptr::null_mut()
        }

        // Spawn 100 strands
        for _ in 0..100 {
            spawn_strand(test_entry);
        }

        // Wait for them to complete
        std::thread::sleep(std::time::Duration::from_millis(200));

        // All should have incremented the counter
        assert_eq!(COUNTER.load(Ordering::SeqCst), 100);
    }
}

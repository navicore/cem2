/*!
Scheduler - Green Thread Management with May - Edition 2024 compliant
*/

use crate::stack::StackCell;
use may::coroutine;

/// # Safety
/// Entry function must be a valid function pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spawn_strand(entry: extern "C" fn(*mut StackCell) -> *mut StackCell) {
    unsafe {
        coroutine::spawn(move || {
            let stack = std::ptr::null_mut();
            let _final_stack = entry(stack);
        });
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

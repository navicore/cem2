/*!
I/O Operations with May Coroutines - C-compatible layout
*/

use crate::stack::{CellDataUnion, CellType, StackCell};
use std::io::{self, Write};

/// # Safety
/// Stack must have a string on top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_line(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "write_line: stack is empty");

    let (rest, cell) = unsafe { StackCell::pop(stack) };

    assert_eq!(
        cell.cell_type,
        CellType::String,
        "write_line: expected string on stack"
    );

    // Get the C string from the union
    let c_str_ptr = unsafe { cell.data.string_ptr };
    assert!(
        !c_str_ptr.is_null(),
        "write_line: unexpected null string pointer"
    );

    let s = unsafe {
        std::ffi::CStr::from_ptr(c_str_ptr)
            .to_string_lossy()
            .into_owned()
    };

    println!("{}", s);
    io::stdout().flush().unwrap();

    // Free the string
    unsafe {
        let _ = std::ffi::CString::from_raw(c_str_ptr);
    }

    rest
}

/// # Safety
/// Returns a new stack with string pushed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn read_line(stack: *mut StackCell) -> *mut StackCell {
    use std::io::BufRead;

    let stdin = io::stdin();
    let mut line = String::new();

    stdin.lock().read_line(&mut line).unwrap();

    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }

    // Convert to C string
    let c_string = std::ffi::CString::new(line).expect("read_line: input contains null byte");

    let cell = Box::new(StackCell {
        cell_type: CellType::String,
        _padding: 0,
        data: CellDataUnion {
            string_ptr: c_string.into_raw(),
        },
        next: std::ptr::null_mut(),
    });

    unsafe { StackCell::push(stack, cell) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::push_string;
    use std::ffi::CString;

    #[test]
    fn test_write_line() {
        unsafe {
            let stack = std::ptr::null_mut();
            let test_str = CString::new("Hello, World!").unwrap();
            let stack = push_string(stack, test_str.as_ptr());
            let _stack = write_line(stack);
        }
    }
}

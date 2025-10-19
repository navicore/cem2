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

    // Get the C string using safe accessor
    let c_str_ptr = cell
        .as_string_ptr()
        .expect("write_line: expected string on stack");

    assert!(
        !c_str_ptr.is_null(),
        "write_line: unexpected null string pointer"
    );

    let s = unsafe {
        match std::ffi::CStr::from_ptr(c_str_ptr).to_str() {
            Ok(s) => s.to_owned(),
            Err(_) => crate::runtime_error(c"write_line: string contains invalid UTF-8".as_ptr()),
        }
    };

    println!("{}", s);
    io::stdout().flush().unwrap();

    // String is automatically freed when cell is dropped
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
    // Note: Cem strings must not contain null bytes (interior nulls are not supported)
    let c_string = std::ffi::CString::new(line).unwrap_or_else(|_| unsafe {
        crate::runtime_error(
            c"read_line: input contains null byte (not supported in Cem strings)".as_ptr(),
        )
    });

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

/// Exit the program with a status code
///
/// # Safety
/// Stack must have an Int on top representing the exit code.
/// Exit code must be in range 0-255 (standard Unix exit code range).
/// This function never returns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn exit_op(stack: *mut StackCell) -> ! {
    assert!(!stack.is_null(), "exit_op: stack is empty");

    let (_rest, cell) = unsafe { StackCell::pop(stack) };

    let exit_code = cell
        .as_int()
        .expect("exit_op: expected integer exit code on stack");

    // Validate exit code is in valid range (0-255 for Unix compatibility)
    if !(0..=255).contains(&exit_code) {
        // unsafe {
        //     crate::runtime_error(c"exit_op: exit code must be in range 0-255".as_ptr())
        // }
        unsafe { crate::runtime_error(c"exit_op: exit code must be in range 0-255".as_ptr()) }
    }

    std::process::exit(exit_code as i32);
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

/*!
I/O Operations with May Coroutines - Edition 2024 compliant
*/

use crate::stack::{CellData, StackCell};
use std::io::{self, Write};

/// # Safety
/// Stack must have a string on top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_line(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "write_line: stack is empty");

    let (rest, cell) = unsafe { StackCell::pop(stack) };

    let s = match cell.data {
        CellData::String(s) => s,
        _ => panic!("write_line: expected string on stack"),
    };

    println!("{}", s);
    io::stdout().flush().unwrap();

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

    let cell = Box::new(StackCell {
        cell_type: crate::stack::CellType::String,
        data: CellData::String(line),
        next: None,
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

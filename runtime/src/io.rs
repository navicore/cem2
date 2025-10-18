/*!
I/O Operations with May Coroutines

May automatically yields on I/O operations, giving you the green thread behavior
you had with your custom scheduler - but without writing the scheduler yourself!
*/

use crate::stack::{CellData, StackCell};
use std::io::{self, Write};

/// Write a string and newline to stdout
/// This automatically yields to May's scheduler while waiting for I/O
///
/// LLVM IR: %stack = call ptr @write_line(ptr %stack)
#[no_mangle]
pub extern "C" fn write_line(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "write_line: stack is empty");

    let (rest, cell) = StackCell::pop(stack);

    let s = match cell.data {
        CellData::String(s) => s,
        _ => panic!("write_line: expected string on stack"),
    };

    // May automatically yields here if stdout blocks!
    // This is the magic - no manual scheduler_yield() needed
    println!("{}", s);
    io::stdout().flush().unwrap();

    rest
}

/// Read a line from stdin
/// This yields to May's scheduler while waiting for input
///
/// LLVM IR: %stack = call ptr @read_line(ptr %stack)
#[no_mangle]
pub extern "C" fn read_line(stack: *mut StackCell) -> *mut StackCell {
    use std::io::BufRead;

    let stdin = io::stdin();
    let mut line = String::new();

    // May yields here while waiting for input
    stdin.lock().read_line(&mut line).unwrap();

    // Remove trailing newline
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }

    // Push the string onto the stack
    let cell = Box::new(StackCell {
        cell_type: crate::stack::CellType::String,
        data: CellData::String(line),
        next: None,
    });

    StackCell::push(stack, cell)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::push_string;
    use std::ffi::CString;

    #[test]
    fn test_write_line() {
        // This test just verifies it doesn't crash
        // Actual output goes to stdout
        let stack = std::ptr::null_mut();
        let test_str = CString::new("Hello, World!").unwrap();
        let stack = push_string(stack, test_str.as_ptr());
        let _stack = write_line(stack);
    }
}

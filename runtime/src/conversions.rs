/*!
Type Conversion Operations
*/

use crate::stack::{StackCell, push_string};
use std::ffi::CString;

/// Convert integer to string
///
/// # Safety
/// Stack must have an integer on top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn int_to_string(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "int_to_string: stack is empty");

    let (rest, cell) = unsafe { StackCell::pop(stack) };

    let int_val = cell
        .as_int()
        .expect("int_to_string: expected integer on stack");

    // Convert to string
    let string = int_val.to_string();
    let c_string = CString::new(string).expect("int_to_string: conversion failed");

    unsafe { push_string(rest, c_string.as_ptr()) }
}

/// Convert boolean to string
///
/// # Safety
/// Stack must have a boolean on top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bool_to_string(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "bool_to_string: stack is empty");

    let (rest, cell) = unsafe { StackCell::pop(stack) };

    let bool_val = cell
        .as_bool()
        .expect("bool_to_string: expected boolean on stack");

    // Convert to string ("true" or "false")
    let string = if bool_val { "true" } else { "false" };
    let c_string = CString::new(string).expect("bool_to_string: conversion failed");

    unsafe { push_string(rest, c_string.as_ptr()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::push_bool;
    use crate::stack::push_int;

    #[test]
    fn test_int_to_string() {
        unsafe {
            let stack = std::ptr::null_mut();
            let stack = push_int(stack, 42);
            let stack = int_to_string(stack);

            let (rest, cell) = StackCell::pop(stack);
            let string_ptr = cell.as_string_ptr().expect("should be string");
            let rust_str = std::ffi::CStr::from_ptr(string_ptr).to_str().unwrap();

            assert_eq!(rust_str, "42");
            assert!(rest.is_null());

            // Clean up
            crate::scheduler::free_stack(std::ptr::null_mut());
        }
    }

    #[test]
    fn test_bool_to_string() {
        unsafe {
            // Test true
            let stack = std::ptr::null_mut();
            let stack = push_bool(stack, true);
            let stack = bool_to_string(stack);

            let (rest, cell) = StackCell::pop(stack);
            let string_ptr = cell.as_string_ptr().expect("should be string");
            let rust_str = std::ffi::CStr::from_ptr(string_ptr).to_str().unwrap();

            assert_eq!(rust_str, "true");
            assert!(rest.is_null());

            // Test false
            let stack = std::ptr::null_mut();
            let stack = push_bool(stack, false);
            let stack = bool_to_string(stack);

            let (rest, cell) = StackCell::pop(stack);
            let string_ptr = cell.as_string_ptr().expect("should be string");
            let rust_str = std::ffi::CStr::from_ptr(string_ptr).to_str().unwrap();

            assert_eq!(rust_str, "false");
            assert!(rest.is_null());

            // Clean up
            crate::scheduler::free_stack(std::ptr::null_mut());
        }
    }

    #[test]
    fn test_negative_int_to_string() {
        unsafe {
            let stack = std::ptr::null_mut();
            let stack = push_int(stack, -123);
            let stack = int_to_string(stack);

            let (rest, cell) = StackCell::pop(stack);
            let string_ptr = cell.as_string_ptr().expect("should be string");
            let rust_str = std::ffi::CStr::from_ptr(string_ptr).to_str().unwrap();

            assert_eq!(rust_str, "-123");
            assert!(rest.is_null());

            // Clean up
            crate::scheduler::free_stack(std::ptr::null_mut());
        }
    }
}

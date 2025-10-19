/*!
String Operations - C-compatible string manipulation
*/

use crate::stack::{StackCell, push_bool, push_int, push_string};
use std::ffi::CString;

/// Get the length of a string
///
/// # Safety
/// Stack must have a string on top.
/// Returns stack with integer length pushed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn string_length(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "string_length: stack is empty");

    let (rest, cell) = unsafe { StackCell::pop(stack) };

    let string_ptr = cell
        .as_string_ptr()
        .expect("string_length: expected string on stack");

    assert!(
        !string_ptr.is_null(),
        "string_length: unexpected null string pointer"
    );

    // Get length of C string (excluding null terminator)
    let length = unsafe { std::ffi::CStr::from_ptr(string_ptr).to_bytes().len() as i64 };

    // String is freed by cell Drop
    unsafe { push_int(rest, length) }
}

/// Concatenate two strings
///
/// # Safety
/// Stack must have two strings: ( str1 str2 -- str1+str2 )
/// Top of stack is str2, second is str1.
/// Result is str1 concatenated with str2.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn string_concat(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "string_concat: stack is empty");

    let (rest, cell2) = unsafe { StackCell::pop(stack) };
    assert!(!rest.is_null(), "string_concat: need two strings");
    let (rest, cell1) = unsafe { StackCell::pop(rest) };

    let str1_ptr = cell1
        .as_string_ptr()
        .expect("string_concat: first argument must be string");
    let str2_ptr = cell2
        .as_string_ptr()
        .expect("string_concat: second argument must be string");

    assert!(!str1_ptr.is_null(), "string_concat: first string is null");
    assert!(!str2_ptr.is_null(), "string_concat: second string is null");

    // Convert C strings to Rust strings
    let s1 = unsafe {
        std::ffi::CStr::from_ptr(str1_ptr)
            .to_str()
            .expect("string_concat: first string contains invalid UTF-8")
    };
    let s2 = unsafe {
        std::ffi::CStr::from_ptr(str2_ptr)
            .to_str()
            .expect("string_concat: second string contains invalid UTF-8")
    };

    // Concatenate
    let result = format!("{}{}", s1, s2);
    let c_result = CString::new(result).expect("string_concat: result contains null byte");

    // Strings are freed by cell Drop
    unsafe { push_string(rest, c_result.as_ptr()) }
}

/// Compare two strings for equality
///
/// # Safety
/// Stack must have two strings: ( str1 str2 -- bool )
/// Returns true if strings are equal, false otherwise.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn string_equal(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "string_equal: stack is empty");

    let (rest, cell2) = unsafe { StackCell::pop(stack) };
    assert!(!rest.is_null(), "string_equal: need two strings");
    let (rest, cell1) = unsafe { StackCell::pop(rest) };

    let str1_ptr = cell1
        .as_string_ptr()
        .expect("string_equal: first argument must be string");
    let str2_ptr = cell2
        .as_string_ptr()
        .expect("string_equal: second argument must be string");

    assert!(!str1_ptr.is_null(), "string_equal: first string is null");
    assert!(!str2_ptr.is_null(), "string_equal: second string is null");

    // Convert C strings to Rust strings and compare
    let s1 = unsafe {
        std::ffi::CStr::from_ptr(str1_ptr)
            .to_str()
            .expect("string_equal: first string contains invalid UTF-8")
    };
    let s2 = unsafe {
        std::ffi::CStr::from_ptr(str2_ptr)
            .to_str()
            .expect("string_equal: second string contains invalid UTF-8")
    };

    let result = s1 == s2;

    // Strings are freed by cell Drop
    unsafe { push_bool(rest, result) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_string_length() {
        unsafe {
            let stack = std::ptr::null_mut();
            let test_str = CString::new("hello").unwrap();
            let stack = push_string(stack, test_str.as_ptr());
            let stack = string_length(stack);

            let (rest, cell) = StackCell::pop(stack);
            let length = cell.as_int().expect("should be int");

            assert_eq!(length, 5);
            assert!(rest.is_null());
        }
    }

    #[test]
    fn test_string_length_empty() {
        unsafe {
            let stack = std::ptr::null_mut();
            let test_str = CString::new("").unwrap();
            let stack = push_string(stack, test_str.as_ptr());
            let stack = string_length(stack);

            let (rest, cell) = StackCell::pop(stack);
            let length = cell.as_int().expect("should be int");

            assert_eq!(length, 0);
            assert!(rest.is_null());
        }
    }

    #[test]
    fn test_string_concat() {
        unsafe {
            let stack = std::ptr::null_mut();
            let str1 = CString::new("hello").unwrap();
            let str2 = CString::new(" world").unwrap();

            let stack = push_string(stack, str1.as_ptr());
            let stack = push_string(stack, str2.as_ptr());
            let stack = string_concat(stack);

            let (rest, cell) = StackCell::pop(stack);
            let result_ptr = cell.as_string_ptr().expect("should be string");
            let result = std::ffi::CStr::from_ptr(result_ptr).to_str().unwrap();

            assert_eq!(result, "hello world");
            assert!(rest.is_null());
        }
    }

    #[test]
    fn test_string_concat_empty() {
        unsafe {
            let stack = std::ptr::null_mut();
            let str1 = CString::new("hello").unwrap();
            let str2 = CString::new("").unwrap();

            let stack = push_string(stack, str1.as_ptr());
            let stack = push_string(stack, str2.as_ptr());
            let stack = string_concat(stack);

            let (rest, cell) = StackCell::pop(stack);
            let result_ptr = cell.as_string_ptr().expect("should be string");
            let result = std::ffi::CStr::from_ptr(result_ptr).to_str().unwrap();

            assert_eq!(result, "hello");
            assert!(rest.is_null());
        }
    }

    #[test]
    fn test_string_equal_true() {
        unsafe {
            let stack = std::ptr::null_mut();
            let str1 = CString::new("test").unwrap();
            let str2 = CString::new("test").unwrap();

            let stack = push_string(stack, str1.as_ptr());
            let stack = push_string(stack, str2.as_ptr());
            let stack = string_equal(stack);

            let (rest, cell) = StackCell::pop(stack);
            let result = cell.as_bool().expect("should be bool");

            assert!(result);
            assert!(rest.is_null());
        }
    }

    #[test]
    fn test_string_equal_false() {
        unsafe {
            let stack = std::ptr::null_mut();
            let str1 = CString::new("hello").unwrap();
            let str2 = CString::new("world").unwrap();

            let stack = push_string(stack, str1.as_ptr());
            let stack = push_string(stack, str2.as_ptr());
            let stack = string_equal(stack);

            let (rest, cell) = StackCell::pop(stack);
            let result = cell.as_bool().expect("should be bool");

            assert!(!result);
            assert!(rest.is_null());
        }
    }

    #[test]
    fn test_string_equal_different_lengths() {
        unsafe {
            let stack = std::ptr::null_mut();
            let str1 = CString::new("hi").unwrap();
            let str2 = CString::new("hello").unwrap();

            let stack = push_string(stack, str1.as_ptr());
            let stack = push_string(stack, str2.as_ptr());
            let stack = string_equal(stack);

            let (rest, cell) = StackCell::pop(stack);
            let result = cell.as_bool().expect("should be bool");

            assert!(!result);
            assert!(rest.is_null());
        }
    }
}

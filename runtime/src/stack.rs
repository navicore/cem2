/*!
Stack Cell Implementation - Edition 2024 compliant

Memory-safe stack operations using Rust's Box.
*/

use std::ptr;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellType {
    Int,
    Bool,
    String,
    Variant,
}

#[derive(Debug, Clone)]
pub enum CellData {
    Int(i64),
    Bool(bool),
    String(String),
    Variant {
        tag: u32,
        fields: Vec<Box<StackCell>>,
    },
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct StackCell {
    pub cell_type: CellType,
    pub data: CellData,
    pub next: Option<Box<StackCell>>,
}

impl StackCell {
    /// # Safety
    /// Stack pointer must be a valid StackCell or null.
    pub unsafe fn pop(stack: *mut StackCell) -> (*mut StackCell, Box<StackCell>) {
        assert!(!stack.is_null(), "pop: stack is empty");
        unsafe {
            let mut cell = Box::from_raw(stack);
            let rest = match cell.next.take() {
                Some(b) => Box::into_raw(b),
                None => ptr::null_mut(),
            };
            (rest, cell)
        }
    }

    /// # Safety
    /// Stack pointer must be a valid StackCell or null.
    pub unsafe fn push(stack: *mut StackCell, mut cell: Box<StackCell>) -> *mut StackCell {
        cell.next = if stack.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(stack) })
        };
        Box::into_raw(cell)
    }
}

// FFI functions - all properly marked unsafe for edition 2024

/// # Safety
/// Caller must ensure stack pointer is valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn push_int(stack: *mut StackCell, value: i64) -> *mut StackCell {
    let cell = Box::new(StackCell {
        cell_type: CellType::Int,
        data: CellData::Int(value),
        next: None,
    });
    unsafe { StackCell::push(stack, cell) }
}

/// # Safety
/// Caller must ensure stack pointer is valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn push_bool(stack: *mut StackCell, value: bool) -> *mut StackCell {
    let cell = Box::new(StackCell {
        cell_type: CellType::Bool,
        data: CellData::Bool(value),
        next: None,
    });
    unsafe { StackCell::push(stack, cell) }
}

/// # Safety
/// Caller must ensure both stack and string pointers are valid. String must be null-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn push_string(stack: *mut StackCell, s: *const i8) -> *mut StackCell {
    let s = unsafe {
        assert!(!s.is_null(), "push_string: null string pointer");
        std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned()
    };

    let cell = Box::new(StackCell {
        cell_type: CellType::String,
        data: CellData::String(s),
        next: None,
    });
    unsafe { StackCell::push(stack, cell) }
}

/// # Safety
/// Stack must not be empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dup(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "dup: stack is empty");

    unsafe {
        let top = &*stack;
        let duplicated = Box::new(StackCell {
            cell_type: top.cell_type,
            data: match &top.data {
                CellData::Int(n) => CellData::Int(*n),
                CellData::Bool(b) => CellData::Bool(*b),
                CellData::String(s) => CellData::String(s.clone()),
                CellData::Variant { tag, fields } => CellData::Variant {
                    tag: *tag,
                    fields: fields.clone(),
                },
            },
            next: None,
        });
        StackCell::push(stack, duplicated)
    }
}

/// # Safety
/// Stack can be empty (returns null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn drop(stack: *mut StackCell) -> *mut StackCell {
    if stack.is_null() {
        return ptr::null_mut();
    }
    let (rest, _cell) = unsafe { StackCell::pop(stack) };
    rest
}

/// # Safety
/// Stack must have at least 2 elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn swap(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "swap: stack too small");
    let (rest, b) = unsafe { StackCell::pop(stack) };
    assert!(!rest.is_null(), "swap: stack too small");
    let (rest, a) = unsafe { StackCell::pop(rest) };
    let rest = unsafe { StackCell::push(rest, b) };
    unsafe { StackCell::push(rest, a) }
}

/// # Safety
/// Stack must have at least 2 elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn over(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "over: stack too small");

    unsafe {
        let top = &*stack;
        let second = top.next.as_ref().expect("over: stack too small");

        let duplicated = Box::new(StackCell {
            cell_type: second.cell_type,
            data: match &second.data {
                CellData::Int(n) => CellData::Int(*n),
                CellData::Bool(b) => CellData::Bool(*b),
                CellData::String(s) => CellData::String(s.clone()),
                CellData::Variant { tag, fields } => CellData::Variant {
                    tag: *tag,
                    fields: fields.clone(),
                },
            },
            next: None,
        });
        StackCell::push(stack, duplicated)
    }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let result = match (a.data, b.data) {
        (CellData::Int(x), CellData::Int(y)) => x + y,
        _ => panic!("add: type error"),
    };

    unsafe { push_int(rest, result) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let result = match (a.data, b.data) {
        (CellData::Int(x), CellData::Int(y)) => x * y,
        _ => panic!("multiply: type error"),
    };

    unsafe { push_int(rest, result) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        unsafe {
            let stack = ptr::null_mut();
            let stack = push_int(stack, 42);
            let (rest, cell) = StackCell::pop(stack);

            assert!(rest.is_null());
            assert_eq!(cell.cell_type, CellType::Int);
            match cell.data {
                CellData::Int(n) => assert_eq!(n, 42),
                _ => panic!("wrong type"),
            }
        }
    }

    #[test]
    fn test_dup() {
        unsafe {
            let stack = ptr::null_mut();
            let stack = push_int(stack, 42);
            let stack = dup(stack);

            let (rest, top) = StackCell::pop(stack);
            let (rest, second) = StackCell::pop(rest);

            assert!(rest.is_null());
            match (top.data, second.data) {
                (CellData::Int(a), CellData::Int(b)) => {
                    assert_eq!(a, 42);
                    assert_eq!(b, 42);
                }
                _ => panic!("wrong type"),
            }
        }
    }

    #[test]
    fn test_swap() {
        unsafe {
            let stack = ptr::null_mut();
            let stack = push_int(stack, 1);
            let stack = push_int(stack, 2);
            let stack = swap(stack);

            let (rest, top) = StackCell::pop(stack);
            let (rest, second) = StackCell::pop(rest);

            assert!(rest.is_null());
            match (top.data, second.data) {
                (CellData::Int(a), CellData::Int(b)) => {
                    assert_eq!(a, 1);
                    assert_eq!(b, 2);
                }
                _ => panic!("wrong type"),
            }
        }
    }

    #[test]
    fn test_arithmetic() {
        unsafe {
            let stack = ptr::null_mut();
            let stack = push_int(stack, 6);
            let stack = push_int(stack, 7);
            let stack = multiply(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());

            match result.data {
                CellData::Int(n) => assert_eq!(n, 42),
                _ => panic!("wrong type"),
            }
        }
    }
}

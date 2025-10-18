/*!
Stack Cell Implementation

This is your Cem data stack - the concatenative language stack where values live.
NOT the OS call stack - this is fully controlled by us.

Memory safety: Using Rust's Box for heap allocation means:
- No use-after-free (ownership system)
- No double-free (Drop trait)
- No memory leaks (automatic cleanup)
*/

use std::ptr;

/// Cell types matching your original cem implementation
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellType {
    Int,
    Bool,
    String,
    Variant,  // Sum type (Option, Result, custom ADTs)
}

/// The actual data in a stack cell
#[derive(Debug)]
pub enum CellData {
    Int(i64),
    Bool(bool),
    String(String),
    Variant {
        tag: u32,           // Which variant (e.g., Some=0, None=1)
        fields: Vec<Box<StackCell>>,  // Variant payload (safe!)
    },
}

/// Stack cell - linked list node
/// This is heap-allocated and forms your concatenative stack
#[repr(C)]
pub struct StackCell {
    pub cell_type: CellType,
    pub data: CellData,
    pub next: Option<Box<StackCell>>,
}

impl StackCell {
    /// Pop a value from the stack
    pub fn pop(stack: *mut StackCell) -> (*mut StackCell, Box<StackCell>) {
        assert!(!stack.is_null(), "pop: stack is empty");

        unsafe {
            let cell = Box::from_raw(stack);
            let rest = cell.next.map_or(ptr::null_mut(), |b| Box::into_raw(b));
            (rest, cell)
        }
    }

    /// Push a value onto the stack
    pub fn push(stack: *mut StackCell, mut cell: Box<StackCell>) -> *mut StackCell {
        cell.next = if stack.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(stack) })
        };
        Box::into_raw(cell)
    }
}

// ============================================================================
// Stack Operations - Called from LLVM IR
// ============================================================================

/// Push an integer onto the stack
/// LLVM IR: %stack = call ptr @push_int(ptr %stack, i64 42)
#[no_mangle]
pub extern "C" fn push_int(stack: *mut StackCell, value: i64) -> *mut StackCell {
    let cell = Box::new(StackCell {
        cell_type: CellType::Int,
        data: CellData::Int(value),
        next: None,
    });
    StackCell::push(stack, cell)
}

/// Push a boolean onto the stack
#[no_mangle]
pub extern "C" fn push_bool(stack: *mut StackCell, value: bool) -> *mut StackCell {
    let cell = Box::new(StackCell {
        cell_type: CellType::Bool,
        data: CellData::Bool(value),
        next: None,
    });
    StackCell::push(stack, cell)
}

/// Push a string onto the stack
/// LLVM IR passes C string pointer, we convert to Rust String
#[no_mangle]
pub extern "C" fn push_string(stack: *mut StackCell, s: *const i8) -> *mut StackCell {
    let s = unsafe {
        assert!(!s.is_null(), "push_string: null string pointer");
        std::ffi::CStr::from_ptr(s)
            .to_string_lossy()
            .into_owned()
    };

    let cell = Box::new(StackCell {
        cell_type: CellType::String,
        data: CellData::String(s),
        next: None,
    });
    StackCell::push(stack, cell)
}

/// Duplicate top stack element
/// ( a -- a a )
#[no_mangle]
pub extern "C" fn dup(stack: *mut StackCell) -> *mut StackCell {
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

/// Drop top stack element
/// ( a -- )
#[no_mangle]
pub extern "C" fn drop(stack: *mut StackCell) -> *mut StackCell {
    if stack.is_null() {
        return ptr::null_mut();
    }

    let (rest, _cell) = StackCell::pop(stack);
    // _cell automatically freed here (Rust Drop trait!)
    rest
}

/// Swap top two elements
/// ( a b -- b a )
#[no_mangle]
pub extern "C" fn swap(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "swap: stack too small");

    let (rest, b) = StackCell::pop(stack);
    assert!(!rest.is_null(), "swap: stack too small");

    let (rest, a) = StackCell::pop(rest);

    let rest = StackCell::push(rest, b);
    StackCell::push(rest, a)
}

/// Over: Copy second element to top
/// ( a b -- a b a )
#[no_mangle]
pub extern "C" fn over(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "over: stack too small");

    unsafe {
        let top = &*stack;
        let second = &*top.next.as_ref().expect("over: stack too small");

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

// ============================================================================
// Arithmetic Operations
// ============================================================================

#[no_mangle]
pub extern "C" fn add(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = StackCell::pop(stack);
    let (rest, a) = StackCell::pop(rest);

    let result = match (a.data, b.data) {
        (CellData::Int(x), CellData::Int(y)) => x + y,
        _ => panic!("add: type error"),
    };

    push_int(rest, result)
}

#[no_mangle]
pub extern "C" fn multiply(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = StackCell::pop(stack);
    let (rest, a) = StackCell::pop(rest);

    let result = match (a.data, b.data) {
        (CellData::Int(x), CellData::Int(y)) => x * y,
        _ => panic!("multiply: type error"),
    };

    push_int(rest, result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
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

    #[test]
    fn test_dup() {
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

    #[test]
    fn test_swap() {
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

    #[test]
    fn test_arithmetic() {
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

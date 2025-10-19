/*!
Stack Cell Implementation - C-compatible layout for LLVM codegen

CRITICAL: This module uses C-compatible memory layout to match LLVM codegen assumptions.
The StackCell structure MUST have this exact layout:

Memory Layout (64-bit):
- cell_type: 4 bytes (i32) at offset 0
- _padding: 4 bytes at offset 4
- data union: 16 bytes at offset 8
  - int_val: 8 bytes (i64)
  - bool_val: 1 byte (bool) + 7 bytes padding
  - string_ptr: 8 bytes (*mut i8)
  - quotation_ptr: 8 bytes (*mut ())
  - variant: 16 bytes (u32 tag + u32 padding + *mut StackCell data)
- next: 8 bytes (*mut StackCell) at offset 24
  TOTAL: 32 bytes
*/

use std::ptr;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellType {
    Int = 0,
    Bool = 1,
    String = 2,
    Variant = 3,
}

/// Variant data - matches C layout: { uint32_t tag; uint32_t padding; void* data; }
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VariantData {
    pub tag: u32,
    pub _padding: u32,
    pub data: *mut StackCell,
}

impl std::fmt::Debug for VariantData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Variant(tag={}, data={:?})", self.tag, self.data)
    }
}

/// Cell data union - 16 bytes
#[repr(C)]
#[derive(Copy, Clone)]
pub union CellDataUnion {
    pub int_val: i64,
    pub bool_val: bool,
    pub string_ptr: *mut i8,
    pub quotation_ptr: *mut (),
    pub variant: VariantData,
}

impl std::fmt::Debug for CellDataUnion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<union>")
    }
}

/// Stack cell - C-compatible layout for LLVM
#[repr(C)]
pub struct StackCell {
    pub cell_type: CellType,
    pub _padding: u32,
    pub data: CellDataUnion,
    pub next: *mut StackCell,
}

impl std::fmt::Debug for StackCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StackCell({:?}, next={:?})", self.cell_type, self.next)
    }
}

impl Drop for StackCell {
    fn drop(&mut self) {
        unsafe {
            match self.cell_type {
                CellType::String => {
                    if !self.data.string_ptr.is_null() {
                        let _ = std::ffi::CString::from_raw(self.data.string_ptr);
                    }
                }
                CellType::Variant => {
                    if !self.data.variant.data.is_null() {
                        let _ = Box::from_raw(self.data.variant.data);
                    }
                }
                _ => {}
            }
        }
    }
}

impl StackCell {
    /// Safe accessor for integer value
    ///
    /// # Returns
    /// `Some(value)` if cell contains an integer, `None` otherwise
    pub fn as_int(&self) -> Option<i64> {
        match self.cell_type {
            CellType::Int => Some(unsafe { self.data.int_val }),
            _ => None,
        }
    }

    /// Safe accessor for boolean value
    ///
    /// # Returns
    /// `Some(value)` if cell contains a boolean, `None` otherwise
    pub fn as_bool(&self) -> Option<bool> {
        match self.cell_type {
            CellType::Bool => Some(unsafe { self.data.bool_val }),
            _ => None,
        }
    }

    /// Safe accessor for string pointer
    ///
    /// # Returns
    /// `Some(ptr)` if cell contains a string, `None` otherwise
    pub fn as_string_ptr(&self) -> Option<*mut i8> {
        match self.cell_type {
            CellType::String => Some(unsafe { self.data.string_ptr }),
            _ => None,
        }
    }

    /// Safe accessor for variant data
    ///
    /// # Returns
    /// `Some(variant_data)` if cell contains a variant, `None` otherwise
    pub fn as_variant(&self) -> Option<VariantData> {
        match self.cell_type {
            CellType::Variant => Some(unsafe { self.data.variant }),
            _ => None,
        }
    }

    /// # Safety
    /// Stack pointer must be a valid StackCell or null.
    pub unsafe fn pop(stack: *mut StackCell) -> (*mut StackCell, Box<StackCell>) {
        assert!(!stack.is_null(), "pop: stack is empty");
        unsafe {
            let cell = Box::from_raw(stack);
            let rest = cell.next;
            (rest, cell)
        }
    }

    /// # Safety
    /// Stack pointer must be a valid StackCell or null.
    pub unsafe fn push(stack: *mut StackCell, mut cell: Box<StackCell>) -> *mut StackCell {
        cell.next = stack;
        Box::into_raw(cell)
    }

    /// Deep clone a cell (recursively clones heap-allocated data)
    ///
    /// # Safety
    /// Cell pointer must be valid. This properly deep-copies all heap allocations
    /// to prevent double-free issues.
    pub unsafe fn deep_clone(cell: &StackCell) -> StackCell {
        match cell.cell_type {
            CellType::Int => {
                let int_val = cell.as_int().expect("deep_clone: invalid Int cell");
                StackCell {
                    cell_type: CellType::Int,
                    _padding: 0,
                    data: CellDataUnion { int_val },
                    next: ptr::null_mut(),
                }
            }
            CellType::Bool => {
                let bool_val = cell.as_bool().expect("deep_clone: invalid Bool cell");
                StackCell {
                    cell_type: CellType::Bool,
                    _padding: 0,
                    data: CellDataUnion { bool_val },
                    next: ptr::null_mut(),
                }
            }
            CellType::String => {
                // Deep copy the string (should already be valid UTF-8)
                let original_ptr = cell
                    .as_string_ptr()
                    .expect("deep_clone: invalid String cell");
                let rust_str = unsafe {
                    std::ffi::CStr::from_ptr(original_ptr)
                        .to_str()
                        .expect("deep_clone: string should be valid UTF-8")
                        .to_owned()
                };
                let new_c_str = std::ffi::CString::new(rust_str)
                    .expect("deep_clone: string should not contain null bytes");
                StackCell {
                    cell_type: CellType::String,
                    _padding: 0,
                    data: CellDataUnion {
                        string_ptr: new_c_str.into_raw(),
                    },
                    next: ptr::null_mut(),
                }
            }
            CellType::Variant => {
                // Deep copy the variant and its field data (recursively)
                let variant = cell.as_variant().expect("deep_clone: invalid Variant cell");
                let cloned_data = if variant.data.is_null() {
                    ptr::null_mut()
                } else {
                    // Recursively deep-clone the field cell
                    unsafe {
                        let field = &*variant.data;
                        Box::into_raw(Box::new(Self::deep_clone(field)))
                    }
                };
                StackCell {
                    cell_type: CellType::Variant,
                    _padding: 0,
                    data: CellDataUnion {
                        variant: VariantData {
                            tag: variant.tag,
                            _padding: 0,
                            data: cloned_data,
                        },
                    },
                    next: ptr::null_mut(),
                }
            }
        }
    }
}

// ============================================================================
// FFI functions - all properly marked unsafe for edition 2024
// ============================================================================

/// # Safety
/// Caller must ensure stack pointer is valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn push_int(stack: *mut StackCell, value: i64) -> *mut StackCell {
    let cell = Box::new(StackCell {
        cell_type: CellType::Int,
        _padding: 0,
        data: CellDataUnion { int_val: value },
        next: ptr::null_mut(),
    });
    unsafe { StackCell::push(stack, cell) }
}

/// # Safety
/// Caller must ensure stack pointer is valid or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn push_bool(stack: *mut StackCell, value: bool) -> *mut StackCell {
    let cell = Box::new(StackCell {
        cell_type: CellType::Bool,
        _padding: 0,
        data: CellDataUnion { bool_val: value },
        next: ptr::null_mut(),
    });
    unsafe { StackCell::push(stack, cell) }
}

/// # Safety
/// Caller must ensure both stack and string pointers are valid. String must be null-terminated and valid UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn push_string(stack: *mut StackCell, s: *const i8) -> *mut StackCell {
    assert!(!s.is_null(), "push_string: null string pointer");

    // Copy the C string to owned Rust String, then back to C string
    // Validate UTF-8 encoding
    let rust_string = unsafe {
        match std::ffi::CStr::from_ptr(s).to_str() {
            Ok(s) => s.to_owned(),
            Err(_) => crate::runtime_error(c"push_string: string contains invalid UTF-8".as_ptr()),
        }
    };

    let c_string = std::ffi::CString::new(rust_string).unwrap_or_else(|_| unsafe {
        crate::runtime_error(c"push_string: string contains null byte".as_ptr())
    });
    let owned_ptr = c_string.into_raw();

    let cell = Box::new(StackCell {
        cell_type: CellType::String,
        _padding: 0,
        data: CellDataUnion {
            string_ptr: owned_ptr,
        },
        next: ptr::null_mut(),
    });
    unsafe { StackCell::push(stack, cell) }
}

/// # Safety
/// Stack must not be empty.
/// Deep-copies all heap-allocated data to prevent double-free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dup(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "dup: stack is empty");

    unsafe {
        let top = &*stack;
        let duplicated = Box::new(StackCell::deep_clone(top));
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

    unsafe {
        let (rest, _cell) = StackCell::pop(stack);
        // Cell is automatically cleaned up by Drop impl
        rest
    }
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
/// Deep-copies all heap-allocated data to prevent double-free.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn over(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "over: stack too small");

    unsafe {
        let top = &*stack;
        assert!(!top.next.is_null(), "over: stack too small");
        let second = &*top.next;

        let duplicated = Box::new(StackCell::deep_clone(second));
        StackCell::push(stack, duplicated)
    }
}

/// # Safety
/// Stack must have at least 3 elements.
/// rot: ( A B C -- B C A )
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rot(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "rot: stack too small");
    let (rest, c) = unsafe { StackCell::pop(stack) };
    assert!(!rest.is_null(), "rot: stack too small");
    let (rest, b) = unsafe { StackCell::pop(rest) };
    assert!(!rest.is_null(), "rot: stack too small");
    let (rest, a) = unsafe { StackCell::pop(rest) };

    // ( A B C -- B C A )
    let rest = unsafe { StackCell::push(rest, b) };
    let rest = unsafe { StackCell::push(rest, c) };
    unsafe { StackCell::push(rest, a) }
}

/// # Safety
/// Stack must have at least 2 elements.
/// nip: ( A B -- B )
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nip(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "nip: stack too small");
    let (rest, b) = unsafe { StackCell::pop(stack) };
    assert!(!rest.is_null(), "nip: stack too small");
    let (rest, _a) = unsafe { StackCell::pop(rest) };
    // Drop a, keep b
    unsafe { StackCell::push(rest, b) }
}

/// # Safety
/// Stack must have at least 2 elements.
/// Deep-copies the top element to prevent double-free.
/// tuck: ( A B -- B A B )
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tuck(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "tuck: stack too small");
    let (rest, b) = unsafe { StackCell::pop(stack) };
    assert!(!rest.is_null(), "tuck: stack too small");
    let (rest, a) = unsafe { StackCell::pop(rest) };

    // ( A B -- B A B )
    // Need to clone B since it appears twice in result
    let b_clone = unsafe { Box::new(StackCell::deep_clone(&b)) };
    let rest = unsafe { StackCell::push(rest, b) };
    let rest = unsafe { StackCell::push(rest, a) };
    unsafe { StackCell::push(rest, b_clone) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let a_val = a.as_int().expect("add: first operand must be an integer");
    let b_val = b.as_int().expect("add: second operand must be an integer");

    let result = a_val + b_val;
    unsafe { push_int(rest, result) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let a_val = a
        .as_int()
        .expect("multiply: first operand must be an integer");
    let b_val = b
        .as_int()
        .expect("multiply: second operand must be an integer");

    let result = a_val * b_val;
    unsafe { push_int(rest, result) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn subtract(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let a_val = a
        .as_int()
        .expect("subtract: first operand must be an integer");
    let b_val = b
        .as_int()
        .expect("subtract: second operand must be an integer");

    let result = a_val - b_val;
    unsafe { push_int(rest, result) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn divide(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let a_val = a
        .as_int()
        .expect("divide: first operand must be an integer");
    let b_val = b
        .as_int()
        .expect("divide: second operand must be an integer");

    assert!(b_val != 0, "divide: division by zero");

    let result = a_val / b_val;
    unsafe { push_int(rest, result) }
}

// ============================================================================
// Comparison operations
// ============================================================================

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eq(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let a_val = a.as_int().expect("eq: first operand must be an integer");
    let b_val = b.as_int().expect("eq: second operand must be an integer");

    let result = a_val == b_val;
    unsafe { push_bool(rest, result) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lt(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let a_val = a.as_int().expect("lt: first operand must be an integer");
    let b_val = b.as_int().expect("lt: second operand must be an integer");

    let result = a_val < b_val;
    unsafe { push_bool(rest, result) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gt(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let a_val = a.as_int().expect("gt: first operand must be an integer");
    let b_val = b.as_int().expect("gt: second operand must be an integer");

    let result = a_val > b_val;
    unsafe { push_bool(rest, result) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn le(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let a_val = a.as_int().expect("le: first operand must be an integer");
    let b_val = b.as_int().expect("le: second operand must be an integer");

    let result = a_val <= b_val;
    unsafe { push_bool(rest, result) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ge(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let a_val = a.as_int().expect("ge: first operand must be an integer");
    let b_val = b.as_int().expect("ge: second operand must be an integer");

    let result = a_val >= b_val;
    unsafe { push_bool(rest, result) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ne(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    let a_val = a.as_int().expect("ne: first operand must be an integer");
    let b_val = b.as_int().expect("ne: second operand must be an integer");

    let result = a_val != b_val;
    unsafe { push_bool(rest, result) }
}

/// Deep copy a cell (for variant field extraction)
///
/// # Safety
/// Cell pointer must be valid
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_cell(cell: *mut StackCell) -> *mut StackCell {
    assert!(!cell.is_null(), "copy_cell: null cell");

    unsafe {
        let original = &*cell;
        let cloned = StackCell::deep_clone(original);
        Box::into_raw(Box::new(cloned))
    }
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
            assert_eq!(cell.data.int_val, 42);
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
            assert_eq!(top.data.int_val, 42);
            assert_eq!(second.data.int_val, 42);
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
            assert_eq!(top.data.int_val, 1);
            assert_eq!(second.data.int_val, 2);
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
            assert_eq!(result.data.int_val, 42);
        }
    }

    #[test]
    fn test_dup_drop_no_double_free() {
        use std::ffi::CString;

        unsafe {
            // Test with string (heap-allocated)
            let stack = ptr::null_mut();
            let test_str = CString::new("test").unwrap();
            let stack = push_string(stack, test_str.as_ptr());

            // Duplicate the string
            let stack = dup(stack);

            // Now we have two copies - drop both should not double-free
            let stack = drop(stack); // Drop the duplicate
            let stack = drop(stack); // Drop the original

            assert!(stack.is_null());
        }
    }

    #[test]
    fn test_variant_dup_drop() {
        unsafe {
            // Test with variant containing heap data
            let field = push_int(ptr::null_mut(), 42);
            let stack = crate::pattern::push_variant(ptr::null_mut(), 0, field);

            // Duplicate the variant
            let stack = dup(stack);

            // Both copies should be independently droppable
            let stack = drop(stack); // Drop duplicate
            let stack = drop(stack); // Drop original

            assert!(stack.is_null());
        }
    }

    #[test]
    fn test_rot() {
        unsafe {
            // Test rot: ( A B C -- B C A )
            let stack = ptr::null_mut();
            let stack = push_int(stack, 1); // A
            let stack = push_int(stack, 2); // B
            let stack = push_int(stack, 3); // C
            let stack = rot(stack);

            let (rest, a) = StackCell::pop(stack); // Should be 1
            let (rest, c) = StackCell::pop(rest); // Should be 3
            let (rest, b) = StackCell::pop(rest); // Should be 2

            assert!(rest.is_null());
            assert_eq!(a.as_int().unwrap(), 1);
            assert_eq!(c.as_int().unwrap(), 3);
            assert_eq!(b.as_int().unwrap(), 2);
        }
    }

    #[test]
    fn test_nip() {
        unsafe {
            // Test nip: ( A B -- B )
            let stack = ptr::null_mut();
            let stack = push_int(stack, 1); // A
            let stack = push_int(stack, 2); // B
            let stack = nip(stack);

            let (rest, b) = StackCell::pop(stack); // Should be 2

            assert!(rest.is_null());
            assert_eq!(b.as_int().unwrap(), 2);
        }
    }

    #[test]
    fn test_tuck() {
        unsafe {
            // Test tuck: ( A B -- B A B )
            let stack = ptr::null_mut();
            let stack = push_int(stack, 1); // A
            let stack = push_int(stack, 2); // B
            let stack = tuck(stack);

            let (rest, b2) = StackCell::pop(stack); // Should be 2
            let (rest, a) = StackCell::pop(rest); // Should be 1
            let (rest, b1) = StackCell::pop(rest); // Should be 2

            assert!(rest.is_null());
            assert_eq!(b2.as_int().unwrap(), 2);
            assert_eq!(a.as_int().unwrap(), 1);
            assert_eq!(b1.as_int().unwrap(), 2);
        }
    }

    #[test]
    fn test_subtract() {
        unsafe {
            let stack = ptr::null_mut();
            let stack = push_int(stack, 10);
            let stack = push_int(stack, 3);
            let stack = subtract(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert_eq!(result.as_int().unwrap(), 7);
        }
    }

    #[test]
    fn test_divide() {
        unsafe {
            let stack = ptr::null_mut();
            let stack = push_int(stack, 20);
            let stack = push_int(stack, 4);
            let stack = divide(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert_eq!(result.as_int().unwrap(), 5);
        }
    }

    #[test]
    fn test_comparison_eq() {
        unsafe {
            // Test equal
            let stack = ptr::null_mut();
            let stack = push_int(stack, 5);
            let stack = push_int(stack, 5);
            let stack = eq(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(result.as_bool().unwrap());

            // Test not equal
            let stack = ptr::null_mut();
            let stack = push_int(stack, 5);
            let stack = push_int(stack, 3);
            let stack = eq(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(!result.as_bool().unwrap());
        }
    }

    #[test]
    fn test_comparison_lt() {
        unsafe {
            // Test less than (true)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 3);
            let stack = push_int(stack, 5);
            let stack = lt(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(result.as_bool().unwrap());

            // Test less than (false)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 5);
            let stack = push_int(stack, 3);
            let stack = lt(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(!result.as_bool().unwrap());
        }
    }

    #[test]
    fn test_comparison_gt() {
        unsafe {
            // Test greater than (true)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 5);
            let stack = push_int(stack, 3);
            let stack = gt(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(result.as_bool().unwrap());

            // Test greater than (false)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 3);
            let stack = push_int(stack, 5);
            let stack = gt(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(!result.as_bool().unwrap());
        }
    }

    #[test]
    fn test_comparison_le() {
        unsafe {
            // Test <= (true, equal)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 5);
            let stack = push_int(stack, 5);
            let stack = le(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(result.as_bool().unwrap());

            // Test <= (true, less)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 3);
            let stack = push_int(stack, 5);
            let stack = le(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(result.as_bool().unwrap());

            // Test <= (false)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 6);
            let stack = push_int(stack, 5);
            let stack = le(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(!result.as_bool().unwrap());
        }
    }

    #[test]
    fn test_comparison_ge() {
        unsafe {
            // Test >= (true, equal)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 5);
            let stack = push_int(stack, 5);
            let stack = ge(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(result.as_bool().unwrap());

            // Test >= (true, greater)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 6);
            let stack = push_int(stack, 5);
            let stack = ge(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(result.as_bool().unwrap());

            // Test >= (false)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 3);
            let stack = push_int(stack, 5);
            let stack = ge(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(!result.as_bool().unwrap());
        }
    }

    #[test]
    fn test_comparison_ne() {
        unsafe {
            // Test not equal (true)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 5);
            let stack = push_int(stack, 3);
            let stack = ne(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(result.as_bool().unwrap());

            // Test not equal (false)
            let stack = ptr::null_mut();
            let stack = push_int(stack, 5);
            let stack = push_int(stack, 5);
            let stack = ne(stack);

            let (rest, result) = StackCell::pop(stack);
            assert!(rest.is_null());
            assert!(!result.as_bool().unwrap());
        }
    }
}

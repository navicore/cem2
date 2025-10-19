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

impl StackCell {
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
/// Caller must ensure both stack and string pointers are valid. String must be null-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn push_string(stack: *mut StackCell, s: *const i8) -> *mut StackCell {
    assert!(!s.is_null(), "push_string: null string pointer");

    // Copy the C string to owned Rust String, then back to C string
    let rust_string = unsafe { std::ffi::CStr::from_ptr(s).to_string_lossy().into_owned() };

    let c_string = std::ffi::CString::new(rust_string).unwrap();
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dup(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "dup: stack is empty");

    unsafe {
        let top = &*stack;
        let duplicated = Box::new(match top.cell_type {
            CellType::Int => StackCell {
                cell_type: CellType::Int,
                _padding: 0,
                data: CellDataUnion {
                    int_val: top.data.int_val,
                },
                next: ptr::null_mut(),
            },
            CellType::Bool => StackCell {
                cell_type: CellType::Bool,
                _padding: 0,
                data: CellDataUnion {
                    bool_val: top.data.bool_val,
                },
                next: ptr::null_mut(),
            },
            CellType::String => {
                // Clone the string
                let original_ptr = top.data.string_ptr;
                let rust_str = std::ffi::CStr::from_ptr(original_ptr)
                    .to_string_lossy()
                    .into_owned();
                let new_c_str = std::ffi::CString::new(rust_str).unwrap();
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
                // For variants, we need to deep-clone the data
                let variant = top.data.variant;
                let cloned_data = if variant.data.is_null() {
                    ptr::null_mut()
                } else {
                    // Clone the field cell
                    let field = &*variant.data;
                    Box::into_raw(Box::new(StackCell {
                        cell_type: field.cell_type,
                        _padding: 0,
                        data: field.data,
                        next: ptr::null_mut(),
                    }))
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

    unsafe {
        let (rest, cell) = StackCell::pop(stack);

        // Free owned resources
        match cell.cell_type {
            CellType::String => {
                if !cell.data.string_ptr.is_null() {
                    let _ = std::ffi::CString::from_raw(cell.data.string_ptr);
                }
            }
            CellType::Variant => {
                // Free variant data if present
                if !cell.data.variant.data.is_null() {
                    let _ = Box::from_raw(cell.data.variant.data);
                }
            }
            _ => {}
        }

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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn over(stack: *mut StackCell) -> *mut StackCell {
    assert!(!stack.is_null(), "over: stack too small");

    unsafe {
        let top = &*stack;
        assert!(!top.next.is_null(), "over: stack too small");
        let second = &*top.next;

        let duplicated = Box::new(match second.cell_type {
            CellType::Int => StackCell {
                cell_type: CellType::Int,
                _padding: 0,
                data: CellDataUnion {
                    int_val: second.data.int_val,
                },
                next: ptr::null_mut(),
            },
            CellType::Bool => StackCell {
                cell_type: CellType::Bool,
                _padding: 0,
                data: CellDataUnion {
                    bool_val: second.data.bool_val,
                },
                next: ptr::null_mut(),
            },
            CellType::String => {
                let original_ptr = second.data.string_ptr;
                let rust_str = std::ffi::CStr::from_ptr(original_ptr)
                    .to_string_lossy()
                    .into_owned();
                let new_c_str = std::ffi::CString::new(rust_str).unwrap();
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
                let variant = second.data.variant;
                let cloned_data = if variant.data.is_null() {
                    ptr::null_mut()
                } else {
                    let field = &*variant.data;
                    Box::into_raw(Box::new(StackCell {
                        cell_type: field.cell_type,
                        _padding: 0,
                        data: field.data,
                        next: ptr::null_mut(),
                    }))
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

    assert_eq!(a.cell_type, CellType::Int, "add: type error");
    assert_eq!(b.cell_type, CellType::Int, "add: type error");

    let result = unsafe { a.data.int_val + b.data.int_val };
    unsafe { push_int(rest, result) }
}

/// # Safety
/// Stack must have 2 integers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply(stack: *mut StackCell) -> *mut StackCell {
    let (rest, b) = unsafe { StackCell::pop(stack) };
    let (rest, a) = unsafe { StackCell::pop(rest) };

    assert_eq!(a.cell_type, CellType::Int, "multiply: type error");
    assert_eq!(b.cell_type, CellType::Int, "multiply: type error");

    let result = unsafe { a.data.int_val * b.data.int_val };
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
}

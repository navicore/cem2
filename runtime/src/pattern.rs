/*!
Pattern Matching Runtime Support - C-compatible variant operations
*/

use crate::stack::{CellDataUnion, CellType, StackCell, VariantData};

/// Maximum allowed variant tag value
///
/// This limit serves as a sanity check to catch obviously invalid variant tags
/// from malicious or buggy codegen. A well-formed Cem program is unlikely to
/// define more than 1000 variants in a single type.
///
/// If you hit this limit legitimately (e.g., code-generated enums), the limit
/// can be increased or removed. The limit exists primarily to detect corruption:
/// if a type tag is accidentally set to a garbage value (e.g., 0xFFFFFFFF),
/// we want to fail fast rather than allocate invalid memory or exhibit UB.
///
/// For most programs, the number of variants per type will be < 100.
const MAX_VARIANT_TAG: u32 = 1000;

/// Push a variant onto the stack
///
/// # Safety
/// - `stack` must be a valid StackCell pointer or null
/// - `tag` must be a valid variant tag (< MAX_VARIANT_TAG)
/// - `field_data` must be either:
///   - null (for 0-field variants like None)
///   - a valid StackCell pointer (for 1-field variants like Some)
///
/// The variant takes ownership of `field_data` if provided.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn push_variant(
    stack: *mut StackCell,
    tag: u32,
    field_data: *mut StackCell,
) -> *mut StackCell {
    assert!(
        tag < MAX_VARIANT_TAG,
        "push_variant: invalid variant tag {} (max {})",
        tag,
        MAX_VARIANT_TAG
    );

    let cell = Box::new(StackCell {
        cell_type: CellType::Variant,
        _padding: 0,
        data: CellDataUnion {
            variant: VariantData {
                tag,
                _padding: 0,
                data: field_data, // null for 0-field, pointer for 1-field
            },
        },
        next: std::ptr::null_mut(),
    });

    unsafe { StackCell::push(stack, cell) }
}

/// Allocate a new empty StackCell
///
/// # Safety
/// Always safe - allocates a new cell with uninitialized data.
/// The caller is responsible for initializing the cell before use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alloc_cell() -> *mut StackCell {
    let cell = Box::new(StackCell {
        cell_type: CellType::Int, // Placeholder type
        _padding: 0,
        data: CellDataUnion { int_val: 0 }, // Placeholder data
        next: std::ptr::null_mut(),
    });

    Box::into_raw(cell)
}

/// Get variant tag
///
/// # Safety
/// Variant pointer must be valid and of type Variant.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn variant_get_tag(variant: *mut StackCell) -> u32 {
    assert!(!variant.is_null(), "variant_get_tag: null variant");

    unsafe {
        let var = &*variant;
        var.as_variant()
            .expect("variant_get_tag: not a variant")
            .tag
    }
}

/// Get variant field data pointer
///
/// # Safety
/// Variant pointer must be valid and of type Variant.
///
/// # Returns
/// Returns the raw data pointer which **may be null** for 0-field variants (e.g., `None`).
/// **IMPORTANT**: Callers MUST null-check the result before dereferencing:
/// ```c
/// StackCell* data = variant_get_data(variant);
/// if (data != NULL) {
///     // Safe to use data
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn variant_get_data(variant: *mut StackCell) -> *mut StackCell {
    assert!(!variant.is_null(), "variant_get_data: null variant");

    unsafe {
        let var = &*variant;
        var.as_variant()
            .expect("variant_get_data: not a variant")
            .data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::push_int;

    #[test]
    fn test_variant_creation() {
        unsafe {
            // Test 1-field variant (Some(42))
            let field = push_int(std::ptr::null_mut(), 42);
            let variant = push_variant(std::ptr::null_mut(), 0, field);

            let tag = variant_get_tag(variant);
            assert_eq!(tag, 0);

            let data = variant_get_data(variant);
            assert!(!data.is_null());

            let field_cell = &*data;
            let val = field_cell.as_int().expect("should be int");
            assert_eq!(val, 42);

            // Clean up
            crate::scheduler::free_stack(variant);
        }
    }

    #[test]
    fn test_variant_none() {
        unsafe {
            // Test 0-field variant (None)
            let variant = push_variant(std::ptr::null_mut(), 1, std::ptr::null_mut());

            let tag = variant_get_tag(variant);
            assert_eq!(tag, 1);

            let data = variant_get_data(variant);
            assert!(data.is_null());

            // Clean up
            crate::scheduler::free_stack(variant);
        }
    }

    #[test]
    fn test_nested_variants() {
        unsafe {
            // Test nested variants: Some(Some(42))
            // Inner variant: Some(42)
            let inner_field = push_int(std::ptr::null_mut(), 42);
            let inner_variant = push_variant(std::ptr::null_mut(), 0, inner_field);

            // Outer variant: Some(inner_variant)
            let outer_variant = push_variant(std::ptr::null_mut(), 0, inner_variant);

            // Verify outer variant
            let outer_tag = variant_get_tag(outer_variant);
            assert_eq!(outer_tag, 0);

            let outer_data = variant_get_data(outer_variant);
            assert!(!outer_data.is_null());

            // Verify inner variant
            let inner = &*outer_data;
            let inner_variant_data = inner.as_variant().expect("should be variant");
            assert_eq!(inner_variant_data.tag, 0);

            // Verify innermost value
            let innermost_cell = &*inner_variant_data.data;
            let innermost_val = innermost_cell.as_int().expect("should be int");
            assert_eq!(innermost_val, 42);

            // Clean up (should recursively free all nested data)
            crate::scheduler::free_stack(outer_variant);
        }
    }

    #[test]
    fn test_variant_with_max_tag() {
        unsafe {
            // Test variant with tag near MAX_VARIANT_TAG
            let variant = push_variant(
                std::ptr::null_mut(),
                MAX_VARIANT_TAG - 1,
                std::ptr::null_mut(),
            );

            let tag = variant_get_tag(variant);
            assert_eq!(tag, MAX_VARIANT_TAG - 1);

            // Clean up
            crate::scheduler::free_stack(variant);
        }
    }

    // Note: We cannot test tag >= MAX_VARIANT_TAG with #[should_panic]
    // because push_variant is extern "C" and cannot unwind.
    // The assertion exists to catch bugs in codegen, not for runtime testing.

    #[test]
    fn test_multiple_variant_types() {
        unsafe {
            // Test multiple different variant tags (simulating different enum cases)
            for tag in 0..10 {
                let variant = push_variant(std::ptr::null_mut(), tag, std::ptr::null_mut());
                let retrieved_tag = variant_get_tag(variant);
                assert_eq!(retrieved_tag, tag);
                crate::scheduler::free_stack(variant);
            }
        }
    }

    #[test]
    fn test_variant_with_string_field() {
        use std::ffi::CString;
        unsafe {
            // Test variant containing a string
            let test_str = CString::new("hello").unwrap();
            let str_cell = crate::stack::push_string(std::ptr::null_mut(), test_str.as_ptr());
            let variant = push_variant(std::ptr::null_mut(), 0, str_cell);

            let tag = variant_get_tag(variant);
            assert_eq!(tag, 0);

            let data = variant_get_data(variant);
            assert!(!data.is_null());

            let field_cell = &*data;
            assert_eq!(field_cell.cell_type, CellType::String);

            // Clean up (should free the string too)
            crate::scheduler::free_stack(variant);
        }
    }
}

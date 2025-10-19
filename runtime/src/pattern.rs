/*!
Pattern Matching Runtime Support - C-compatible variant operations
*/

use crate::stack::{CellDataUnion, CellType, StackCell, VariantData};

/// Push a variant onto the stack
///
/// # Safety
/// - `stack` must be a valid StackCell pointer or null
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
        assert_eq!(
            var.cell_type,
            CellType::Variant,
            "variant_get_tag: not a variant"
        );
        var.data.variant.tag
    }
}

/// Get variant field data pointer
///
/// # Safety
/// Variant pointer must be valid and of type Variant.
/// Returns the raw data pointer (may be null for 0-field variants).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn variant_get_data(variant: *mut StackCell) -> *mut StackCell {
    assert!(!variant.is_null(), "variant_get_data: null variant");

    unsafe {
        let var = &*variant;
        assert_eq!(
            var.cell_type,
            CellType::Variant,
            "variant_get_data: not a variant"
        );
        var.data.variant.data
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
            assert_eq!(field_cell.cell_type, CellType::Int);
            assert_eq!(field_cell.data.int_val, 42);

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
}

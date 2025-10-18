/*!
Pattern Matching Runtime Support

This is the part that caused segfaults in your C runtime!
With Rust, variant allocation and matching is memory-safe.
*/

use crate::stack::{CellData, CellType, StackCell};

/// Allocate a variant (sum type constructor)
///
/// Example: For Option<T>, this creates Some(value) or None
///
/// LLVM IR: %variant = call ptr @alloc_variant(i32 %tag, i64 %field_count)
#[no_mangle]
pub extern "C" fn alloc_variant(tag: u32, field_count: usize) -> *mut StackCell {
    let cell = Box::new(StackCell {
        cell_type: CellType::Variant,
        data: CellData::Variant {
            tag,
            fields: Vec::with_capacity(field_count),
        },
        next: None,
    });

    Box::into_raw(cell)
}

/// Add a field to a variant
/// This is called during variant construction
///
/// LLVM IR: %variant = call ptr @variant_push_field(ptr %variant, ptr %field)
#[no_mangle]
pub extern "C" fn variant_push_field(
    variant: *mut StackCell,
    field: *mut StackCell,
) -> *mut StackCell {
    assert!(!variant.is_null(), "variant_push_field: null variant");
    assert!(!field.is_null(), "variant_push_field: null field");

    unsafe {
        let mut var = Box::from_raw(variant);

        match &mut var.data {
            CellData::Variant { fields, .. } => {
                fields.push(Box::from_raw(field));
            }
            _ => panic!("variant_push_field: not a variant"),
        }

        Box::into_raw(var)
    }
}

/// Match on a variant and extract tag
/// Returns (rest_of_stack, tag, fields_as_stack)
///
/// LLVM IR:
///   %tag = call i32 @variant_get_tag(ptr %variant)
///   %fields = call ptr @variant_get_fields(ptr %variant)
#[no_mangle]
pub extern "C" fn variant_get_tag(variant: *mut StackCell) -> u32 {
    assert!(!variant.is_null(), "variant_get_tag: null variant");

    unsafe {
        let var = &*variant;
        match &var.data {
            CellData::Variant { tag, .. } => *tag,
            _ => panic!("variant_get_tag: not a variant"),
        }
    }
}

/// Get the fields of a variant as a new stack
/// The variant itself is consumed (moved)
#[no_mangle]
pub extern "C" fn variant_get_fields(variant: *mut StackCell) -> *mut StackCell {
    assert!(!variant.is_null(), "variant_get_fields: null variant");

    unsafe {
        let var = Box::from_raw(variant);

        match var.data {
            CellData::Variant { fields, .. } => {
                // Build a stack from the fields (in reverse order)
                let mut stack = std::ptr::null_mut();
                for field in fields.into_iter().rev() {
                    stack = StackCell::push(stack, field);
                }
                stack
            }
            _ => panic!("variant_get_fields: not a variant"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::push_int;

    #[test]
    fn test_variant_creation() {
        // Create Some(42)
        let variant = alloc_variant(0, 1); // tag=0 for Some
        let field = push_int(std::ptr::null_mut(), 42);
        let variant = variant_push_field(variant, field);

        let tag = variant_get_tag(variant);
        assert_eq!(tag, 0);

        let fields = variant_get_fields(variant);
        let (rest, field) = StackCell::pop(fields);
        assert!(rest.is_null());

        match field.data {
            CellData::Int(n) => assert_eq!(n, 42),
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn test_variant_none() {
        // Create None (no fields)
        let variant = alloc_variant(1, 0); // tag=1 for None

        let tag = variant_get_tag(variant);
        assert_eq!(tag, 1);

        let fields = variant_get_fields(variant);
        assert!(fields.is_null()); // No fields
    }
}

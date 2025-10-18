/*!
Pattern Matching Runtime Support - Edition 2024 compliant
*/

use crate::stack::{CellData, CellType, StackCell};

/// # Safety
/// Always safe - allocates new variant.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alloc_variant(tag: u32, field_count: usize) -> *mut StackCell {
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

/// # Safety
/// Both pointers must be valid StackCell pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn variant_push_field(
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

/// # Safety
/// Variant pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn variant_get_tag(variant: *mut StackCell) -> u32 {
    assert!(!variant.is_null(), "variant_get_tag: null variant");

    unsafe {
        let var = &*variant;
        match &var.data {
            CellData::Variant { tag, .. } => *tag,
            _ => panic!("variant_get_tag: not a variant"),
        }
    }
}

/// # Safety
/// Consumes the variant, returns fields as stack.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn variant_get_fields(variant: *mut StackCell) -> *mut StackCell {
    assert!(!variant.is_null(), "variant_get_fields: null variant");

    unsafe {
        let var = Box::from_raw(variant);

        match var.data {
            CellData::Variant { fields, .. } => {
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
        unsafe {
            let variant = alloc_variant(0, 1);
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
    }

    #[test]
    fn test_variant_none() {
        unsafe {
            let variant = alloc_variant(1, 0);

            let tag = variant_get_tag(variant);
            assert_eq!(tag, 1);

            let fields = variant_get_fields(variant);
            assert!(fields.is_null());
        }
    }
}

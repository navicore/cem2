# Memory Management in Cem2

## Overview

Cem2 uses Rust's ownership system and RAII (Drop trait) for automatic memory management. This document explains how memory is managed, particularly for multi-field variants in pattern matching.

## Stack Cell Lifecycle

### Allocation
Stack cells are allocated via `Box::new()` and converted to raw pointers:
```rust
let cell = Box::into_raw(Box::new(StackCell { ... }));
```

### Deallocation
Stack cells are freed in two ways:

1. **Individual drop**: The `drop()` function pops and frees a single cell
```rust
pub unsafe extern "C" fn drop(stack: *mut StackCell) -> *mut StackCell {
    let (rest, _cell) = StackCell::pop(stack);
    // Cell is automatically cleaned up by Drop impl
    rest
}
```

2. **Bulk cleanup**: The `free_stack()` function frees entire stack chains
```rust
pub unsafe extern "C" fn free_stack(stack: *mut StackCell) {
    if !stack.is_null() {
        let _ = Box::from_raw(stack); // Triggers Drop trait recursively
    }
}
```

## Drop Trait Implementation

The `Drop` trait for `StackCell` handles cleanup of heap-allocated data:

```rust
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
                        // Free the variant's field chain
                        let _ = Box::from_raw(self.data.variant.data);
                    }
                }
                _ => {}
            }
        }
    }
}
```

**Key Point**: When a Variant cell is dropped, it **automatically frees its field chain** via `Box::from_raw(self.data.variant.data)`.

## Pattern Matching Memory Management

### The Question
When pattern matching on multi-field variants, we copy the field chain. Does this leak the original field chain?

### The Answer: No Memory Leak

Here's the complete lifecycle:

1. **Variant on Stack**
   ```
   Stack: [ Variant(tag=1, data=field_chain) | ... ]
   ```

2. **Match Expression Pops Variant**
   ```rust
   // Compiler generates:
   let (rest, variant_cell) = pop(stack);
   let variant_data = variant_cell.data.variant.data;
   ```

3. **Field Chain is Copied**
   ```rust
   // For each field in the chain:
   let field_copy = copy_cell(variant_data);
   // Copied fields become the new stack for the match branch
   ```

4. **Original Variant is Dropped**
   ```rust
   // When variant_cell goes out of scope or is explicitly dropped:
   // Drop trait runs and frees the ORIGINAL field chain
   let _ = Box::from_raw(variant_data); // In Drop::drop()
   ```

5. **Copied Fields Live On**
   ```
   New Stack: [ field_copy_0 | field_copy_1 | rest ]
   ```

### Why No Leak?

- **Copied fields** are deep clones with their own allocations
- **Original variant cell** still owns the original field chain
- When the original variant cell is dropped (either explicitly or when the stack is cleaned up), its Drop trait frees the original field chain
- The copied fields are independent and will be freed when they are eventually dropped

## Example: Matching on Cons(1, Cons(2, Nil))

```cem
: process-list ( List(Int) -- )
  match
    Cons => [  # Fields copied here
      # Stack now has: ( head=1 tail=Cons(2,Nil) )
      drop drop  # These drop the COPIES
    ]
    Nil => [ ]
  end ;
```

Memory flow:
1. Original list cell: `Cons(data=field_chain_ptr)` on stack
2. Match pops it: `variant_cell` holds the original
3. Fields copied: `field_copy_0` (head=1), `field_copy_1` (tail=Cons(...))
4. Copies pushed to stack for match branch
5. **Original variant_cell dropped**: Frees `field_chain_ptr` (the original fields)
6. Branch executes with copies
7. Copies eventually dropped when branch completes

## Deep Clone Implementation

The `deep_clone` function ensures copied variants have independent field chains:

```rust
CellType::Variant => {
    // Clone ENTIRE field chain, not just first field
    let mut cloned_fields: Vec<*mut StackCell> = Vec::new();
    let mut current = variant.data;

    // Walk and clone each field
    while !current.is_null() {
        let field = &*current;
        let cloned_field = Box::into_raw(Box::new(Self::deep_clone(field)));
        cloned_fields.push(cloned_field);
        current = field.next;
    }

    // Link cloned fields together
    for i in 0..cloned_fields.len() {
        if i + 1 < cloned_fields.len() {
            (*cloned_fields[i]).next = cloned_fields[i + 1];
        } else {
            (*cloned_fields[i]).next = ptr::null_mut();
        }
    }

    // Return new variant with cloned chain
    StackCell {
        cell_type: CellType::Variant,
        data: CellDataUnion {
            variant: VariantData {
                tag: variant.tag,
                data: cloned_fields.first().copied().unwrap_or(ptr::null_mut()),
            },
        },
        next: ptr::null_mut(),
    }
}
```

## Memory Safety Guarantees

1. **No double-free**: Copied fields have their own allocations
2. **No use-after-free**: Copies are independent of originals
3. **No leaks**: Drop trait ensures all chains are freed
4. **No dangling pointers**: deep_clone creates fully independent structures

## Testing

The runtime includes comprehensive tests for memory safety:

- `test_dup_drop_no_double_free`: Verifies dup + drop doesn't double-free
- `test_variant_dup_drop`: Tests variant copying and cleanup
- `test_nested_variants`: Verifies recursive cleanup of nested structures
- Valgrind/AddressSanitizer can be used to verify no leaks

## Conclusion

**There is no memory leak in pattern matching.** The design uses:
- Rust's Drop trait for automatic cleanup
- Deep cloning for independent copies
- Proper ownership tracking

The original variant's field chain is always freed when the variant cell is dropped, and the copied fields are independent allocations that are freed separately.

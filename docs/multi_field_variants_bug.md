# Multi-Field Variant Implementation - Bug Report

## Status: IN PROGRESS

Multi-field variant support has been partially implemented but has a field counting bug that needs to be resolved.

## Background

### Motivation
List operations require `List<T>` with constructor `Cons(T, List(T))` which has 2 fields. The original implementation only supported 0-field (e.g., `None`) and 1-field (e.g., `Some(T)`) variants.

### Design Decision
**Approach**: Chain fields as a linked list

For `Cons(head, tail)` with stack `[tail, head]`:
1. Pop each field and allocate a StackCell for it
2. Link them: `field[0].next = field[1]`, `field[1].next = null`
3. Create variant: `Variant { tag: Cons, data: &field[0] }`

**Pattern matching** unwraps by:
1. Extract data pointer (points to field[0])
2. Walk chain to find last field
3. Link last field to rest of stack
4. Stack for match body: `field[0] field[1] rest...`

This design means fields appear on the stack in declaration order, matching how single-field variants work.

## Implementation Completed

### Runtime
- ✅ No changes needed! Existing `push_variant()` and variant accessors work
- ✅ Added `exit_op()` function (compiler/src/codegen/mod.rs:130, runtime/src/io.rs:84)

### Compiler - Construction (compiler/src/codegen/mod.rs:974-1062)
```rust
// Multi-field variant construction
_ => {
    let mut field_cells = Vec::new();
    let mut current_stack = stack.to_string();

    // Pop and allocate each field
    for _i in 0..field_count {
        let field_cell = alloc_cell();
        memcpy(field_cell, current_stack);
        field_cells.push(field_cell);
        current_stack = pop(current_stack);
    }

    // Link fields together
    for i in 0..field_count {
        if i + 1 < field_count {
            field[i].next = field[i+1];
        } else {
            field[i].next = null;
        }
    }

    // Create variant
    push_variant(current_stack, tag, field_cells[0]);
}
```

### Compiler - Pattern Matching (compiler/src/codegen/mod.rs:1284-1333)
```rust
// Multi-field variant pattern matching
else {
    // Walk chain to find last field
    let mut current_field = variant_data.clone();
    for i in 0..field_count - 1 {
        let next_field = current_field.next;
        if i == field_count - 2 {
            // Last field, link to rest
            next_field.next = rest_var;
        }
        current_field = next_field;
    }

    // Return first field as initial stack
    variant_data.clone()
}
```

## Current Bug

### Symptom
When compiling `Cons(42, Nil)`:
- Expected: Allocate 2 cells (for 2 fields)
- Actual: Allocating 3 cells

### Evidence (test_cons.ll)
```llvm
define ptr @cem_main(ptr %stack) {
  ; ... create Nil, push 42, swap ...

  ; Bug: allocating 3 cells instead of 2!
  %6 = call ptr @alloc_cell()   ; Cell 1
  %9 = call ptr @alloc_cell()   ; Cell 2
  %12 = call ptr @alloc_cell()  ; Cell 3 ← SHOULD NOT EXIST

  ; Linking 3 cells instead of 2
  store ptr %9, ptr %15   ; field[0].next = field[1]
  store ptr %12, ptr %16  ; field[1].next = field[2] ← WRONG
  store ptr null, ptr %17 ; field[2].next = null

  %18 = call ptr @push_variant(ptr %14, i32 0, ptr %6)
}
```

### Root Cause Hypothesis
The `field_count` variable is being set to 3 instead of 2.

**Possibilities:**
1. **Type definition parsing**: When parsing `type List(T) | Cons(T, List(T))`, we're counting 3 fields instead of 2
2. **Variant registration**: The `variant_field_counts` HashMap is being populated incorrectly
3. **Stack depth confusion**: We might be counting items on the stack instead of declared fields

### Where field_count comes from

**At construction** (compiler/src/codegen/mod.rs:907):
```rust
let field_count = self.variant_field_counts.get(name).copied().unwrap_or(0);
```

**At pattern matching** (compiler/src/codegen/mod.rs:1260):
```rust
let field_count = self.variant_field_counts.get(name).copied().unwrap_or(0);
```

**Population happens** in `register_type_defs()` (need to find this code):
- When the compiler processes type definitions
- Should store: `variant_field_counts["Cons"] = 2`

## Test Cases

### Minimal Reproduction (examples/test_cons.cem)
```cem
type List(T)
  | Cons(T, List(T))
  | Nil

: main ( -- )
  "Testing Cons construction..." write_line
  Nil
  42 swap Cons
  match
    Cons => [ "Got Cons!" write_line swap drop drop ]
    Nil  => [ "Got Nil" write_line ]
  end
  "Done!" write_line ;
```

**Result**: Stack overflow due to too many cells being allocated

### Full Test Suite (examples/test_list.cem)
Contains list operations from stdlib/prelude.cem - will work once bug is fixed.

## Debugging Steps

1. **Find variant registration code**
   - Search for where `variant_field_counts` is populated
   - Look for `register_type_defs` or similar
   - Check how we count fields in `Variant { fields: Vec<Type> }`

2. **Add debug output**
   - Print `field_count` during codegen for `Cons`
   - Print what's in `variant_field_counts` HashMap
   - Verify type definition parsing

3. **Check type representation**
   ```rust
   // In AST, how is Cons represented?
   Variant {
       name: "Cons",
       fields: vec![Type::Var("T"), Type::Named("List", ...)]  // Should be length 2!
   }
   ```

4. **Test hypothesis**
   - If `fields.len()` returns 2 but `field_count` is 3, the bug is in registration
   - If `fields.len()` returns 3, the bug is in parsing

## Files Modified

### Runtime
- `runtime/src/io.rs`: Added `exit_op()` function (lines 78-94)

### Compiler
- `compiler/src/codegen/mod.rs`:
  - Lines 974-1062: Multi-field variant construction
  - Lines 1284-1333: Multi-field variant pattern matching
  - Line 130: Map `exit` to `exit_op`
  - Line 279: Declare `exit_op` extern function

### Stdlib
- `stdlib/prelude.cem`: List type and operations (ready to use)
- `compiler/src/main.rs`: Lines 78-88: Auto-include prelude

### Tests
- `examples/test_cons.cem`: Minimal reproduction case
- `examples/test_list.cem`: Full list operations test

## Next Steps

1. **Locate and fix field counting bug**
   - Find where `variant_field_counts` is populated
   - Verify it correctly counts fields from type definition
   - Expected: `variant_field_counts["Cons"] = 2`

2. **Verify construction works**
   - Compile and run `test_cons.cem`
   - Should print: "Testing Cons construction... Got Cons! Done!"

3. **Verify pattern matching works**
   - Test that fields are unwrapped in correct order
   - Stack should have: `head tail rest...`

4. **Test list operations**
   - Run `test_list.cem`
   - Should execute all list operations without stack overflow

5. **Add comprehensive tests**
   - Test 0-field, 1-field, 2-field, 3+ field variants
   - Test nested variants
   - Test recursive variants (List)

## Success Criteria

- [ ] `test_cons.cem` compiles and runs successfully
- [ ] LLVM IR shows exactly 2 `alloc_cell()` calls for Cons construction
- [ ] `test_list.cem` runs all list operations
- [ ] CI passes with new multi-field variant tests

## Architecture Notes

This is a **fundamental language feature** that enables:
- List operations (Cons has 2 fields)
- Tree structures (Node has 3+ fields)
- General algebraic data types
- Pattern matching on complex data

We're taking time to do this right because:
1. It's core to the language design
2. Changing it later would require revisiting memory layout
3. It affects both runtime (memory) and compiler (codegen)
4. Better to find architectural issues now than after users depend on it

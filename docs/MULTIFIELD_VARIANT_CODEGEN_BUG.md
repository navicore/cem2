# Multi-Field Variant Codegen Bug - Systematic Debugging Plan

## Status: ACTIVE INVESTIGATION

## The Bug

Multi-field variants (e.g., `Cons(T, List(T))`) constructed with function return values create malformed variants that fail pattern matching.

### Reproduction

```cem
: double ( Int -- Int )
  2 * ;

: main ( -- )
  5 double      # Returns 10 on stack
  Nil swap Cons # Create Cons(10, Nil)

  list-reverse  # FAILS: "match: non-exhaustive pattern"
  drop ;
```

### What Works vs. What Fails

| Test Case | Result | Notes |
|-----------|--------|-------|
| `Some(10)` (literal) | ✅ Works | Single-field, literal value |
| `Some(double(5))` (function) | ✅ Works | Single-field, function result |
| `Cons(10, Nil)` (literal) | ✅ Works | Multi-field, literal values |
| `Cons(double(5), Nil)` (function) | ❌ FAILS | Multi-field, function result |

**Hypothesis:** The memcpy-based construction of multi-field variants corrupts something when the source StackCell comes from a function call.

## What We Know

1. **Parsing is correct** - `variant.fields.len()` returns 2 for Cons
2. **Registration is correct** - `variant_field_counts["Cons"] = 2`
3. **Single-field variants work** - Even with function results
4. **Multi-field literals work** - `Cons(10, Nil)` where 10 is pushed directly
5. **The bug is in codegen** - Specifically in how multi-field variant construction handles the stack

## Codegen Flow for Multi-Field Variants

Location: `compiler/src/codegen/mod.rs:1031-1124`

```rust
// For Cons(head, tail) with stack = [tail, head]
for i in 0..field_count {
    // 1. Allocate new cell
    field_cell[i] = alloc_cell()

    // 2. Copy entire StackCell (32 bytes) including 'next' pointer!
    llvm.memcpy(field_cell[i], current_stack, 32, align 8)

    // 3. Pop to next stack element
    current_stack = current_stack->next

    field_cells.push(field_cell[i])
}

// 4. Link field cells together
for i in 0..field_count {
    if i + 1 < field_count:
        field[i].next = field[i+1]  // Overwrites copied 'next'
    else:
        field[i].next = null
}

// 5. Create variant with first field as data
push_variant(current_stack, tag, field_cells[0])
```

## Debugging Hypothesis

### Theory 1: Stack Cell Aliasing
When we `memcpy` from `current_stack`, we copy the `next` pointer. If this `next` pointer points to something that gets freed or reused, we have dangling pointers.

**Check:**
- Does function return leave temporary cells on the stack?
- Are those cells being freed before we link fields?
- Is the `next` pointer in the copied cell pointing to freed memory?

### Theory 2: Deep Clone vs. Shallow Copy
`memcpy` does a shallow copy of the union. For variants or strings, the `data` pointer is copied but the pointed-to memory is not duplicated.

**Check:**
- Does the StackCell from a function return have different ownership semantics?
- Should we be using `deep_clone` instead of `memcpy`?

### Theory 3: Stack Pointer Corruption
The `current_stack` pointer might not be advancing correctly, causing us to copy from the wrong location.

**Check:**
- Is `getelementptr` generating the correct offset?
- Are we correctly loading the `next` pointer?

## Systematic Debugging Steps

### Step 1: Add Debug Output to Generated LLVM IR
**Goal:** See what's being copied and linked

**Action:** Modify codegen to emit debug prints showing:
- Address of current_stack before memcpy
- Address of allocated field_cell
- Value of next pointer before and after memcpy
- Values after linking

### Step 2: Compare Working vs. Broken Cases
**Goal:** See the difference in generated IR

**Cases to compare:**
1. `Cons(10, Nil)` - WORKS
2. `Cons(double(5), Nil)` - BROKEN

**Look for:**
- Differences in how the stack is set up before Cons construction
- Differences in the values being copied

### Step 3: Examine Runtime Memory State
**Goal:** Understand what the malformed variant looks like

**Action:** Add runtime assertions in variant construction:
- Check that field cells are valid
- Check that linked list is correct
- Dump field cell contents

### Step 4: Test with Simpler Cases
**Goal:** Narrow down the exact failure mode

**Test cases:**
```cem
# Test 1: Function that returns literal
: id ( Int -- Int ) ;
Cons(id(10), Nil)  # Does identity function break it?

# Test 2: Intermediate variable
: main ( -- )
  5 double
  dup         # Duplicate the result
  drop        # Keep one copy
  Nil swap Cons ;

# Test 3: Two function calls
Cons(double(5), Cons(double(10), Nil))
```

### Step 5: Check StackCell Layout
**Goal:** Verify our assumptions about memory layout

**Action:**
```rust
// In runtime/src/stack.rs, print actual sizes:
println!("StackCell size: {}", std::mem::size_of::<StackCell>());
println!("Offset of next: {}", offset_of!(StackCell, next));
```

Verify that LLVM's `getelementptr ... i32 0, i32 3` matches the actual offset.

## Test Files Created

Location: `/Users/navicore/git/navicore/cem2/tmp/`

- `test-cons-from-call.cem` - Minimal failing case
- `test-cons-without-call.cem` - Even direct calls fail
- `test-option-from-call.cem` - Single-field works
- `minimal-bug.cem` - Absolute minimal reproduction

## Next Actions

1. **Generate LLVM IR with symbols preserved** - Need to see the actual IR
2. **Add debug prints to runtime** - See what's being constructed
3. **Compare IR for working vs broken** - Find the difference
4. **Hypothesis**: The `next` pointer from function return points to freed memory
5. **Fix**: Either use `deep_clone` instead of `memcpy`, or ensure source cells are stable

## Success Criteria

- [ ] `Cons(double(5), Nil)` creates valid variant
- [ ] Can match on the resulting Cons
- [ ] `list-map` works with single-element list
- [ ] `list-map` works with multi-element list
- [ ] Deep recursion doesn't stack overflow

## Risks

**Time Risk:** This could take hours if the bug is subtle
**Complexity Risk:** Might require redesigning variant construction
**Stability Risk:** Fix might break existing working cases

## Fallback Plan

If we can't fix this in reasonable time:
1. Document the limitation clearly
2. Consider alternative designs (no memcpy, always deep_clone)
3. Potentially rethink multi-field variant representation

---

## Investigation Log

### 2025-10-19 - Initial Discovery
- Found that multi-field variants with function results fail pattern matching
- Single-field variants work fine
- Literals work fine
- Issue is specifically in codegen for multi-field construction with dynamic values

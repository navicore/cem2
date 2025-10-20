# Multi-Field Variant Codegen Bug - Systematic Debugging Plan

## Status: ACTIVE INVESTIGATION - ROOT CAUSE ISOLATED

## The Bug

**CORRECTED UNDERSTANDING:** Multi-field variants (e.g., `Cons(T, List(T))`) constructed **inside match branches** with a **specific stack shuffling pattern** create malformed variants that fail pattern matching when passed to recursive functions.

### Minimal Reproduction

```cem
: test-exact-shuffle ( List(Int) List(Int) -- List(Int) )
  swap
  match
    Cons => [
      # Stack: ( acc head tail )
      # Do the exact shuffling from list-reverse-helper
      rot swap              # ( head acc tail )
      rot                   # ( acc tail head )
      rot                   # ( tail head acc )
      swap Cons             # ( tail Cons(head, acc) )
      swap drop  # Drop tail, keep the Cons
    ]
    Nil => [ ]
  end ;

: main ( -- )
  Nil 10 swap Cons  # Simple literal Cons
  Nil               # accumulator

  test-exact-shuffle  # Creates new Cons with shuffled values

  list-reverse      # FAILS: "match: non-exhaustive pattern"
  drop ;
```

**Test file:** `tmp/test-exact-shuffle.cem`

### What Works vs. What Fails

| Test Case | Result | Notes |
|-----------|--------|-------|
| `Cons(10, Nil)` anywhere | ✅ Works | Literal values |
| `Cons(double(5), Nil)` in main | ✅ Works | Function result, outside match |
| `Cons(double(5), Nil)` passed to simple function | ✅ Works | Function result works in general |
| `Cons(10, Nil)` in match branch | ✅ Works | Literal in match branch |
| `Cons(head, Nil)` in match (simple shuffle) | ✅ Works | Extracted value with simple shuffle |
| `swap Cons` in match after `rot swap rot rot swap` | ❌ FAILS | Complex shuffling pattern |

**Corrected Hypothesis:** The issue is NOT about function calls vs. literals. It's about constructing Cons **inside a match branch** after a **specific sequence of stack operations** (`rot swap rot rot swap`). The Cons appears valid immediately after creation but becomes malformed when passed to functions that pattern match on it.

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

### 2025-10-20 - Root Cause Isolation

**Major Breakthrough:** The bug is NOT about function results vs. literals!

#### Discovery Process

1. **Initial hypothesis was wrong**: Thought it was about function results (e.g., `Cons(double(5), Nil)`)
2. **Created systematic tests** (see tmp/*.cem files):
   - `test-cons-reverse-literal.cem` - ✅ Literal Cons with list-reverse works
   - `test-cons-reverse-direct.cem` - ❌ Cons from function result fails with list-reverse
   - But: `test-cons-pass-through.cem` - ✅ Same Cons works through simple function
3. **Added debug output to list-reverse-helper**:
   - First match on original Cons: ✅ SUCCESS
   - Second match on newly-created Cons: ❌ FAILURE
4. **Isolated the pattern**: Created `test-exact-shuffle.cem` that replicates ONLY the shuffling pattern:
   ```cem
   rot swap rot rot swap Cons
   ```
   This fails even with literal initial values!

#### Key Findings

1. **Cons constructed outside match branches** - Always works (literals or function results)
2. **Cons constructed inside match branches with simple shuffling** - Works
3. **Cons constructed inside match branches after `rot swap rot rot swap`** - FAILS

#### The Smoking Gun

File: `tmp/test-exact-shuffle.cem`
```cem
: test-exact-shuffle ( List(Int) List(Int) -- List(Int) )
  swap
  match
    Cons => [
      rot swap rot rot swap Cons  # This creates malformed Cons!
      swap drop
    ]
    Nil => [ ]
  end ;
```

When this Cons is passed to `list-reverse`, it fails with "non-exhaustive pattern" even though:
- The Cons variant tag is correct (0)
- It can be inspected immediately after creation
- The IR shows identical Cons construction code

#### Current Hypothesis

The issue is likely in how the compiler manages stack cell lifetimes and linking **within match branch context**. After multiple rot/swap operations:
1. The `next` pointers in the stack cells might be pointing to wrong locations
2. The variant field linking might be using stale stack cell addresses
3. Or there's a subtle interaction between match extraction's `copy_cell` and subsequent shuffling

#### Test Files Created

All in `tmp/` directory:
- `test-literal-in-match.cem` - ✅ Literal Cons in match works
- `test-extracted-value-in-match.cem` - ✅ Extracted value + literal Nil works
- `test-extracted-acc-in-match.cem` - ✅ Literal + extracted acc works
- `test-both-extracted-in-match.cem` - ✅ Both extracted (simple shuffle) works
- `test-exact-shuffle.cem` - ❌ Complex shuffle pattern fails
- `debug-reverse.cem` - Shows failure happens on second match, not first

#### Next Steps

1. **Examine LLVM IR deeply**: Compare `test-both-extracted-in-match.ll` (works) vs `test-exact-shuffle.ll` (fails)
2. **Look for stack pointer issues**: After multiple rot/swap, are we copying from the right stack locations?
3. **Check variant field linking**: When we link the two Cons fields, are we using the correct cell addresses?
4. **Instrument the runtime**: Add assertions in `push_variant` to validate field cell integrity

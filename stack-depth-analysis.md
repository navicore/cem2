# Stack Depth Analysis for Higher-Order Functions

## Problem Statement

With only `swap` (depth 2) and `rot` (depth 3), manipulating items at depth 4+ is extremely difficult.

## Common Patterns Needed

### Pattern 1: Duplicate item at depth N
For `( a b c d )` where we want `( a b c d a )`:

**With pick4:**
```
pick4   # ( a b c d a )
```

**Without pick4:**
```
# Bring a to top, dup, restore
swap            # ( a b d c )
rot             # ( a d c b )
rot             # ( d c b a )
dup             # ( d c b a a )
rot             # ( d c a a b )
rot             # ( d a a b c )
swap            # ( d a a c b )
# Wrong! We need ( a b c d a )
```

Actually, let me trace this correctly:
```
( a b c d )
swap            # ( a b d c )
rot             # ( a d c b )
rot             # ( d c b a )  ✓ a is on top
dup             # ( d c b a a )
# Now restore to ( a b c d a )
rot             # ( d c a a b )
rot             # ( d a a b c )
swap            # ( d a a c b )
# Still wrong...
```

This is **extremely complex** even for depth 4.

### Pattern 2: Apply function to element deep in stack
For `( a b c d )` where `a` is a function and we want `a(d)`:

**With pick4 + call:**
```
pick4           # ( a b c d a )
swap call       # ( a b c result )
```

**Without:** Would be 10+ operations

## Recommendation

We should add **depth-4 primitives** for stdlib HOFs:

1. **pick ( n -- )**: Copy item at depth n to top
   - `pick 2` = `over`
   - `pick 3` would copy 3rd item
   - `pick 4` would copy 4th item

2. **roll ( n -- )**: Rotate top n items
   - `roll 2` = `swap`
   - `roll 3` = `rot`
   - `roll 4` would rotate 4 items

## Why This Is Better

1. **Readability**: `pick 4` is clearer than 8 swap/rot operations
2. **Correctness**: Easier to verify than complex shuffling
3. **Performance**: Single operation vs. many
4. **Extensibility**: Works for any depth user code needs

## Implementation Cost

Add to runtime (`runtime/src/stack.rs`):
```rust
pub unsafe extern "C" fn pick(stack: *mut StackCell, n: i64) -> *mut StackCell
pub unsafe extern "C" fn roll(stack: *mut StackCell, n: i64) -> *mut StackCell
```

Add to codegen as builtin words.

## Alternative: Just pick4

If we want to minimize additions, just add `pick4` (or `pick-4`) specifically for depth-4 access:
```cem
: pick4 ( a b c d -- a b c d a )
  # Implemented as builtin
```

This solves 90% of the problem for stdlib.

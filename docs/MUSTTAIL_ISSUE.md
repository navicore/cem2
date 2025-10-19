# Musttail Continuation Issue: Cannot Call Operations in Match Branches

**Status**: CRITICAL BLOCKER
**Severity**: HIGH - Makes language practically unusable
**Found**: 2025-10-19

## The Problem

Users cannot call basic stack operations like `drop`, `swap`, etc. inside match expression branches. This causes programs to hang.

### Example That Hangs

```cem
42 Some

match
  Some => [ drop ]  # ❌ HANGS
  None => [ ]
end
```

### Workaround (Not Acceptable Long-Term)

```cem
42 Some

match
  Some => [ ]  # Leave field on stack
  None => [ ]
end

drop  # ✅ Works outside match
```

## Why This Is Unacceptable

**This is a fundamental language design flaw.** It's unprecedented in modern languages to restrict which primitives can be called in different scopes.

Users expect:
- `drop` should work everywhere
- `swap`, `dup`, `rot` should work everywhere
- All stack operations should be available in all contexts

Without this, the language is practically unusable for real programs.

## Impact

Blocks all stdlib functions that need to manipulate matched fields:

```cem
# list-head: BROKEN
: list-head ( List(T) -- T )
  match
    Cons => [ swap drop ]  # ❌ HANGS - can't drop tail
    Nil  => [ "empty" write_line 1 exit ]
  end ;

# list-tail: BROKEN
: list-tail ( List(T) -- List(T) )
  match
    Cons => [ drop ]  # ❌ HANGS - can't drop head
    Nil  => [ "empty" write_line 1 exit ]
  end ;

# list-length: BROKEN
: list-length ( List(T) -- Int )
  0 swap
  match
    Cons => [
      rot 1 + swap  # ❌ HANGS - can't manipulate stack
      list-length +
      swap drop
    ]
    Nil => [ ]
  end ;
```

**Every useful list operation is broken.**

## Root Cause

The match expression codegen uses musttail calls for continuation:

**Location**: `compiler/src/codegen/mod.rs:1373-1400`

```rust
let (branch_stack, ends_with_musttail) =
    self.compile_expr_sequence(&branch.body, &initial_stack)?;

// ... later ...

if ends_with_musttail {
    // Branch ends with musttail call, no explicit branch needed
    // The quotation call handles the continuation
} else {
    writeln!(&mut self.output, "  br label %{}", merge_label)?;
}
```

The problem: When a branch contains operations like `drop`, the generated LLVM IR becomes invalid or creates unreachable code after the musttail call.

## Evidence

From earlier session, test_cons.ll showed:
```llvm
; Match branch with drop
%123 = musttail call ptr @drop(ptr %122)
ret ptr %123

; Unreachable code after musttail
%124 = ...  ; ❌ ERROR: Code after musttail/ret
```

LLVM doesn't allow instructions after `musttail call` + `ret`. The continuation must be handled differently.

## Potential Solutions

### Option 1: Don't Use Musttail for Simple Operations

If the branch body is just stack operations (no quotation calls), don't use musttail:

```rust
let uses_musttail = branch_ends_with_quotation_call(&branch.body);

if uses_musttail {
    // Use musttail continuation
    writeln!(&mut self.output, "  musttail call ptr @call_quotation(...)")?;
    writeln!(&mut self.output, "  ret ptr %result")?;
} else {
    // Normal branch - just execute operations and branch to merge
    self.compile_expr_sequence(&branch.body, &initial_stack)?;
    writeln!(&mut self.output, "  br label %{}", merge_label)?;
}
```

### Option 2: Trampoline Pattern

Use a trampoline to handle continuations without musttail:

```rust
// Generate branch code without musttail
let result = self.compile_expr_sequence(&branch.body, &initial_stack)?;
// Store result and continuation
writeln!(&mut self.output, "  store ptr %{}, ptr @continuation", result)?;
writeln!(&mut self.output, "  br label %{}", merge_label)?;
```

Then in the merge block, check if there's a continuation and jump to it.

### Option 3: CPS Transform Only When Needed

Only use continuation-passing style for recursive functions or quotation calls:

```rust
if self.is_recursive_call(&expr) || self.is_quotation_call(&expr) {
    // Use CPS with musttail
} else {
    // Normal direct-style code
}
```

## Recommended Approach

**Start with Option 1** - it's the simplest and most direct.

1. Detect if branch body ends with quotation call
2. Only use musttail for quotation calls
3. Use normal branches for stack operations

This preserves tail-call optimization where needed (quotations) while allowing normal operations everywhere else.

## Testing Strategy

1. **Unit test**: Match branch with `drop`
2. **Unit test**: Match branch with multiple stack ops (`swap drop drop`)
3. **Integration test**: `list-head` from stdlib
4. **Integration test**: `list-length` (recursive)
5. **Integration test**: Nested matches with operations

## Success Criteria

- [ ] `test_option_match.cem` works (drop in Some branch)
- [ ] `test_cons_match.cem` works with `swap drop drop`
- [ ] `list-head` works
- [ ] `list-tail` works
- [ ] `list-length` works
- [ ] All stdlib list operations work
- [ ] No performance regression for quotation calls

## Files to Modify

- `compiler/src/codegen/mod.rs:1200-1400` - Match expression codegen
- Add helper: `fn branch_uses_quotation(&self, exprs: &[Expr]) -> bool`
- Update: Match branch compilation to conditionally use musttail

## References

- LLVM musttail documentation
- Previous session notes about "unreachable code after musttail"
- test_cons.ll IR inspection showing the error

## Timeline

**Priority**: CRITICAL - blocks all practical use of the language

**Estimated effort**: 2-4 hours
- 1 hour: Understand current musttail logic
- 1-2 hours: Implement conditional musttail
- 1 hour: Test and verify

## Notes

This is not a "nice to have" optimization issue. This is a **fundamental language correctness issue**. Without this fix, users cannot write basic programs. The musttail optimization is good, but it must not break basic functionality.

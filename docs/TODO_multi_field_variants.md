# TODO: Complete Multi-Field Variant Support

## Context
See `docs/multi_field_variants_bug.md` for full implementation details and bug report.

## Current Status
- ✅ Design completed (linked-list field chaining)
- ✅ Runtime support (no changes needed)
- ✅ Codegen for construction implemented
- ✅ Codegen for pattern matching implemented
- ✅ `exit_op` runtime function added
- ✅ Stdlib prelude created with list operations
- ❌ **BLOCKED**: Field counting bug (allocating 3 cells instead of 2 for Cons)

## Task List

### 1. Find Variant Registration Code
**Goal**: Locate where `variant_field_counts` HashMap is populated

**Search for**:
```bash
grep -r "variant_field_counts" compiler/src/
grep -r "register.*type" compiler/src/
grep -r "type.*def" compiler/src/codegen/
```

**Expected to find**:
- Function that processes `TypeDef` AST nodes
- Code that iterates over `variants` and counts `fields.len()`
- HashMap insertion: `variant_field_counts.insert("Cons", 2)`

**Files to check**:
- `compiler/src/codegen/mod.rs` (likely in `new()` or `compile()`)
- `compiler/src/typechecker/environment.rs` (type registration)

### 2. Debug Field Count
**Goal**: Understand why `field_count` is 3 instead of 2

**Add debug output**:
```rust
// In variant construction code (codegen/mod.rs:~907)
let field_count = self.variant_field_counts.get(name).copied().unwrap_or(0);
eprintln!("DEBUG: Variant {} has field_count={}", name, field_count);
```

**Verify**:
- What does `fields.len()` return for Cons in the TypeDef?
- Is the bug in type parsing or registration?
- Check if prelude type definition matches user type definition

### 3. Inspect Type Definition AST
**Goal**: Verify the parsed type structure

**Add debug**:
```rust
// When processing type definitions
for variant in &type_def.variants {
    eprintln!("Variant {}: {} fields", variant.name, variant.fields.len());
    for (i, field) in variant.fields.iter().enumerate() {
        eprintln!("  Field {}: {:?}", i, field);
    }
}
```

**Expected for Cons**:
```
Variant Cons: 2 fields
  Field 0: Var("T")
  Field 1: Named { name: "List", args: [Var("T")] }
```

**If seeing 3 fields**, the bug is in type definition parsing (parser or typechecker).

### 4. Fix the Bug

**Hypothesis 1: Prelude vs User Type Collision** ⭐ MOST LIKELY
- Both stdlib/prelude.cem and test files declare `type List`
- Compiler auto-includes prelude (main.rs:78-88), then parses user file
- **Code review insight**: This likely causes double registration!
  - First pass: Prelude defines `List(T)` with `Cons(T, List(T))` → 2 fields
  - Second pass: User file defines `List(T)` again → might overwrite or corrupt

**Evidence from review**:
- Registration code at codegen/mod.rs:169-170 looks correct
- No duplicate detection in registration loop
- String concatenation creates combined source, both definitions present

**Test**: Try test_cons.cem WITHOUT prelude being auto-included
```rust
// Temporarily comment out in compiler/src/main.rs:78-88
// let prelude = fs::read_to_string(prelude_path)...
```

**Proper fix** (from review):
```rust
// Check for duplicates during registration
if self.variant_tags.contains_key(&variant_name) {
    eprintln!("Warning: Duplicate variant definition for {}", variant_name);
    continue; // Skip duplicate
}
```

**Hypothesis 2: Type Parameter Counting**
- Maybe counting `List(T)` as 2 items instead of 1?
- T and List instead of just the List type?

**Test**: Create a simpler variant
```cem
type Pair(A, B)
  | MakePair(A, B)
  | Empty

: main ( -- )
  1 2 MakePair drop ;
```

**Hypothesis 3: Stack Depth Confusion**
- Maybe counting stack depth instead of field count?

**Fix locations**:
- Find variant registration code
- Ensure: `variant_field_counts["Cons"] = type_def.variants[0].fields.len()`
- Should be exactly 2 for Cons

### 5. Verify Construction Works
**Test**: `./target/release/cem compile examples/test_cons.cem && ./test_cons`

**Expected output**:
```
Testing Cons construction...
Got Cons!
Done!
```

**Verify in LLVM IR**:
```bash
grep "alloc_cell" test_cons.ll | wc -l
# Should see exactly 2 for Cons construction
```

### 6. Test List Operations
**Test**: `./target/release/cem compile examples/test_list.cem && ./test_list`

**Expected output**:
```
Testing list operations...
Test 1: Empty list...
true
Test 2: Creating list [1, 2, 3]...
false
Test 3: Getting head of [1, 2, 3]...
1
Test 4: Getting head of tail...
2
Test 5: Getting length of [1, 2, 3]...
3
All list tests passed!
```

### 7. Run CI
**Ensure all checks pass**:
```bash
just ci
```

### 8. Add Comprehensive Tests
**Create**: `examples/test_multifield_variants.cem`

```cem
# Test 2-field variant
type Pair(A, B)
  | MakePair(A, B)
  | NoPair

# Test 3-field variant
type Triple(A, B, C)
  | MakeTriple(A, B, C)
  | NoTriple

# Test 4-field variant
type Quad(A, B, C, D)
  | MakeQuad(A, B, C, D)
  | NoQuad

: main ( -- )
  # Test 2-field construction and matching
  1 2 MakePair match
    MakePair => [ + int-to-string write_line ]  # Should print 3
    NoPair => [ "error" write_line ]
  end

  # Test 3-field
  1 2 3 MakeTriple match
    MakeTriple => [ + + int-to-string write_line ]  # Should print 6
    NoTriple => [ "error" write_line ]
  end

  # Test 4-field
  1 2 3 4 MakeQuad match
    MakeQuad => [ + + + int-to-string write_line ]  # Should print 10
    NoQuad => [ "error" write_line ]
  end

  "All multi-field tests passed!" write_line ;
```

## Additional Issues from Code Review

### Security: Path Traversal Risk
**Location**: compiler/src/main.rs:78
**Issue**: Prelude path is relative to CWD, not binary location
**Fix**: Make stdlib path configurable or relative to binary:
```rust
let stdlib_dir = std::env::var("CEM_STDLIB_PATH")
    .unwrap_or_else(|_| {
        let exe_path = std::env::current_exe().unwrap();
        let exe_dir = exe_path.parent().unwrap();
        exe_dir.join("../stdlib").to_str().unwrap().to_string()
    });
```

### Source Location Accuracy
**Location**: compiler/src/main.rs:84
**Issue**: Concatenating prelude + user source breaks line numbers in errors
**Better approach**: Parse prelude separately, merge ASTs
```rust
let prelude_parser = Parser::new_with_filename(&prelude_source, prelude_path);
let prelude_program = prelude_parser.parse()?;
let user_program = user_parser.parse()?;
let merged = merge_programs(prelude_program, user_program)?;
```

### Exit Code Validation
**Location**: runtime/src/io.rs:93
**Issue**: Large integers truncate silently when cast to i32
**Fix**: Clamp to valid range:
```rust
if exit_code < 0 || exit_code > 255 {
    eprintln!("Warning: exit code {} out of range", exit_code);
}
std::process::exit((exit_code & 0xFF) as i32);
```

## Success Criteria
- [ ] Understand where variant_field_counts is populated
- [ ] Identify root cause of field count being 3 instead of 2
- [ ] Fix the bug (likely: add duplicate detection)
- [ ] test_cons.cem compiles and runs
- [ ] test_list.cem works with all stdlib list operations
- [ ] LLVM IR shows correct number of alloc_cell calls
- [ ] CI passes
- [ ] Multi-field variant tests added and passing
- [ ] (Post-merge) Fix security issues: path traversal, source locations, exit code validation

## Reference Files
- Bug report: `docs/multi_field_variants_bug.md`
- Test cases: `examples/test_cons.cem`, `examples/test_list.cem`
- Implementation: `compiler/src/codegen/mod.rs` lines 974-1062, 1284-1333
- Stdlib: `stdlib/prelude.cem`

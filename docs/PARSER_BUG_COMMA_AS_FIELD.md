# Parser Bug: Comma Treated as Field Type

**Status**: IDENTIFIED - Ready to fix
**Severity**: CRITICAL (blocks multi-field variants)
**Found**: 2025-10-19

## The Bug

When parsing type definitions with multiple fields like:
```cem
type List(T)
  | Cons(T, List(T))
  | Nil
```

The parser treats the **comma** between fields as a third field with name `,`:

```rust
Variant Cons: 3 fields ([
    Var("T"),                                    // Field 0: correct
    Named { name: ",", args: [] },               // Field 1: BUG! Comma parsed as type
    Named { name: "List", args: [Var("T")] }     // Field 2: correct
])
```

**Expected**: 2 fields `[Var("T"), Named("List", ...)]`
**Actual**: 3 fields with comma as middle field

## Impact

This causes:
1. Field count to be wrong (3 instead of 2)
2. Multi-field variant construction to allocate too many cells
3. Pattern matching to walk wrong number of fields
4. Stack overflow when using lists

## Evidence

Debug output from compiler/src/codegen/mod.rs:
```
DEBUG: Processing type List
  Variant Cons: 3 fields ([Var("T"), Named { name: ",", args: [] }, Named { name: "List", args: [Var("T")] }])
  Variant Nil: 0 fields ([])
```

Compiled LLVM IR shows 3 `alloc_cell()` calls for Cons construction instead of 2.

## Root Cause

The bug is in the **parser's type definition handling**. The relevant code is likely in:
- `compiler/src/parser/parse.rs` - type definition parsing
- Function that parses variant field lists

The parser needs to:
1. Parse comma-separated types
2. **Treat commas as separators, not as type names**
3. Return a Vec<Type> with only the actual field types

## Location to Fix

Search for where variant fields are parsed:

```bash
grep -n "parse.*variant\|parse.*field" compiler/src/parser/parse.rs
grep -n "TypeDef\|Variant {" compiler/src/parser/parse.rs
```

Look for code that builds the `fields: Vec<Type>` for a variant.

**Likely issue**: Using a tokenizer that treats `,` as a word/identifier instead of a separator.

## Fix Strategy

### Option 1: Fix Field List Parsing

```rust
// Current (buggy) - might be doing something like:
fn parse_variant_fields(&mut self) -> Result<Vec<Type>> {
    let mut fields = vec![];
    while !self.is_at_end() && !self.check(Token::RParen) {
        fields.push(self.parse_type()?);  // BUG: Also captures comma as type
    }
    Ok(fields)
}

// Fixed - consume comma as separator:
fn parse_variant_fields(&mut self) -> Result<Vec<Type>> {
    let mut fields = vec![];
    loop {
        if self.check(Token::RParen) {
            break;
        }
        fields.push(self.parse_type()?);

        // Consume comma if present
        if self.check(Token::Comma) {
            self.advance(); // Skip the comma
        } else {
            break; // No more fields
        }
    }
    Ok(fields)
}
```

### Option 2: Filter Out Comma Types

Quick fix (not recommended):
```rust
// In variant registration (codegen/mod.rs:167)
for (idx, variant) in typedef.variants.iter().enumerate() {
    let actual_fields: Vec<_> = variant.fields.iter()
        .filter(|f| !matches!(f, Type::Named { name, .. } if name == ","))
        .collect();

    self.variant_field_counts.insert(
        variant.name.clone(),
        actual_fields.len()
    );
}
```

**Recommendation**: Use Option 1 - fix the parser properly.

## Test Case

Add parser test in `compiler/src/parser/tests.rs`:

```rust
#[test]
fn test_parse_multifield_variant() {
    let source = r#"
        type List(T)
          | Cons(T, List(T))
          | Nil
    "#;

    let mut parser = Parser::new(source);
    let program = parser.parse().unwrap();

    assert_eq!(program.type_defs.len(), 1);
    let typedef = &program.type_defs[0];
    assert_eq!(typedef.name, "List");
    assert_eq!(typedef.variants.len(), 2);

    // Check Cons has exactly 2 fields
    let cons_variant = &typedef.variants[0];
    assert_eq!(cons_variant.name, "Cons");
    assert_eq!(cons_variant.fields.len(), 2, "Cons should have 2 fields, not {}", cons_variant.fields.len());

    // Verify no comma in fields
    for field in &cons_variant.fields {
        if let Type::Named { name, .. } = field {
            assert_ne!(name, ",", "Comma should not be parsed as a field type");
        }
    }

    // Check Nil has 0 fields
    let nil_variant = &typedef.variants[1];
    assert_eq!(nil_variant.name, "Nil");
    assert_eq!(nil_variant.fields.len(), 0);
}
```

## Verification Steps

After fixing:

1. **Run parser test**:
   ```bash
   cargo test test_parse_multifield_variant
   ```

2. **Rebuild compiler and test**:
   ```bash
   cargo build --release -p cem-compiler
   ./target/release/cem compile examples/test_cons.cem
   ```

   Should see: `Variant Cons: 2 fields ([Var("T"), Named { name: "List", ... }])`

3. **Run compiled program**:
   ```bash
   ./test_cons
   ```

   Should output: "Testing Cons construction... Done!"

4. **Check LLVM IR**:
   ```bash
   grep -c "alloc_cell" test_cons.ll
   ```

   Should return: 2 (not 3)

5. **Run full test suite**:
   ```bash
   ./target/release/cem compile examples/test_list.cem && ./test_list
   ```

## Related Files

- **Parser**: `compiler/src/parser/parse.rs` (needs fix)
- **AST**: `compiler/src/ast/mod.rs` (Type, Variant definitions)
- **Tests**: `compiler/src/parser/tests.rs` (add test)
- **Examples**: `examples/test_cons.cem`, `examples/test_list.cem`

## Next Steps

1. Find variant field parsing code in parse.rs
2. Fix to treat comma as separator, not type
3. Add parser test
4. Verify all tests pass
5. Test examples work
6. Update PR and merge

## Success Criteria

- [ ] Parser test passes (Cons has 2 fields)
- [ ] No comma in parsed field list
- [ ] test_cons.cem compiles and runs
- [ ] test_list.cem works
- [ ] LLVM IR shows 2 alloc_cell calls
- [ ] All CI checks pass

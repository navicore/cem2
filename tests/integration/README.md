# Integration Tests

This directory contains **integration tests** that verify the Cem2 compiler and runtime work correctly end-to-end.

## Purpose

These tests:
- **Verify compilation** - Check that various language constructs compile without errors
- **Test runtime behavior** - Ensure programs execute correctly
- **Exercise edge cases** - Test boundary conditions and error handling
- **Prevent regressions** - Catch bugs that might reappear

## Running Integration Tests

```bash
# Run all integration tests
just test-integration

# This is part of CI
just ci
```

The test runner will:
1. Compile each `.cem` file
2. Execute the compiled program
3. Report success (✅) or failure (❌)

## Test Categories

### tdd-tests/
Tests moved from the old `examples/tdd-tests` directory. These verify:
- Pattern matching behavior
- List operations
- Option type handling
- Stack operations
- Variant construction and matching

## Writing Integration Tests

Integration tests should:
- **Exit with code 0** on success
- **Be self-contained** - don't depend on other tests
- **Test one thing** - focus on a specific feature or edge case
- **Use descriptive names** - e.g., `cons-construction.cem`, not `test1.cem`

### Example Test Structure

```cem
# Test that Cons construction works correctly
: main ( -- )
  # Create a list
  Nil
  42 swap Cons

  # Verify it matches as Cons
  match
    Cons => [ drop drop ]  # Success - it's a Cons
    Nil => [ 1 exit ]      # Failure - should never reach here
  end ;
```

## Tests vs Examples

**Integration Tests** (here in `tests/integration/`):
- Verify correctness
- May just print "Done!" or nothing
- Part of CI pipeline
- Can use assertions and exit codes

**Examples** (in `examples/`):
- Demonstrate features
- Must produce meaningful output
- Educational purpose
- Show real-world usage

If your code just checks that something compiles or runs, it's a **test**.
If it demonstrates how to use a feature with clear output, it's an **example**.

## Manual Tests

Some tests require manual verification (non-zero exit codes, specific error messages, etc.).
These are in `tests/manual/` and are NOT run automatically by CI.

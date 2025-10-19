# TDD Test Examples

Examples demonstrating test-driven development patterns in Cem2.

## Testing Philosophy

These examples show how to write tests in Cem2:
- **Small, focused tests** - Each file tests one specific behavior
- **Clear assertions** - Tests should clearly show expected vs actual
- **Debugging techniques** - Using `write_line` to trace execution
- **Edge cases** - Testing boundary conditions and error cases

## Example Categories

### Construction Tests
- `cons-construction.cem` - Building Cons cells
- `cons-simple.cem` - Simple list creation and destruction

### Matching Tests
- `cons-matching.cem` - Pattern matching on Cons
- `match-drop-test.cem` - Testing drop in match branches
- `match-debugging.cem` - Debugging match expressions

### List Operation Tests
- `list-simple.cem` - Basic list operations without recursion
- `list-head.cem` - Testing list-head function
- `list-length-manual.cem` - Manual list-length implementation (for comparison)
- `list-full.cem` - Comprehensive list operation tests

### Option Type Tests
- `option-type.cem` - Basic Option matching
- `option-nodrop.cem` - Option without dropping values

### Exit Code Tests
- `exit-code-valid.cem` - Testing valid exit codes (0)
- `exit-code-max-valid.cem` - Testing maximum valid exit code (255)
- `exit-code-invalid.cem` - Testing invalid exit code (256, should error)
- `exit-code-negative.cem` - Testing negative exit code (-1, should error)

### Debugging Examples
- `debugging-with-output.cem` - Using write_line for debugging
- `match-debugging.cem` - Debugging pattern matching

## TDD Pattern in Cem2

### 1. Red Phase - Write a Failing Test

```cem
: test-list-length ( -- )
  "Testing list-length..." write_line

  # Create [1, 2, 3]
  Nil 3 swap Cons 2 swap Cons 1 swap Cons

  # Should be 3
  list-length
  3 = not [ "FAIL: Expected 3" write_line 1 exit ] [ ] if

  "PASS" write_line ;
```

### 2. Green Phase - Make it Pass

Implement the `list-length` function in the stdlib.

### 3. Refactor Phase - Improve the Code

Make list-length tail-recursive for better performance.

## Assertions

Since Cem2 doesn't have a test framework yet, use conditional exits:

```cem
# Assert equal
expected actual = not [
  "FAIL: assertion failed" write_line
  1 exit
] [ ] if

# Assert not equal
expected actual = [
  "FAIL: should not be equal" write_line
  1 exit
] [ ] if
```

## Debugging with write_line

Insert debug output to trace execution:

```cem
: process ( List(Int) -- )
  "Before match..." write_line
  match
    Cons => [
      "In Cons branch..." write_line
      "Head: " write swap int-to-string write_line
      drop
      "After drop..." write_line
    ]
    Nil => [ "In Nil branch..." write_line ]
  end
  "After match..." write_line ;
```

## Running Tests

```bash
# Run a single test
./target/release/cem compile examples/tdd-tests/list-simple.cem -o test
./test && echo "PASS" || echo "FAIL (exit code $?)"

# Run all tests
for test in examples/tdd-tests/*.cem; do
  name=$(basename "$test" .cem)
  ./target/release/cem compile "$test" -o "test_$name"
  ./test_$name && echo "$name: PASS" || echo "$name: FAIL"
done
```

## Edge Cases to Test

- **Empty inputs**: Empty lists, None values
- **Boundary values**: 0, -1, max values (255 for exit codes)
- **Invalid inputs**: Out of range values, wrong types
- **Nested structures**: Lists containing lists, nested Options

## Best Practices

1. **Test one thing** - Each test should verify one specific behavior
2. **Clear names** - File names should describe what is being tested
3. **Minimal code** - Tests should be as simple as possible
4. **Good output** - Use write_line to show what's being tested
5. **Exit codes** - Use exit code 0 for pass, 1 for fail

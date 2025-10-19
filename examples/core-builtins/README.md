# Core Builtins Examples

Examples demonstrating core language features: stack operations, strings, I/O, and arithmetic.

## Examples

### stack-operations.cem
Demonstrates core stack manipulation operations.

**Operations covered**:
- `dup` - Duplicate top of stack
- `swap` - Swap top two items
- `rot` - Rotate top three items
- `drop` - Remove top item
- `over` - Copy second item to top

### stack-drop.cem
Shows how to drop values from the stack and clean up resources.

### string-operations.cem
String manipulation using stdlib functions.

**Operations covered**:
- `string-concat` - Concatenate two strings
- `string-length` - Get string length
- `string-equal` - Compare strings for equality

## Stack-Based Programming

Cem2 is a concatenative, stack-based language. All operations work on an implicit stack:

```cem
# ( -- 5 )
5

# ( -- 5 10 )
5 10

# ( -- 10 5 ) - swap
5 10 swap

# ( -- 15 ) - add
5 10 +
```

Stack comments show the stack state: `( before -- after )`

## Running Examples

```bash
./target/release/cem compile examples/core-builtins/stack-operations.cem -o stack-ops
./stack-ops
```

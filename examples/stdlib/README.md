# Standard Library Examples

Examples demonstrating stdlib types and functions.

## Built-in Types

The Cem2 standard library provides:

### List(T)
A linked list with two constructors:
- `Cons(T, List(T))` - A node with a value and a tail
- `Nil` - The empty list

### Option(T)
An optional value with two constructors:
- `Some(T)` - A value is present
- `None` - No value

## Examples

### list-operations.cem
Demonstrates list operations from the stdlib.

**Functions shown**:
- `list-length` - Count elements (tail-recursive, safe for large lists)
- `list-head` - Get first element (unsafe: crashes on empty)
- `list-head-safe` - Get first element safely (returns Option)
- `list-tail` - Get rest of list (unsafe)
- `list-tail-safe` - Get rest of list safely (returns Option)
- `list-reverse` - Reverse a list
- `list-append` - Concatenate two lists
- `list-is-empty` - Check if list is empty

**Building lists**:
```cem
# Create [1, 2, 3]
Nil
3 swap Cons
2 swap Cons
1 swap Cons
```

### option-matching.cem
Shows how to use the Option type with pattern matching.

**Pattern matching on Option**:
```cem
: process ( Option(Int) -- )
  match
    Some => [ "Got value: " write swap int-to-string write_line ]
    None => [ "No value" write_line ]
  end ;
```

## Safe vs Unsafe Functions

Some stdlib functions have both safe and unsafe variants:

**Unsafe** (crashes on invalid input):
- `list-head` - Crashes on empty list
- `list-tail` - Crashes on empty list

**Safe** (returns Option):
- `list-head-safe` - Returns `None` on empty list
- `list-tail-safe` - Returns `None` on empty list

Use safe variants when the list might be empty. Use unsafe variants when you've already checked the list is non-empty (e.g., after `list-is-empty`).

## Running Examples

```bash
./target/release/cem compile examples/stdlib/list-operations.cem -o list-ops
./list-ops
```

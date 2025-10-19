# Cem2 Examples

This directory contains examples demonstrating various features of the Cem2 language.

## Directory Structure

### [getting-started/](getting-started/)
Start here! Simple examples for new users learning Cem2.
- `hello-world.cem` - The classic first program

### [core-builtins/](core-builtins/)
Examples of core language features: stack operations, strings, I/O, and arithmetic.
- `stack-operations.cem` - Stack manipulation (dup, swap, rot, etc.)
- `stack-drop.cem` - Dropping values from the stack
- `string-operations.cem` - String concatenation and operations

### [stdlib/](stdlib/)
Standard library features: Lists, Options, and other common data structures.
- `list-operations.cem` - Working with the List type (list-length, list-head, etc.)
- `option-matching.cem` - Using the Option type with pattern matching

### [pattern-matching/](pattern-matching/)
Pattern matching on algebraic data types and custom variants.
- `simple-variant.cem` - Basic variant matching
- `nested-matching.cem` - Matching on nested structures
- `custom-pair-type.cem` - Creating and matching custom types

### [tdd-tests/](tdd-tests/)
Test-driven development examples showing how to write tests in Cem2.
These examples demonstrate testing patterns, edge cases, and debugging techniques.

## Running Examples

To compile and run any example:

```bash
# From project root
./target/release/cem compile examples/<category>/<example>.cem -o <output-name>
./<output-name>
```

For example:

```bash
./target/release/cem compile examples/getting-started/hello-world.cem -o hello
./hello
```

## Building All Examples

```bash
just build-examples
```

This compiles all examples to `target/examples/`.

## Learning Path

Recommended order for learning Cem2:

1. **getting-started/** - Learn the basics
2. **core-builtins/** - Understand stack operations and primitives
3. **pattern-matching/** - Learn algebraic data types
4. **stdlib/** - Use built-in data structures
5. **tdd-tests/** - See testing and debugging patterns

## Contributing Examples

When adding new examples:
- Place them in the appropriate category directory
- Use descriptive filenames with hyphens (e.g., `custom-pair-type.cem`)
- Add comments explaining what the example demonstrates
- Update the category's README.md
- Keep examples focused on one concept

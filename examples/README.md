# Cem2 Examples

This directory contains **meaningful, demonstrative examples** that showcase Cem2's features with actual output.

## Philosophy

Examples here should:
- **Demonstrate language features** with clear, visible output
- **Show real-world usage** patterns
- **Be self-explanatory** and educational
- **NOT** just print "Done!" - they should show what the code does!

## Directory Structure

### [getting-started/](getting-started/)
Start here! Simple examples for new users learning Cem2.
- `hello-world.cem` - The classic first program

### [core-builtins/](core-builtins/)
Examples of core language features: stack operations, strings, I/O, and arithmetic.
- `stack-operations.cem` - Demonstrates arithmetic, comparisons, and stack manipulation
- `string-operations.cem` - String length, concatenation, and equality testing

## Running Examples

To compile and run any example:

```bash
# Build all examples
just build-examples

# Run a specific example
./target/examples/getting-started-hello-world
```

For example:

```bash
./target/release/cem compile examples/getting-started/hello-world.cem -o hello
./hello
```

## Note on Tests vs Examples

**Examples** demonstrate features with meaningful output.
**Tests** verify that features work correctly.

If you're writing code to **test** language features (checking compilation, exercising edge cases, etc.), put it in `tests/integration/` instead. See [tests/integration/README.md](../tests/integration/README.md).

Tests belong in `tests/`, examples belong in `examples/`. Keep them separate!

## Contributing Examples

When adding new examples:
- Ensure they produce **meaningful output** that demonstrates the feature
- Use descriptive filenames with hyphens (e.g., `custom-data-structures.cem`)
- Add comments explaining what the example demonstrates
- Update this README if adding a new category
- Keep examples focused on one concept

# Getting Started Examples

Simple examples for new users learning Cem2.

## Examples

### hello-world.cem
The classic "Hello, World!" program demonstrating basic I/O.

```cem
: main ( -- )
  "Hello, World!" write_line ;
```

**Concepts**:
- Function definitions with `:` and `;`
- String literals
- Built-in I/O functions

**Run**:
```bash
./target/release/cem compile examples/getting-started/hello-world.cem -o hello
./hello
```

## Next Steps

After mastering these basics, explore:
- [core-builtins/](../core-builtins/) - Stack operations and primitives
- [pattern-matching/](../pattern-matching/) - Algebraic data types
- [stdlib/](../stdlib/) - Lists, Options, and more

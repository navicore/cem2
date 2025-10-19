# Cem2 Project Status

Last updated: 2025-10-19

## Recent Accomplishments

### Merged PRs
- **PR #4**: Memory safety fixes (Edition 2024 compliance)
- **PR #5**: Missing runtime operations (stack ops, arithmetic, comparisons, type conversions)
- **PR #6**: String operations with optimizations from code review

### Current Work: Multi-Field Variant Support
**Status**: Implementation complete, debugging field count bug

**Why this matters**: Multi-field variants are fundamental for:
- List operations (`Cons(T, List(T))` has 2 fields)
- Tree structures (3+ fields)
- General algebraic data types
- Enables stdlib implementation in pure Cem

**What's done**:
- ✅ Design: Chain fields as linked list
- ✅ Codegen: Construction and pattern matching
- ✅ Runtime: `exit_op` function
- ✅ Stdlib: Prelude with list operations ready
- ❌ Bug: Allocating 3 cells instead of 2 for 2-field variant

**Next step**: Debug and fix field counting (see `docs/TODO_multi_field_variants.md`)

## Test Coverage
- **Runtime tests**: 43 passing
- **Compiler tests**: 46 passing
- **Integration tests**: hello_world, test_option, test_variant_simple, test_new_ops, test_strings

## Architecture

### Runtime (Rust)
- Memory-safe with Edition 2024
- C-compatible FFI for LLVM
- May coroutines for green threads
- Proper Drop trait implementation

**Modules**:
- `stack.rs`: Stack operations, arithmetic, comparisons
- `io.rs`: I/O and exit
- `strings.rs`: String primitives (optimized, no double allocation)
- `conversions.rs`: Type conversions
- `pattern.rs`: Variant support
- `scheduler.rs`: May-based green threads

### Compiler (Rust)
- Parser with source location tracking
- Type checker with Hindley-Milner inference
- LLVM IR codegen
- Pattern matching on variants

**Modules**:
- `parser/`: Lexer and parser
- `typechecker/`: Type inference and checking
- `codegen/`: LLVM IR generation

### Stdlib (Cem)
- **Prelude**: Auto-included in every program
- **List operations**: Ready once multi-field variants work
  - `list-empty`, `list-cons`, `list-head`, `list-tail`
  - `list-is-empty`, `list-length`
  - `list-reverse`, `list-append`

## Language Features

### Working
- [x] Basic types: Int, Bool, String
- [x] Stack operations: dup, drop, swap, over, rot, nip, tuck
- [x] Arithmetic: +, -, *, /
- [x] Comparisons: =, <, >, <=, >=, !=
- [x] String operations: length, concat, equal
- [x] Type conversions: int-to-string, bool-to-string
- [x] 0-field variants (None)
- [x] 1-field variants (Some)
- [x] Pattern matching
- [x] Recursion
- [x] Type definitions
- [x] Polymorphic types
- [x] I/O: write_line, read_line
- [x] exit

### In Progress
- [ ] Multi-field variants (2+ fields)

### Planned
- [ ] Quotations as first-class values
- [ ] Higher-order functions (map, filter, fold)
- [ ] Module system / imports
- [ ] More stdlib types (Map, Set, etc.)

## Development Workflow

```bash
# Format code
just fmt

# Run all checks
just ci

# Build components
just build-runtime
just build-compiler

# Compile and run a program
./target/release/cem compile examples/hello_world.cem
./hello_world
```

## Key Design Decisions

### Stdlib Approach
**Decision**: Prelude (auto-included) for now, imports later

**Rationale**:
- Get list operations working quickly
- Can add imports as a non-breaking change
- Allows writing stdlib in pure Cem

### Multi-Field Variant Memory Layout
**Decision**: Chain fields as linked list

**Rationale**:
- Minimal changes to existing code
- Fields unwrap onto stack in declaration order
- Reuses StackCell linking mechanism
- Memory cleanup via Drop already works

### String Operations
**Decision**: Built-ins (C FFI), not stdlib

**Rationale**:
- Need unsafe C string operations
- Performance (avoid copies)
- UTF-8 validation required

## Documentation
- `docs/multi_field_variants_bug.md`: Detailed bug report and implementation
- `docs/TODO_multi_field_variants.md`: Task list for fixing current bug
- `docs/PROJECT_STATUS.md`: This file

## Getting Help
- Issues: https://github.com/navicore/cem2/issues
- Discussions: https://github.com/navicore/cem2/discussions

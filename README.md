# Cem2

*Pronounced "seam"*

A minimal, safe, concatenative programming language with linear types, pattern matching, and Erlang-style green threads.

## What Changed from Cem v1?

Cem2 uses the same language design and compiler architecture, but replaces the C/assembly runtime with a **memory-safe Rust runtime** built on [May coroutines](https://github.com/Xudong-Huang/may).

### Architecture Evolution

**Cem v1** (archived):
```
Rust Compiler → LLVM IR → C Runtime (manual memory, assembly context switching)
                            ❌ Segfaults in pattern matching
```

**Cem2** (this project):
```
Rust Compiler → LLVM IR → Rust Runtime (May coroutines, safe memory)
                            ✅ Memory safe, production-ready
```

### What We Keep

- ✅ **Concatenative syntax** - Stack-based composition
- ✅ **Linear types** - Resources used exactly once
- ✅ **Pattern matching** - Exhaustive checking on sum types
- ✅ **Erlang-style green threads** - 500K+ lightweight processes
- ✅ **Standalone binaries** - No runtime dependencies
- ✅ **LLVM backend** - Native code generation

### What We Fix

- ✅ **Memory safety** - Rust prevents segfaults
- ✅ **Simpler debugging** - No C memory management bugs
- ✅ **Production coroutines** - May is battle-tested
- ✅ **Better I/O** - May supports io_uring, epoll, kqueue

## Quick Example

```cem
: main ( -- )
  "Creating a list..." write-line

  # Build a list [1, 2, 3] using stdlib
  Nil
  3 swap Cons
  2 swap Cons
  1 swap Cons

  # Get length (should be 3)
  list-length
  int-to-string write-line

  "Done!" write-line ;
```

## Standard Library

Cem2 includes a **standard library prelude** that is automatically included in every program. The prelude provides:

### Built-in Types
- `List(T)` - Linked list with `Cons(head, tail)` and `Nil` constructors
- `Option(T)` - Optional values with `Some(value)` and `None` constructors

### List Operations
- `list-head ( List(T) -- T )` - Get first element
- `list-tail ( List(T) -- List(T) )` - Get rest of list
- `list-length ( List(T) -- Int )` - Count elements
- `list-reverse ( List(T) -- List(T) )` - Reverse a list
- `list-append ( List(T) List(T) -- List(T) )` - Concatenate two lists

### String Operations
- `string-concat ( String String -- String )` - Concatenate strings
- `string-length ( String -- Int )` - Get string length
- `string-equal ( String String -- Bool )` - Compare strings

The prelude is embedded at compile time, so programs work regardless of where the `cem` binary is installed.

## Status

**Phase 1 (In Progress)**: Port from Cem v1
- [ ] Copy parser/typechecker from cem
- [ ] Copy LLVM IR codegen from cem
- [ ] Implement Rust runtime with May
- [ ] Test pattern matching (the part that failed before)

See [docs/](docs/) for design documents.

## Building

Requirements:
- Rust toolchain (stable)
- Clang (for linking LLVM IR)

```bash
cargo build --release
```

Compile and run examples:
```bash
./target/release/cem compile examples/hello.cem
./hello
```

## Why May?

[May](https://github.com/Xudong-Huang/may) provides:
- **Erlang-scale concurrency** - Millions of coroutines
- **Fast context switching** - ~10ns per switch
- **Automatic yielding** - I/O operations yield cooperatively
- **Production proven** - Used in real systems
- **Memory safe** - Written in Rust

This gives us the green thread model from Cem v1, but without the C debugging nightmares.

## License

MIT

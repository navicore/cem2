# Cem2 Project Context for Claude

## Build System: Just (NOT Make, NOT pure Cargo)

**IMPORTANT**: This project uses **`just`** as the build orchestrator.

- ✅ Use `just <task>` for all build/test/lint operations
- ❌ Do NOT create Makefiles
- ❌ Do NOT suggest "cargo can do everything" - it can't for compiler projects

### Why Just?

Cem2 is a **compiler that generates LLVM IR**, not just a Rust program. The build process includes:
1. Building Rust compiler code (cargo)
2. Building Rust runtime as static library (cargo)
3. Compiling `.cem` → LLVM IR (our compiler)
4. Linking LLVM IR with runtime (clang)
5. Running integration tests on generated binaries

**Cargo alone cannot orchestrate this.** Just is the standard tool for multi-step builds.

### Common Tasks

```bash
just build          # Build compiler + runtime
just test           # Run all tests (Rust + integration)
just lint           # Run clippy
just fmt            # Format code
just ci             # Run all CI checks (same as GitHub Actions)
just compile FILE   # Compile a .cem file
```

### GitHub Actions Philosophy

**CRITICAL**: GitHub Actions workflows MUST use `just` commands exclusively.

- ✅ `.github/workflows/ci.yml` calls `just ci`
- ❌ NEVER duplicate logic in both justfile and GitHub Actions
- ❌ NEVER run `cargo` directly in CI

**Rationale**: If CI has different commands than local development:
1. Developers discover issues in CI instead of locally (slow feedback)
2. Build drift between local and CI (hard to debug)
3. Two places to maintain build logic (error-prone)

**Solution**: CI runs `just ci`, developers run `just ci` locally. Same result.

## Project Structure

```
cem2/
├── compiler/          # Rust compiler (parser, typechecker, codegen)
│   └── src/
│       ├── parser/    # Lexer + parser
│       ├── typechecker/ # Type inference, linear types
│       ├── codegen/   # LLVM IR generation
│       └── main.rs    # CLI entry point
├── runtime/           # Rust runtime (replaces C runtime from cem v1)
│   └── src/
│       ├── stack.rs   # Stack operations (dup, swap, etc.)
│       ├── io.rs      # I/O with May coroutines
│       ├── scheduler.rs # Green threads (May-based)
│       └── pattern.rs # Pattern matching (FIXED - was segfaulting in C!)
├── examples/          # .cem example programs
├── justfile           # Build orchestration (THE SOURCE OF TRUTH)
└── .github/workflows/ # CI (calls just commands only)
```

## Language: Cem (Concatenative)

Cem is a **stack-based, concatenative language** with:
- Linear types (resources used exactly once)
- Pattern matching on sum types
- Erlang-style green threads (via May coroutines)
- LLVM backend (compiles to native code)

Example:
```cem
: square ( Int -- Int )
  dup * ;

: main ( -- )
  5 square int-to-string write-line ;
```

## Architecture: Rust Compiler + Rust Runtime + LLVM

```
.cem source → Rust compiler → LLVM IR (.ll) → clang + libcem_runtime.a → standalone binary
```

**Key insight**: Same architecture as cem v1, but:
- cem v1: C runtime (4,500 lines, segfaults in pattern matching)
- cem2: Rust runtime (800 lines, memory safe, uses May for green threads)

## Runtime: May Coroutines (NOT Custom Scheduler)

- ✅ Uses [May](https://github.com/Xudong-Huang/may) library for green threads
- ✅ Erlang-style concurrency (500K+ lightweight processes)
- ✅ Work-stealing scheduler (better than cem v1's FIFO)
- ✅ Auto-yielding on I/O (no manual `scheduler_yield()`)
- ❌ NO custom assembly context switching (May handles it)
- ❌ NO manual scheduler implementation (May provides it)

**Why May?** It does everything we built in C/assembly for cem v1, but production-hardened and memory-safe.

## Development Workflow

### Making Changes

1. **Edit Rust code** (compiler or runtime)
2. **Run `just ci`** (same as CI runs)
3. **If it passes locally, CI will pass**

### Adding Features

1. Edit compiler or runtime
2. Add tests (Rust unit tests or integration tests in justfile)
3. Run `just ci` to verify
4. Commit

### Running Examples

```bash
just build                          # Build everything
just compile examples/hello.cem     # Compile example
./hello                             # Run it
```

## Common Mistakes to Avoid

1. ❌ **Suggesting "use cargo for everything"**
   - Cargo can't compile .cem files or link LLVM IR

2. ❌ **Creating Makefiles**
   - We use Just, not Make

3. ❌ **Duplicating build logic in GitHub Actions**
   - Actions must call `just ci`, not reimplement logic

4. ❌ **Suggesting custom scheduler implementation**
   - We use May library, not custom code

5. ❌ **Suggesting LuaJIT or other VMs**
   - We compile to standalone binaries via LLVM

## Testing Philosophy

- **Unit tests**: Rust code tests (`cargo test`)
- **Integration tests**: Compile .cem files and run them (in justfile)
- **CI**: Runs `just ci` which includes both

## Documentation Locations

- **This file**: Project context for AI assistants
- **README.md**: User-facing project overview
- **RUNTIME_STATUS.md**: Runtime implementation status
- **justfile**: Build task documentation (comments in recipes)

## When to Update This File

Update `.claude/README.md` when:
- Build system changes
- New tools are introduced
- Common mistakes keep happening
- Project conventions change

This file is the **source of truth** for how Claude should understand this project.

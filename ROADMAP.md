# Cem2 Roadmap

This document outlines the strategic direction and planned features for Cem2.

## Vision

**Cem2 aims to be a practical, batteries-included stack-based systems programming language** that can power the entire application stack - from embedded systems to edge computing to web services.

### Core Principles

1. **Batteries Included** - Go-inspired stdlib with networking, HTTP, and essential tools
2. **Write Once, Deploy Anywhere** - Native and WebAssembly compilation targets
3. **Green Threads First** - Efficient concurrency built into the runtime
4. **Simple & Explicit** - Stack-based semantics make control flow obvious

## Strategic Goals

### 1. Production-Ready Networking Stack
Enable building real networked applications without external dependencies.

**Rationale:** Following Go's philosophy, networking should be in the standard library. Our async green-thread runtime (May) is already perfect for high-performance servers.

**What Success Looks Like:**
```cem
# HTTP server in ~10 lines
: handle-request ( Request -- Response )
  request-path match
    "/api/users" => [ get-users respond-json ]
    "/health"    => [ "OK" respond-200 ]
    _            => [ respond-404 ]
  end ;

: main ( -- )
  8080 http-server handle-request serve ;
```

### 2. WebAssembly Compilation Target
Run Cem2 code in browsers, serverless platforms, and edge computing environments.

**Rationale:** LLVM has a mature WASM backend. Since we emit LLVM IR, we can target WASM with disciplined runtime abstraction.

**What Success Looks Like:**
```bash
# Native binary
cem compile server.cem -o server
./server  # Runs on OS with full networking

# WebAssembly
cem compile --target wasm32 app.cem -o app.wasm
# Runs in browser, Cloudflare Workers, etc.
```

**Enables:**
- Client-side web applications
- Serverless functions (Cloudflare Workers, AWS Lambda)
- Edge computing
- Embedded in other applications
- Universal code: same source, multiple targets

## Development Phases

### Phase 1: Language Foundations ✅ (Current)

**Status:** Mostly Complete

- [x] Parser and type checker
- [x] LLVM codegen
- [x] Pattern matching on algebraic data types
- [x] Green thread runtime (May-based)
- [x] Basic I/O (write_line, read_line)
- [x] Core data structures (List, Option)
- [x] Integration test framework
- [x] Regression test for critical bugs

**Recent Wins:**
- Fixed critical codegen bugs (continuation code, tail-call optimization)
- Established clean separation between examples and tests
- All 48 compiler tests + 43 runtime tests pass
- 17 integration tests verify end-to-end behavior

### Phase 2: Enhanced Standard Library

**Goal:** Expand stdlib to support common programming tasks.

**List Operations:**
- [ ] `map` - Transform list elements
- [ ] `filter` - Select elements matching predicate
- [ ] `fold` / `reduce` - Aggregate list values
- [ ] `zip` - Combine two lists
- [ ] `take` / `drop` - List slicing
- [ ] `sort` - Sorting with custom comparators

**String Operations:**
- [ ] `split` - Split string by delimiter
- [ ] `join` - Join list of strings
- [ ] `trim` - Remove whitespace
- [ ] `contains` - Substring search
- [ ] String formatting / interpolation
- [ ] Regular expressions

**Data Structures:**
- [ ] `Map(K, V)` - Hash map / dictionary
- [ ] `Set(T)` - Hash set
- [ ] `Array(T)` - Fixed-size arrays
- [ ] `Result(T, E)` - Error handling type

**File I/O:**
- [ ] `read-file` - Read entire file
- [ ] `write-file` - Write entire file
- [ ] `open-file` / `close-file` - File handles
- [ ] `read-lines` - Iterate over file lines
- [ ] Directory operations (list, create, delete)

### Phase 3: Networking & HTTP (🎯 Strategic Goal)

**Goal:** Production-ready networking stack for building real applications.

**Foundation (TCP/UDP):**
- [ ] TCP client (connect, send, receive)
- [ ] TCP server (listen, accept, concurrent connections)
- [ ] UDP sockets
- [ ] DNS resolution
- [ ] Socket options (timeout, keepalive, etc.)

**HTTP Client:**
- [ ] GET, POST, PUT, DELETE requests
- [ ] Headers and query parameters
- [ ] Request/Response types
- [ ] JSON parsing/serialization
- [ ] Connection pooling
- [ ] Timeouts and retries

**HTTP Server:**
- [ ] Request routing (path matching)
- [ ] Middleware pipeline
- [ ] Static file serving
- [ ] JSON request/response handling
- [ ] Streaming responses
- [ ] Concurrent request handling (leveraging green threads)

**Advanced Networking:**
- [ ] WebSockets (client & server)
- [ ] TLS/HTTPS support
- [ ] HTTP/2
- [ ] Server-sent events (SSE)

**Example Use Cases Enabled:**
- REST APIs and microservices
- Web scrapers and API clients
- Real-time communication (WebSockets)
- Network tools and utilities
- Production-grade web services

### Phase 4: WebAssembly Target (🎯 Strategic Goal)

**Goal:** Compile Cem2 to WebAssembly with two runtime variants.

**Compilation Pipeline:**
- [ ] WASM backend integration (`--target wasm32`)
- [ ] WASM-compatible LLVM IR generation
- [ ] WASM binary optimization

**Runtime Variants:**
- [ ] **Native Runtime:** Current May-based green threads for OS deployment
- [ ] **WASM Runtime:** Lightweight runtime for browser/serverless environments
- [ ] Conditional compilation for platform-specific features
- [ ] Shared core runtime abstractions

**WASM-Specific Features:**
- [ ] Browser API bindings (DOM, fetch, etc.)
- [ ] JavaScript interop
- [ ] Memory management for WASM linear memory
- [ ] WASI (WebAssembly System Interface) support

**Testing Strategy:**
- [ ] Integration tests run on BOTH native and WASM targets
- [ ] CI/CD runs full test suite for each target
- [ ] Platform-specific test exclusions where needed

**Deployment Targets Enabled:**
- Browser-based applications
- Cloudflare Workers / Deno Deploy
- AWS Lambda / serverless platforms
- Edge computing (Fastly Compute@Edge)
- Embedded in other applications

### Phase 5: Developer Experience

**Goal:** Make Cem2 productive and pleasant to use.

**Tooling:**
- [ ] REPL (Read-Eval-Print Loop)
- [ ] Better error messages with source locations
- [ ] Debugger integration
- [ ] LSP (Language Server Protocol) for IDE support
- [ ] Syntax highlighting for major editors
- [ ] Package manager / dependency system

**Documentation:**
- [ ] Language reference
- [ ] Tutorial series
- [ ] Cookbook (common patterns)
- [ ] API documentation
- [ ] Architecture documentation

**Project Structure:**
- [ ] Standard project layout
- [ ] Build system improvements
- [ ] Benchmark framework
- [ ] Profiling tools

### Phase 6: Advanced Language Features

**Goal:** Enhance type system and language expressiveness.

**Type System:**
- [ ] Type inference improvements
- [ ] Polymorphic recursion
- [ ] Higher-kinded types (if beneficial)
- [ ] Refinement types / contracts

**Modularity:**
- [ ] Module system
- [ ] Import/export mechanisms
- [ ] Namespacing
- [ ] Visibility controls (public/private)

**Concurrency:**
- [ ] Channels for strand communication (Go-style CSP)
- [ ] Synchronization primitives (mutexes, semaphores)
- [ ] Async/await syntax sugar (maybe)

**Metaprogramming:**
- [ ] Macros or compile-time code generation
- [ ] Generic programming improvements

## Success Metrics

### Short Term (Next 3-6 Months)
- [ ] Standard library has 50+ functions
- [ ] Can write a simple HTTP server in <20 lines
- [ ] All integration tests pass on native target

### Medium Term (6-12 Months)
- [ ] Production-quality HTTP client and server
- [ ] WASM compilation target working
- [ ] 100+ integration tests covering both targets
- [ ] First real application built with Cem2

### Long Term (12-24 Months)
- [ ] Full networking stack (TCP, HTTP, WebSockets, TLS)
- [ ] Robust WASM support with browser/serverless deployment
- [ ] LSP and good IDE integration
- [ ] Community-contributed packages/libraries
- [ ] Used in production by early adopters

## Competitive Positioning

**Compared to Go:**
- ✅ Similar batteries-included philosophy
- ✅ Green threads for concurrency
- ✅ Simple, explicit semantics
- 🎯 WASM target (Go's WASM support is improving but not primary)

**Compared to Rust:**
- 🎯 Simpler learning curve (stack-based vs. ownership)
- 🎯 Faster compile times (simpler type system)
- ✅ LLVM backend (same performance potential)
- ✅ WASM-first citizen (same as Rust)

**Unique Value Proposition:**
- **Stack-based + Green threads + WASM** = unique combination
- Explicit stack manipulation makes control flow obvious
- Concatenative style enables powerful composition
- Write once, deploy anywhere (native, web, edge, serverless)

## Non-Goals

**What We Won't Do:**
- ❌ Compete with C++ on low-level systems programming
- ❌ Build a massive framework ecosystem (keep stdlib focused)
- ❌ Support every possible platform (focus on native + WASM)
- ❌ Make the language overly complex

**Philosophy:**
- Keep the core language simple
- Batteries included, but focused batteries
- Let the ecosystem build specialized libraries
- Optimize for clarity over cleverness

## Contributing

This roadmap is a living document. Priorities may shift based on:
- Community feedback
- Real-world usage patterns
- Technical discoveries
- Resource availability

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to get involved.

## Questions / Discussion

Open questions for community input:

1. **Networking API Design:** Go-style interfaces or something more stack-native?
2. **WASM-First Features:** Should some features be WASM-only?
3. **Error Handling:** Result types, exceptions, or something else?
4. **Package Management:** Centralized repository or decentralized?

---

**Last Updated:** 2025-10-19
**Status:** Phase 1 (Foundations) complete, Phase 2 (Enhanced Stdlib) beginning

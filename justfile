# Cem2 Build System
#
# This is the SOURCE OF TRUTH for all build/test/lint operations.
# GitHub Actions calls these recipes directly - no duplication!

# Default recipe: show available commands
default:
    @just --list

# Build everything (compiler + runtime)
build: build-runtime build-compiler

# Build the Rust runtime as static library
build-runtime:
    @echo "Building runtime (May-based green threads)..."
    cargo build --release -p cem-runtime
    @echo "✅ Runtime built: target/release/libcem_runtime.a"

# Build the compiler
build-compiler:
    @echo "Building compiler..."
    cargo build --release -p cem-compiler
    @echo "✅ Compiler built: target/release/cem"

# Install the compiler to ~/.cargo/bin
install:
    cargo install --path compiler

# Run all Rust unit tests
test: build-runtime
    @echo "Running Rust unit tests..."
    cargo test --workspace --all-targets

# Run clippy on all workspace members
lint: build-runtime
    @echo "Running clippy..."
    cargo clippy --workspace --all-targets -- -D warnings

# Format all code
fmt:
    @echo "Formatting code..."
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    @echo "Checking code formatting..."
    cargo fmt --all -- --check

# Run all CI checks (same as GitHub Actions!)
# This is what developers should run before pushing
ci: fmt-check lint test build-compiler
    @echo ""
    @echo "✅ All CI checks passed!"
    @echo "   - Code formatting ✓"
    @echo "   - Clippy lints ✓"
    @echo "   - Unit tests ✓"
    @echo "   - Compiler built ✓"
    @echo ""
    @echo "Safe to push to GitHub - CI will pass."

# Clean all build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean
    rm -f *.ll *.o
    rm -f examples/*_exe
    rm -f hello hello_io echo
    @echo "✅ Clean complete"

# Development: quick format + build
dev: fmt build

# Show runtime test output (verbose)
test-runtime-verbose:
    cargo test -p cem-runtime -- --nocapture

# Show compiler test output (verbose)
test-compiler-verbose:
    cargo test -p cem-compiler -- --nocapture

# Check for outdated dependencies
outdated:
    cargo outdated --workspace

# Generate documentation
doc:
    cargo doc --workspace --no-deps --open

# Verify workspace consistency
verify-workspace:
    @echo "Verifying workspace configuration..."
    cargo tree --workspace
    @echo "✅ Workspace verified"

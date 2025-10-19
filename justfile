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

# Build all example programs (for demonstration purposes)
build-examples: build
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Building examples..."
    mkdir -p target/examples
    # Find all .cem files in examples subdirectories
    find examples -name "*.cem" -type f | while read -r file; do
        # Get category and name (e.g., examples/core-builtins/stack-operations.cem -> core-builtins-stack-operations)
        category=$(dirname "$file" | sed 's|examples/||')
        name=$(basename "$file" .cem)
        output_name="${category}-${name}"
        echo "  Compiling $category/$name..."
        target/release/cem compile "$file" -o "target/examples/$output_name"
    done
    echo "✅ Examples built in target/examples/"
    ls -lh target/examples/

# Run integration tests (compile and execute all test files)
test-integration: build
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Running integration tests..."
    mkdir -p target/integration-tests

    failed=0
    total=0

    # Find all .cem files in tests/integration
    find tests/integration -name "*.cem" -type f | while read -r file; do
        total=$((total + 1))
        category=$(dirname "$file" | sed 's|tests/integration/||')
        name=$(basename "$file" .cem)
        test_name="${category}-${name}"

        # Compile the test
        if target/release/cem compile "$file" -o "target/integration-tests/$test_name" 2>&1 | grep -q "error:"; then
            echo "  ❌ $test_name (compilation failed)"
            failed=$((failed + 1))
        else
            # Run the test and check exit code
            if "target/integration-tests/$test_name" > /dev/null 2>&1; then
                echo "  ✅ $test_name"
            else
                echo "  ❌ $test_name (runtime failed)"
                failed=$((failed + 1))
            fi
        fi
    done

    if [ $failed -eq 0 ]; then
        echo "✅ All integration tests passed!"
    else
        echo "❌ $failed integration test(s) failed"
        exit 1
    fi

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
ci: fmt-check lint test build-compiler test-integration
    @echo ""
    @echo "✅ All CI checks passed!"
    @echo "   - Code formatting ✓"
    @echo "   - Clippy lints ✓"
    @echo "   - Unit tests ✓"
    @echo "   - Compiler built ✓"
    @echo "   - Integration tests ✓"
    @echo ""
    @echo "Safe to push to GitHub - CI will pass."

# Clean all build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean
    rm -f *.ll *.o
    rm -rf target/examples target/integration-tests
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

# fits-header task runner. Run `just` to list recipes.

# Show available recipes
default:
    @just --list

# Build the crate
build:
    cargo build

# Run the test suite
test:
    cargo test

# Lint with clippy (warnings treated as errors)
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Format all sources
fmt:
    cargo fmt

# Check formatting without writing changes
fmt-check:
    cargo fmt --check

# Type/borrow-check without producing a binary
check:
    cargo check --all-targets

# Build the API docs
doc:
    cargo doc --no-deps

# Full local gate: format check, lint, test
verify: fmt-check lint test

# Remove build artifacts
clean:
    cargo clean

set shell := ["bash", "-euo", "pipefail", "-c"]

default_prefix := env_var_or_default("HOME", "") + "/.local"
prefix := default_prefix

# Display the available recipes when no recipe is specified
[private]
default:
    @just --list

# Build a debug binary
build:
    cargo build

# Build an optimized binary
release:
    cargo build --release

# Run jless on the given arguments
run *args:
    cargo run -- {{args}}

# Run the test suite, with and without the sexp feature
test *args:
    cargo test {{args}}
    cargo test --features sexp {{args}}

# Check formatting and lint every target, with and without the sexp feature
lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo clippy --all-targets --features sexp -- -D warnings

# Format the code
fmt:
    cargo fmt --all

# Run linting and tests
check: lint test

# Build a release binary from this checkout and copy it to <dest>/bin (default: ~/.local/bin)
install dest=prefix: release
    mkdir -p "{{dest}}/bin"
    install -m 755 "${CARGO_TARGET_DIR:-target}/release/jless" "{{dest}}/bin/jless"

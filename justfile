# GitCactus development commands

# Run the app
run:
    cargo run

# Build in release mode
build:
    cargo build --release

# Type-check without building
check:
    cargo check

# Format all code
fmt:
    cargo fmt

# Run clippy lints
clippy:
    cargo clippy -- -D warnings

# Run tests
test:
    cargo test

# Format + clippy + test
ci: fmt clippy test

# Clean build artifacts
clean:
    cargo clean

# Contributing to GitCactus

Thanks for your interest in contributing! GitCactus is an early-stage project and contributions are welcome.

## Getting started

1. Fork and clone the repo
2. Make sure you have Rust installed: https://rustup.rs/
3. Run `cargo check` to verify everything compiles
4. Run `cargo run` to see the app

## Development workflow

```bash
just fmt      # format code
just clippy   # lint
just test     # run tests
just ci       # all of the above
```

## Guidelines

- **Keep it simple.** Avoid unnecessary abstractions, generics, or over-engineering.
- **Safety first.** Never add code that silently mutates a user's git repository. All write operations must require explicit confirmation.
- **Idiomatic Rust.** Follow standard Rust conventions. Run `cargo fmt` and `cargo clippy` before submitting.
- **Small PRs.** Prefer focused changes over large multi-feature PRs.
- **Test what matters.** Add tests for logic, especially git integration. UI rendering tests are not expected at this stage.

## Architecture

See [docs/architecture.md](docs/architecture.md) for how the codebase is organized and why.

## What to work on

Check the README roadmap or open issues. If you want to work on something not listed, open an issue first to discuss.

## Code of conduct

Be kind, be constructive, be patient. This is a beginner-friendly project and the community should reflect that.

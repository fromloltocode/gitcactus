# Releasing GitCactus

This document describes how to cut a new release of GitCactus and how
the release flow ties into GitHub Releases and the Homebrew tap.

Nothing here is automated yet — the flow is deliberately small and
understandable for now. Automation can be added later as the project
grows.

## Versioning

GitCactus follows [Semantic Versioning](https://semver.org/):

- `MAJOR` — breaking CLI or config format changes
- `MINOR` — new features, new screens, new terminology additions
- `PATCH` — bug fixes and internal cleanups

The version is tracked in a single place: `Cargo.toml`. The
`gitcactus --version` output is generated from it via `env!("CARGO_PKG_VERSION")`.

## Pre-release checklist

Run these locally before tagging:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
./target/release/gitcactus --version
./target/release/gitcactus --help
```

All must pass with zero warnings.

## Cutting a release

1. Update `Cargo.toml`:
   ```toml
   version = "X.Y.Z"
   ```
2. Update `src/update.rs` mock changelog if you want the in-app
   Release Notes screen to reflect the new version.
3. Commit the bump:
   ```bash
   git add Cargo.toml src/update.rs
   git commit -m "chore: release vX.Y.Z"
   ```
4. Tag and push:
   ```bash
   git tag vX.Y.Z
   git push origin main --tags
   ```
5. On GitHub, create a new Release from the tag. Attach release
   notes summarising the changes since the previous tag.

## Building release binaries

For now, release binaries are not auto-built. To produce one manually:

```bash
cargo build --release
```

The binary will be at `target/release/gitcactus`. You can attach it
to the GitHub Release manually, named like
`gitcactus-vX.Y.Z-<platform>-<arch>` (e.g. `gitcactus-v0.1.0-macos-arm64`).

A future CI workflow will build these automatically on tag push —
see the **Future automation** section below.

## Homebrew release flow

GitCactus uses a custom tap:
`fromloltocode/homebrew-tap`.

Users install with:

```bash
brew install fromloltocode/tap/gitcactus
```

To update the tap after a release:

1. In this repo, compute the source tarball SHA:
   ```bash
   curl -L https://github.com/fromloltocode/gitcactus/archive/refs/tags/vX.Y.Z.tar.gz \
     | shasum -a 256
   ```
2. In the tap repo (`fromloltocode/homebrew-tap`), edit
   `Formula/gitcactus.rb`:
   - Update `url` to the new tag.
   - Update `sha256` with the value from step 1.
3. Optionally sanity-test locally:
   ```bash
   brew install --build-from-source ./Formula/gitcactus.rb
   brew test gitcactus
   brew audit --strict gitcactus
   ```
4. Commit and push the tap update:
   ```bash
   git add Formula/gitcactus.rb
   git commit -m "gitcactus X.Y.Z"
   git push origin main
   ```

A copy of the formula template lives at `Formula/gitcactus.rb` inside
this repository for reference — keep it roughly in sync with the tap.

## Future: self-update inside the app

The Check for Updates screen currently shows a static "How to Update"
notice that points users at Homebrew, GitHub Releases, and the
from-source flow. The long-term plan is:

1. Query the GitHub Releases API for the latest tag.
2. Compare with `update::VERSION`.
3. Show the result in the update screen (still passive — no auto-download).
4. Eventually, opt-in self-update that downloads a prebuilt binary
   and replaces the current executable after user confirmation.

We intentionally have not implemented the network piece yet to avoid
adding an HTTP dependency for a single feature.

## Future automation

Candidate workflows once there's demand for them:

- **`release.yml`** — on tag push, build release binaries for
  macOS (`x86_64`, `aarch64`) and Linux (`x86_64`), upload them as
  release assets.
- **Formula bump** — a small job that computes the new tarball
  sha256 and opens a PR against `fromloltocode/homebrew-tap`.

These are noted here rather than implemented so the scope of each
release stays predictable.

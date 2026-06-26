# Release Checklist

Before tagging a release, verify:

## Core
- [ ] Rust toolchain available (compatible with the project — no toolchain file enforced for private beta)
- [ ] `cargo test` — all tests pass
- [ ] `cargo clippy --all-targets -- -D warnings` — clean
- [ ] `cargo fmt --check` — clean
- [ ] `cargo check --locked` — lockfile up to date (matches Cargo.toml version)
- [ ] `Cargo.lock` committed

## Feature gates
- [ ] `cargo check --no-default-features`
- [ ] `cargo test --features openrouter-http`
- [ ] `cargo test --features groq-http`
- [ ] `cargo test --features "openrouter-http groq-http"`

## Release binary
- [ ] `cargo build --release`
- [ ] `./scripts/build-release.sh` (executable, host-aware, honors `CARGO_TARGET_DIR`)
- [ ] `CARGO_TARGET_DIR=/tmp/target-alt ./scripts/build-release.sh` (works with custom target dir)
- [ ] `./target/release/intentlayer --version`
- [ ] `./target/release/intentlayer --prompt "fix parser bug" --compiled-only`
- [ ] `sha256sums.txt` uses artifact-relative filenames (not full dist path)
- [ ] `sha256sum -c sha256sums.txt` (Linux) or `shasum -a 256 -c sha256sums.txt` (macOS) passes from inside dist version directory

## Docs
- [ ] README version/stale wording checked
- [ ] AGENTS.md and llms.txt present
- [ ] BUILD_STATUS.md latest phase documented
- [ ] CHANGELOG.md updated for this version
- [ ] LICENSE file present

## Publish
- [ ] `cargo publish --dry-run` (if publishing to crates.io)

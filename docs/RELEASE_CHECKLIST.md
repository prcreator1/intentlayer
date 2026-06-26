# Release Checklist

Before tagging a release, verify:

## Core
- [ ] `rust-toolchain.toml` present (stable channel pinned)
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
- [ ] `./scripts/build-release.sh` (executable, host-aware artifact)
- [ ] `./target/release/intentlayer --version`
- [ ] `./target/release/intentlayer --prompt "fix parser bug" --compiled-only`

## Docs
- [ ] README version/stale wording checked
- [ ] AGENTS.md and llms.txt present
- [ ] BUILD_STATUS.md latest phase documented
- [ ] CHANGELOG.md updated for this version
- [ ] LICENSE file present

## Publish
- [ ] `cargo publish --dry-run` (if publishing to crates.io)

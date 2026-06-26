# Release Checklist

Before tagging a release, verify:

## Core
- [ ] `cargo test` — all tests pass
- [ ] `cargo clippy --all-targets -- -D warnings` — clean
- [ ] `cargo fmt --check` — clean
- [ ] `cargo check --locked` — lockfile up to date

## Feature gates
- [ ] `cargo check --no-default-features`
- [ ] `cargo test --features openrouter-http`
- [ ] `cargo test --features groq-http`
- [ ] `cargo test --features "openrouter-http groq-http"`

## Release binary
- [ ] `cargo build --release`
- [ ] `./target/release/intentlayer --version`
- [ ] `./target/release/intentlayer --prompt "fix parser bug" --compiled-only`

## Docs
- [ ] README version/stale wording checked
- [ ] BUILD_STATUS.md latest phase documented
- [ ] CHANGELOG.md updated for this version
- [ ] LICENSE file present

## Publish
- [ ] `cargo publish --dry-run` (if publishing to crates.io)

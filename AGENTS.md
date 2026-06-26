# AGENTS.md
IntentLayer is a prompt compiler middleware for coding agents.
## Build
cargo build
cargo build --release
## Test
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo check --locked
cargo check --no-default-features
cargo check --features openrouter-http
cargo check --features groq-http
cargo check --features "openrouter-http groq-http"
## Usage
intentlayer --prompt "fix parser bug" --compiled-only
intentlayer --input request.json --compiled-only
Use --input for large prompts or pasted Markdown.
Use --compiled-only for downstream agent handoff.
## Safety rules
- Do not commit secrets.
- Do not commit .env with values.
- API keys must be env vars only.
- No live provider calls in normal CI.
- No default network calls.
- Do not change compiler/classifier behavior unless the task explicitly asks.

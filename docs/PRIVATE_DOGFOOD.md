# IntentLayer — Private Dogfood Guide

How to build, install, and use IntentLayer for private live testing.

## Install from source

```bash
git clone https://github.com/prcreator1/intentlayer.git
cd intentlayer

# Build release binary
cargo build --release

# Verify
./target/release/intentlayer --version
```

## Local deterministic mode

```bash
# Basic compile
./target/release/intentlayer --prompt "fix parser bug" --json

# Compiled-only (plain text for downstream agents)
./target/release/intentlayer --prompt "fix parser bug" --compiled-only

# Slash commands pass through unchanged
./target/release/intentlayer --prompt "/help" --compiled-only
```

## Live provider mode

Requires feature gate + env-backed API key.

### OpenRouter

```bash
export OPENROUTER_API_KEY="<key in local shell only>"

cargo build --release --features openrouter-http

./target/release/intentlayer \
  --prompt "Design a retry wrapper for failed HTTP requests. Keep it provider-agnostic." \
  --llm \
  --provider openrouter \
  --api-key-env OPENROUTER_API_KEY \
  --json
```

### Groq

```bash
export GROQ_API_KEY="<key in local shell only>"

cargo build --release --features groq-http

./target/release/intentlayer \
  --prompt "Design a retry wrapper for failed HTTP requests. Keep it provider-agnostic." \
  --llm \
  --provider groq \
  --api-key-env GROQ_API_KEY \
  --json
```

## Downstream agent handoff

```bash
COMPILED_PROMPT="$(./target/release/intentlayer \
  --prompt "refactor this module safely" \
  --compiled-only)"

echo "$COMPILED_PROMPT"
```

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `Error: --llm requires --provider` | Missing `--provider` flag | Add `--provider openrouter` or `--provider groq` |
| `unsupported LLM provider` | Typo in provider name | Use `openrouter` or `groq` |
| `HTTP transport is not enabled` | Missing feature gate | `cargo build --features openrouter-http` or `--features groq-http` |
| `Missing environment variable` | API key env var not set | `export OPENROUTER_API_KEY=...` or `export GROQ_API_KEY=...` |
| `llm_compile` prompt uses local fallback | Prompt not classified as llm_compile | Use architecture/planning prompts like "design the system" |
| Slash command returns unchanged | Expected — deterministic pass-through | Not an error |
| Warnings non-empty in live output | Provider fallback or invention | Check warnings field in JSON output |

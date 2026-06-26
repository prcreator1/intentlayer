# Agent Integration Guide

How to use IntentLayer as a prompt-compiler middleware with coding agents.

## Quick Start — CLI

```bash
cargo build --release
./target/release/intentlayer --prompt "fix parser bug" --compiled-only
./target/release/intentlayer --version
```

## Input Methods

### Direct prompt (small)
```bash
intentlayer --prompt "fix parser bug" --compiled-only
```

### File input (medium/large)
```bash
cat > request.json <<'JSON'
{
  "prompt": "PASTE LARGE MARKDOWN OR TASK HERE"
}
JSON

intentlayer --input request.json --compiled-only
```

### stdin JSON (wrappers)
```bash
printf '%s\n' '{"prompt":"fix parser bug"}' | intentlayer --compiled-only
```

## Large Prompt Handling

- Use `--input` for large pasted Markdown or long prompts.
- Avoid passing huge prompts via `--prompt` because shell quoting/length limits can bite.
- stdin JSON is preferred for wrappers.
- `--compiled-only` is preferred for downstream agent handoff.

```bash
intentlayer --input large-request.json --compiled-only
```

## Compiled-Only Handoff

`--compiled-only` prints only the compiled prompt as plain text — no JSON metadata.
This is the intended mode for handing off to downstream coding agents.

```bash
# Write prompt to temp file, compile, pass to downstream agent
echo '{"prompt":"fix parser bug"}' > /tmp/request.json
intentlayer --input /tmp/request.json --compiled-only
```

If `warnings` are non-empty (invented provider names), IntentLayer prints the warning
to stderr and exits with code 1 without writing to stdout. Wrappers should check the
exit code before forwarding stdout.

## Max Token Behavior

`--max-tokens` controls the **provider output budget** (default: 800), not the input
prompt length. For larger LLM rewrites, increase it:

```bash
intentlayer \
  --input large-request.json \
  --llm \
  --provider openrouter \
  --api-key-env OPENROUTER_API_KEY \
  --max-tokens 1600 \
  --json
```

## Hermes Usage

Hermes can clone and use IntentLayer if Rust is available on the host.

```bash
git clone https://github.com/prcreator1/intentlayer.git
cd intentlayer
cargo build --release
./target/release/intentlayer --version
./target/release/intentlayer --prompt "fix parser bug" --compiled-only
```

Or via the release script:

```bash
./scripts/build-release.sh
./dist/0.1.0-beta.1/intentlayer-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m) --version
```

Clarifications:

- Rust is required to build from source.
- A prebuilt release binary would not require Rust.
- IntentLayer itself makes no provider/network calls unless `--llm` + `--provider` + feature gate + env key are explicitly used.
- Node.js, Python, or other runtimes are not required for IntentLayer itself.
- Wrap IntentLayer's compiled output, not its source, in downstream agent configs.

## OpenCode / opencode-style Wrappers

IntentLayer can be wrapped by OpenCode-style systems without a specific config format.

Generic pattern:

```
user prompt
→ write prompt to temporary JSON file
→ run intentlayer --input temp.json --compiled-only
→ pass compiled prompt to downstream agent
```

### Wrapper script example

```bash
#!/usr/bin/env bash
set -euo pipefail

INPUT_FILE="${1:?usage: intentlayer-agent-wrapper <request.json>}"

COMPILED_PROMPT="$(intentlayer --input "$INPUT_FILE" --compiled-only)"

printf '%s\n' "$COMPILED_PROMPT"
```

Usage:

```bash
echo '{"prompt":"fix parser bug"}' > /tmp/request.json
./scripts/intentlayer-wrapper.sh /tmp/request.json
```

### Pipeline pattern

```bash
printf '%s\n' '{"prompt":"fix parser bug"}' | intentlayer --compiled-only
```

This is the simplest wrapper pattern — no temp files needed for small prompts.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Successful compilation |
| 1 | Invalid input, bad rules, malformed JSON, warnings emitted |

Wrappers must check exit codes. Non-zero means the compiled prompt must not be used.

## Safety

- No default network calls.
- No live provider calls in normal CI.
- API keys from env vars only — never raw keys in CLI args.
- No raw keys committed or printed.
- `--compiled-only` exits 1 if provider names were invented (warnings non-empty).

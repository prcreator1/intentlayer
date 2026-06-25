# IntentLayer

IntentLayer is a prompt-only compiler for coding agents.

**Purpose:** messy user prompt → compact, context-preserving, execution-grade prompt

Designed for: Claude Code, opencode, Cursor-style agents, Codex-style agents, Hermes, local coding agents.

## Status

Phase 005 — CLI usability. See `BUILD_STATUS.md`.

## Quick Start

```bash
# Compile a prompt directly
cargo run -- --prompt "fix this repo"

# Read prompt from a JSON file
echo '{"prompt": "fix this repo"}' > input.json
cargo run -- --input input.json

# Pipe JSON via stdin
echo '{"prompt": "fix this repo"}' | cargo run

# Compact output
cargo run -- --prompt "fix this repo" --json

# Custom rules file
cargo run -- --rules-path research/transformation_rules.json --prompt "fix this repo"
```

## CLI Reference

| Flag | Description |
|------|-------------|
| `--prompt <text>` | Compile the given prompt text directly |
| `--input <path>` | Read JSON input from a file |
| `--rules-path <path>` | Load transformation rules from a JSON file |
| `--pretty` | Pretty-printed JSON (default) |
| `--json` | Compact JSON output |
| `--version` | Print version and exit |
| `--help` | Show usage and exit |

When neither `--prompt` nor `--input` is provided, JSON is read from stdin.

## Output

```json
{
  "original_prompt": "fix this repo",
  "compiled_prompt": "Using the current repository/session context...",
  "mode": "local_compile",
  "category": "repair_debug",
  "changed": true,
  "warnings": []
}
```

## Compiler Modes

| Mode | Description |
|------|-------------|
| `pass_through` | Exact prompt unchanged (slash commands, already-good prompts) |
| `minimal_compile` | 1-15 token expansion (continue, resume, try again) |
| `local_compile` | Category-based rewrite (repair, feature, refactor, etc.) |
| `llm_compile` | Structured prompt generation (architecture, planning) — stub in v0.1 |

## Development

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

## Local Install / Build

```bash
cargo build --release
./target/release/intentlayer --prompt "fix this bug" --pretty
./target/release/intentlayer --version
```

The release binary can be called by coding agents as a local preprocessor
to transform messy prompts before execution.

### Future GitHub Releases

Release artifacts (planned):
- `intentlayer-linux-x86_64` — Linux binary
- `intentlayer-macos-x86_64` — macOS binary
- `intentlayer-windows-x86_64.exe` — Windows binary
- `sha256sums.txt` — checksum file

## Agent Integration Contract

IntentLayer is designed to be called by other coding agents as a
preprocessor. The JSON output follows a stable contract.

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Successful compilation |
| 1 | Invalid/missing input, bad rules path, malformed JSON |

### Output Fields

```json
{
  "original_prompt": "string — the raw input prompt",
  "compiled_prompt": "string — the compiled output prompt",
  "mode": "pass_through | minimal_compile | local_compile | llm_compile",
  "category": "string — classified prompt category",
  "changed": true,
  "warnings": ["string — provider-names invented by compiler"]
}
```

`compiled_prompt` equals `original_prompt` when `mode` is `pass_through`
and `changed` is `false`.

### Agent Usage

```bash
# Direct prompt
./target/release/intentlayer --prompt "fix this bug" --json

# stdin JSON
echo '{"prompt":"fix this bug"}' | ./target/release/intentlayer --json

# Input file
./target/release/intentlayer --input examples/agent_request.json --json
```

Example output (`examples/agent_response.json`):

```json
{"original_prompt":"fix this bug","compiled_prompt":"Using the current error/log/project context, identify the root cause. Apply the smallest safe fix, verify where practical, and report the cause, files changed, and checks performed.","mode":"local_compile","category":"repair_debug","changed":true,"warnings":[]}
```

### Calling Agent Checklist

1. Check exit code — non-zero means the input was rejected.
2. Read `mode` — pass_through means the original prompt was already good.
3. Read `compiled_prompt` — use this as the prompt for the downstream agent.
4. Check `warnings` — non-empty warnings mean the compiler invented provider names (e.g. "Stripe") that were not in the original prompt. Treat these as errors.

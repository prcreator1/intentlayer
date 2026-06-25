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

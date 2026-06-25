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
| `--compiled-only` | Print only compiled_prompt as plain text |
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

## LLM-Assisted Compile Path

`llm_compile` mode is reserved for complex synthesis/planning prompts
that need deeper restructuring. Real LLM calls are **not enabled yet**.
The boundary is defined in `src/llm.rs`.

**Current state:**
- Architecture/planning prompts route to `llm_compile` mode
- A deterministic local template is used as fallback (stub)
- No external API calls are made
- No API keys or secrets are required

**Future provider rules (must preserve):**
- Original context preservation
- No invention of stack choices, services, files, tools, or scope
- Return only a rewritten prompt — do not execute tasks
- Deterministic modes (`pass_through`, `minimal_compile`, `local_compile`) run before LLM

## LLM Safety Envelope

This is the safe prompt contract for future LLM-assisted compilation.
**No real LLM calls are enabled yet.**

The LLM path receives only the latest user-authored prompt plus constraints.
It is asked only to **rewrite and structure** the prompt — never to execute
tasks, modify files, run commands, or invent stack choices / providers /
services / tools / files / scope.

**The envelope includes:**
- Original user prompt
- Detected category
- Explicit rewrite-only instruction
- No-invention constraints (frameworks, providers, files, architecture)
- Preservation constraints (context, intent, project conventions)

**The envelope never includes:**
- System/developer/assistant/tool messages
- File contents (unless in the original user prompt)
- API keys, env var values, or runtime secrets

**Expected response contract:**
```json
{"compiled_prompt": "...", "warnings": []}
```

## LLM Output Parsing and Repair

Upstream model output is never trusted blindly. The parser handles:

- **Strict JSON** — exact contract match
- **Fenced JSON** — extracts from ` ```json ... ``` ` blocks
- **Prose-wrapped** — finds first JSON object in surrounding text
- **Missing warnings** — repairs to empty warnings + parser note
- **Alias keys** — `prompt` or `output` repaired to `compiled_prompt`
- **Bare text** — accepted as best-effort with warnings (if safe)
- **Invalid/empty** — falls back to a safe local prompt

No second LLM call is made during repair. No network.

## Runtime LLM Provider Config

Future LLM providers are configured at runtime. Raw API keys are read from
environment variables only — config files store env-var names, never secrets.

**Real LLM calls are not enabled yet.** This section documents the config
shape for future use.

**OpenAI-compatible provider (example):**

```toml
provider = "openai-compatible"
base_url = "https://api.openai.com/v1"
model = "gpt-4.1-mini"
api_key_env = "OPENAI_API_KEY"
timeout_seconds = 30
max_tokens = 800
temperature = 0.1
```

**Local Ollama / no-key provider (example):**

```toml
provider = "ollama"
base_url = "http://localhost:11434/v1"
model = "qwen2.5-coder"
timeout_seconds = 30
max_tokens = 800
temperature = 0.1
```

**Security rules:**
- Config stores env-var names, never raw API keys
- `api_key_env` must be a valid environment variable name (ASCII letters, digits, `_`; starts with letter or `_`)
- Invalid env-var names are rejected before lookup — errors redact the invalid value
- Credentials read at runtime via `std::env::var`
- `Debug` output redacts keys to `[REDACTED]`
- Error messages reference valid env var names only — never key values

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

### Compiled-Only Handoff

`--compiled-only` prints only the compiled prompt as plain text. This is
intended for direct handoff to downstream coding agents — they receive
a clean prompt without JSON metadata.

If `warnings` are non-empty (invented provider names), the warning is
printed to stderr and the process exits with code 1 without writing to
stdout.  This prevents wrappers from silently forwarding unsafe output.

```bash
intentlayer --prompt "fix this bug" --compiled-only
echo '{"prompt":"fix this bug"}' | intentlayer --compiled-only
```

**Mode selection:**
- **JSON mode** — for wrappers / orchestrators that need `mode`, `category`, or `warnings`
- **compiled-only mode** — for direct downstream-agent handoff
- downstream agents should consume only stdout from `--compiled-only`

### Calling Agent Checklist

1. Check exit code — non-zero means the input was rejected.
2. Read `mode` — pass_through means the original prompt was already good.
3. Read `compiled_prompt` — use this as the prompt for the downstream agent.
4. Check `warnings` — non-empty warnings mean the compiler invented provider names (e.g. "Stripe") that were not in the original prompt. Treat these as errors.

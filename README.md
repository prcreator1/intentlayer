# IntentLayer

IntentLayer is a prompt-only compiler for coding agents.

**Purpose:** messy user prompt → compact, context-preserving, execution-grade prompt

Designed for: Claude Code, opencode, Cursor-style agents, Codex-style agents, Hermes, local coding agents.

## Status

Phase 025 — dogfood and install readiness. See `BUILD_STATUS.md`.

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
| `llm_compile` | Structured prompt generation (architecture, planning) — requires --llm + provider |

## LLM-Assisted Compile Path

`llm_compile` mode is reserved for complex synthesis/planning prompts.
Default compile remains local/deterministic.

**Current state:**
- Architecture/planning prompts route to `llm_compile` mode
- Live provider calls available only with `--llm` + `--provider` + feature gate + env key
- OpenRouter and Groq providers validated end-to-end
- A deterministic local template is used when providers are unavailable
- No live API calls in normal CI

**Future provider rules (must preserve):**
- Original context preservation
- No invention of stack choices, services, files, tools, or scope
- Return only a rewritten prompt — do not execute tasks
- Deterministic modes (`pass_through`, `minimal_compile`, `local_compile`) run before LLM

## LLM Safety Envelope

This is the safe prompt contract for future LLM-assisted compilation.
**Real LLM calls require explicit opt-in (--llm, --provider, feature gate, env key).**

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

## LLM Compile Orchestration

The full LLM-assisted compile path is wired as an explicit opt-in. Default
compile behavior remains deterministic/local. **Provider calls are
enabled only with --llm, --provider, feature gate, and env-backed API key.**

Orchestration flow:
1. Build safety envelope (Phase 014) with secret redaction
2. Respect local secret passthrough (bypasses provider entirely)
3. Call provider trait (mock only for now)
4. Parse/repair/fallback provider output (Phase 015)
5. Return compiled prompt with all warnings preserved

Provider output is never trusted directly — it goes through the parser.

## OpenRouter Provider Adapter

OpenRouter support is added as an explicit provider adapter implementing
`LlmProvider`. Default compile remains local/deterministic.

- API keys are read from environment through runtime config only
- API keys are never stored, printed, or committed
- Provider receives the redacted safety envelope (Phase 014)
- Structured output JSON schema requested for `compiled_prompt` + `warnings`
- `provider.require_parameters` enabled for correct routing
- Provider output still goes through IntentLayer parser and invention guard
- No live API calls unless explicitly configured

## CLI LLM Mode

LLM-assisted compilation is an explicit opt-in. Default mode remains local.

```bash
# Local (default)
intentlayer --prompt "fix parser bug"
intentlayer --prompt "fix parser bug" --compiled-only

# LLM mode (requires openrouter-http feature + runtime API key)
intentlayer --prompt "fix parser bug" --llm --provider openrouter --api-key-env OPENROUTER_API_KEY
intentlayer --prompt "fix parser bug" --llm --provider openrouter --api-key-env OPENROUTER_API_KEY --compiled-only
intentlayer --prompt "fix parser bug" --llm --provider groq --api-key-env GROQ_API_KEY --compiled-only
```

- `--llm` enables LLM mode
- `--provider openrouter` selects OpenRouter
- `--api-key-env` specifies env var name — never a raw key
- OpenRouter requires `openrouter-http` feature gate
- API key must come from env — no raw key CLI arg
- `--compiled-only` works with LLM mode

## Live OpenRouter Smoke Test

Manual only — never runs in normal CI.

```bash
export OPENROUTER_API_KEY="<your-openrouter-api-key>"
INTENTLAYER_RUN_LIVE_SMOKE=1 cargo test --features openrouter-http -- --ignored
```

- Requires `openrouter-http` feature
- Requires `OPENROUTER_API_KEY` env var
- API key is read from env only — never printed or committed
- Verifies deterministic bypass and real llm_compile call
- Fails if provider fallback occurs

## Dogfood / Install

See [Private Dogfood Guide](docs/PRIVATE_DOGFOOD.md) and [Checklist](docs/DOGFOOD_CHECKLIST.md).

## Feature-Gated OpenRouter HTTP Transport

Real OpenRouter HTTP transport exists only behind the `openrouter-http`
feature. The feature is disabled by default.

- Normal compile remains deterministic/local
- Tests do not make live network calls
- API keys come from environment/runtime config only
- API keys are never stored, printed, committed, or included in errors
- HTTP errors are sanitized (status codes only, no URL/body/headers)
- Provider output still goes through parser and invention guard

## Groq Provider

Groq is supported as an explicit opt-in provider. Requires `groq-http` feature.

```bash
intentlayer --prompt "design retry wrapper" --llm --provider groq --api-key-env GROQ_API_KEY
```

- Base URL: `https://api.groq.com/openai/v1`
- Uses `max_completion_tokens` (not `max_tokens`)
- Does not send unsupported fields (`logprobs`, `top_logprobs`, `response_format`)
- Default model: `llama-3.3-70b-versatile`

## Runtime LLM Provider Config

Future LLM providers are configured at runtime. Raw API keys are read from
environment variables only — config files store env-var names, never secrets.

**Live LLM calls require explicit opt-in.** This section documents the config
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

# Dogfood Checklist

Before merging or claiming success, verify these commands work.

## Local (no API key needed)

- [ ] `./target/release/intentlayer --version` prints version
- [ ] `./target/release/intentlayer --prompt "fix parser bug" --json` exits 0
- [ ] `./target/release/intentlayer --prompt "fix parser bug" --compiled-only` returns plain text (no JSON)
- [ ] `./target/release/intentlayer --prompt "/help" --compiled-only` returns `/help`
- [ ] `./target/release/intentlayer --prompt "/help" --llm --provider openrouter --compiled-only` returns `/help` (no live call)

## OpenRouter (requires env key + feature)

- [ ] `cargo build --release --features openrouter-http`
- [ ] Live call returns valid JSON with compiled_prompt + warnings
- [ ] `warnings` is empty for successful call
- [ ] No API key in stdout/stderr
- [ ] No Authorization/Bearer in stdout/stderr
- [ ] Fallback warning means failed dogfood

## Groq (requires env key + feature)

- [ ] `cargo build --release --features groq-http`
- [ ] Live call returns valid JSON with compiled_prompt + warnings
- [ ] `warnings` is empty for successful call
- [ ] No API key in stdout/stderr
- [ ] No Authorization/Bearer in stdout/stderr
- [ ] Fallback warning means failed dogfood

## Exit codes

- [ ] Valid input exits 0
- [ ] Invalid input exits 1
- [ ] Missing API key exits 1 (only for llm_compile prompts)
- [ ] Slash command passes through without needing API key

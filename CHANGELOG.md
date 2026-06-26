# Changelog

## 0.1.0-beta.1 (unreleased — private beta)

### Compiler
- Prompt classification with 16+ categories
- 4 compiler modes: pass_through, minimal_compile, local_compile, llm_compile
- 100-record seed benchmark + 50-record generalization benchmark
- 100% seed category accuracy, 96% generalization category accuracy

### CLI
- `--prompt`, `--input`, `--rules-path`, `--compiled-only`, `--json`
- `--llm`, `--provider`, `--api-key-env`, `--model`, `--base-url`
- `--version`, `--help`
- stdin JSON fallback

### LLM Providers
- OpenRouter provider with structured output
- Groq provider (OpenAI-compatible)
- Shared OpenAI-compatible provider core
- Provider registry for centralized metadata
- Feature gates: `openrouter-http`, `groq-http`

### Safety
- Secret redaction before LLM envelopes
- Local secret passthrough (marker + explicit opt-in)
- Invention guard on all output paths
- Sanitized HTTP errors (status codes only)
- API keys from env only — never committed/printed

### Infrastructure
- GitHub Actions CI (default + all feature combinations)
- Benchmark test runner (mode, category, token cap checks)
- Live smoke tests (manual, env-gated)
- Dogfood documentation and checklist

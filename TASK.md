# TASK.md — IntentLayer Build

## Current Phase

Phase 001: Research, patch, and project bootstrap.

## What Has Been Done

- Corpus mining research complete (6 sources, 436K+ records indexed)
- Source audit, taxonomy, failure patterns, good/reference patterns extracted
- 18 transformation rules defined
- 100 benchmark JSONL records created
- 4 compiler modes defined
- 16 never-do rules established

## What Needs Building (Next Actions)

### Compiler Implementation
- Prompt category classifier (regex + LLM fallback)
- pass_through router (exact unchanged only)
- minimal_compile expansion (1-15 tokens for tiny commands)
- local_compile rule engine (template-based rewrite)
- llm_compile structured prompt generator
- Invention guard (blocklist for provider names)

### Testing
- Benchmark test runner against vibe_prompt_bench.draft.jsonl
- Assert must_preserve items present in output
- Assert must_not_invent items absent from output
- Token budget enforcement
- Mode routing correctness

### Infrastructure
- CI (GitHub Actions) for benchmark tests
- Coverage reporting
- Published npm/PyPI package (future)

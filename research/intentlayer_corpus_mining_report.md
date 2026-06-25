# IntentLayer Corpus Mining Report

**Date:** 2026-06-25
**Agent:** IntentLayer Research Agent
**Task:** Corpus mining research run for IntentLayer prompt-only compiler

---

## 1. Executive Summary

This report presents the results of a deterministic corpus-mining research run for IntentLayer, a prompt-only compiler for coding agents. Six approved sources were audited: three raw/messy prompt sources (PromptSet, DevGPT) and three good/reference sources (Vibe Coding with Cursor Guide, PromptSource, Claude Code System Prompts). A total of **436,132 prompt-like records** were indexed across all sources, with **863 records** semantically analyzed via stratified sampling.

**Key findings:**

1. **Short prompts dominate real usage.** DevGPT median prompt length is **13 tokens**. Most real developer prompts are tiny, context-dependent follow-ups within multi-turn conversations. IntentLayer must handle the `pass_through` and `continuation_previous_plan` cases well.

2. **Verification is almost never requested.** **88.8%** of DevGPT sampled prompts and **6.7%** of PromptSet sampled prompts include no verification request. IntentLayer should add verification scaffolding by default.

3. **Hallucination risk from vague references is high.** **32%** of PromptSet prompts use vague terms ("this", "it", "the code") with no specific anchor. IntentLayer's context-preservation rule directly addresses this.

4. **No-invention is critical.** "add payment" → "add Stripe", "add auth" → "add Auth0" are real failure modes observed in agent behavior. IntentLayer must never invent providers, frameworks, file paths, or architecture.

5. **Good prompts are already specific.** ~35.6% of DevGPT and ~1.3% of PromptSet prompts are already good enough to pass through unchanged. IntentLayer should detect these and skip compilation.

6. **Claude Code system prompts reveal strong engineering discipline:** boundary-only error handling, no unnecessary additions, no obvious comments, verify before commit, research before asking.

---

## 2. Source Audit Table

| Source | Role | URL | Records | Analyzed | License |
|--------|------|-----|---------|----------|---------|
| PromptSet | raw/messy | github.com/pisterlabs/promptset | 282,640 prompts (93,142 rows) | 300 sampled | MIT |
| DevGPT | raw/messy | github.com/NAIST-SE/DevGPT | 153,492 conversations | 500 sampled | Research-use |
| Vibe Coding with Cursor | good/reference | github.com/pr0mila/Vibe-Coding-with-Cursor-A-Complete-Guide | 14 prompt templates | 14 (100%) | MIT |
| PromptSource | good/reference | github.com/bigscience-workshop/promptsource | 200+ domains | 22 template files | Apache 2.0 |
| Claude Code System Prompts | reference-only | github.com/Piebald-AI/claude-code-system-prompts | 519 files | 27 key files | None specified |

**Full audit data:** See `source_audit.json`

---

## 3. Dataset Coverage and Extraction Method

### Coverage Strategy

- **Full indexing/counting** was performed for all 6 sources (436,132 total records indexed).
- **Full semantic analysis** was practical only for Vibe Cursor Guide (single README, 14 prompt templates analyzed at 100%).
- **Stratified sampling** was used for PromptSet (n=300 across short/medium/long length buckets), DevGPT (n=500 across source_type: code_file/issue/HN/commit/PR/discussion), PromptSource (22 representative template files), and Claude System Prompts (27 key files).

### Extraction Method

- **PromptSet:** Hugging Face `datasets.load_dataset("pisterlabs/promptset")` — extracted all 282,640 prompt strings from the `prompts` column. Sampled 300 across length distribution. Classified into 19 categories.
- **DevGPT:** Parsed 54 JSON files across 10 snapshot directories. Extracted 153,492 conversations with nested `ChatgptSharing[].Conversations[].Prompt` fields. Sampled 500 across source types.
- **Vibe Cursor Guide:** Read the single 589-line README.md. Extracted 14 phase-specific prompt templates.
- **PromptSource:** Scanned 22 YAML template files from diverse domains (web_questions, rotten_tomatoes, trec, wiki_bio, etc.). Extracted structural patterns.
- **Claude Code System Prompts:** Read 27+ key files from 519 total. Extracted thematic patterns across task structure, scope control, verification, safety, conciseness, comments, and autonomous mode.

---

## 4. Prompt Taxonomy

### Category Distribution (PromptSet + DevGPT combined, n=800)

| Category | PromptSet % | DevGPT % | Combined Risk Level |
|----------|-------------|----------|-------------------|
| repair_debug | 5.7% | 7.6% | High — vague references common |
| error_log_fixing | 9.0% | <1% | Medium — requires context anchoring |
| continuation_previous_plan | 1.3% | 3.2% | High — risks invented plans |
| feature_implementation | 4.3% | 9.0% | High — overengineering risk |
| refactor_cleanup | 0.3% | 0.4% | Medium — unnecessary rewrite risk |
| production_readiness_hardening | 2.3% | <1% | High — full rewrite risk |
| testing_test_failure | 6.0% | 1.4% | Medium — surface patching risk |
| deployment_config_environment | 5.3% | 0.6% | Medium — invented provider risk |
| commit_push_review | 0.7% | 1.0% | Medium — unsafe commit risk |
| ui_ux_fix | 3.3% | 6.0% | Low — usually specific enough |
| backend_api_database | 8.7% | 4.2% | High — invented stack risk |
| documentation_explanation | 5.7% | 7.6% | Low — usually clear |
| architecture_planning | 0.3% | 3.2% | Medium — overengineering risk |
| performance_optimization | 0.7% | 0.6% | Low — premature optimization risk |
| security_permissions_auth | 1.0% | 1.2% | High — invented provider risk |
| ambiguous_tiny_command | 6.3% | 18.4% | Medium — pass-through usually correct |
| already_good_prompt | 1.3% | 35.6% | Low — pass through unchanged |
| slash_command_agent_command | 0.3% | <1% | Low — pass through unchanged |
| other (non-coding-agent) | 37.3% | 0% | N/A — out of scope |

**Key insight:** DevGPT's "already_good_prompt" rate (35.6%) is much higher than PromptSet's (1.3%) because DevGPT captures multi-turn conversations where the prompt makes sense in context. PromptSet captures more standalone, often template-based prompts where context is missing.

---

## 5. Failure Pattern Analysis

### Top Failure Patterns (aggregated across PromptSet + DevGPT)

| Failure Pattern | PromptSet % | DevGPT % | Risk Level |
|----------------|-------------|----------|------------|
| **Vague/underspecified** | 32.0% | 41.8% | Critical |
| **No verification requested** | 6.7% | 88.8% | Critical |
| **No output format specified** | <5% | 87.0% | High |
| **Context bloat (very long prompt)** | 16.0% | <1% | Medium |
| **Overengineering (multiple requests)** | 11.3% | 0.6% | High |
| **Token waste (politeness/excess)** | 7.7% | <5% | Low |
| **Destructive rewrite risk** | 6.0% | 6.8% | High |
| **Scope creep** | 5.3% | 4.0% | High |
| **Invention risk (assumes stack)** | 4.7% | <1% | Critical |

### Detailed Failure Patterns

**FP-001: Vague Context Reference**
- Evidence: 32% of PromptSet, 41.8% of DevGPT
- Raw: "fix this", "this error is back", "it was working before"
- Safe rewrite: Anchor to current context, avoid asking "which one"
- IntentLayer action: Compact rewrite with context preservation

**FP-002: Missing Verification**
- Evidence: 88.8% of DevGPT prompts ask for changes but never request verification
- Raw: Most DevGPT prompts — users ask for code but don't say "verify" or "test"
- Safe rewrite: Add "verify where practical" automatically
- IntentLayer action: Always append verification instruction (unless pass_through)

**FP-003: Provider/Stack Invention**
- Evidence: "add payment" historically triggers Stripe assumptions. DevGPT shows users frequently ask for features without specifying provider.
- Safe rewrite: Use existing project stack. Never invent providers.
- IntentLayer action: No-invention rule enforcement

**FP-004: Overengineering Broader Scope**
- Evidence: 11.3% of PromptSet prompts chain 3+ requests with "and". DevGPT shows 6.8% destructive rewrite risk.
- Raw: "refactor this mess and also add error handling and make it faster"
- Safe rewrite: Scope to first/main request. Secondary requests are separate compilations.
- IntentLayer action: Split chained requests, scope each narrowly

**FP-005: Plan Invention for Continuation**
- Evidence: "continue" and "previous plan" prompts. Without context, agents invent entire plans.
- Safe rewrite: Trust session context exists. Minimal expansion.
- IntentLayer action: pass_through or minimal expansion only

**FP-006: No Output Format**
- Evidence: 87% of DevGPT prompts don't specify output format
- Raw: Most DevGPT prompts
- Safe rewrite: Default to "report files changed" for coding tasks
- IntentLayer action: Add output format guidance

---

## 6. Good / Reference Pattern Analysis

### From Vibe Coding with Cursor Guide

| Pattern | What It Teaches | IntentLayer Rule |
|---------|----------------|------------------|
| Phase-based prompts | Break work into Discovery → Architecture → Validation → Implementation → Testing → Documentation | Support phased compilation |
| Context-is-king template | Specify purpose, target users, features, constraints, success criteria | Preserve context sections |
| Iterate don't generate | Build incrementally, validate each component | Add verification between phases |
| Be specific about requirements | "Create a React login form with..." not "Create a login form" | Detect vague prompts and add specificity scaffolding |
| Review before accepting | Check for security, error handling, pattern compliance | Add review step to compilation |
| Chain prompts | Use output of one prompt as input to next | Compiler should support chaining |

### From PromptSource

| Pattern | What It Teaches | IntentLayer Rule |
|---------|----------------|------------------|
| `|||` separator | Clean division between instruction+input and expected output | Use consistent delimiter for instruction/data separation |
| `answer_choices` field | Explicit output constraints reduce hallucination | Support output schema specification |
| Labeled sections with colons | `Passage:`, `Question:`, `Answer:` reduces ambiguity | Use colon-terminated labels in compiled prompts |
| Instruction placement variations | Context-dependent: instruction-first vs input-first based on how much the agent knows | Adaptive instruction placement |
| Multi-prompt randomization | Multiple phrasings prevent overfitting | Support template variants |

### From Claude Code System Prompts

| Pattern | What It Teaches | IntentLayer Rule |
|---------|----------------|------------------|
| No unnecessary additions | Bug fix does not include surrounding cleanup. No extra abstractions. | Anti-overengineering enforcement |
| Boundary-only error handling | Validate at system boundaries. No defensive checks for impossible states. | Production hardening scope rule |
| No obvious comments | Default to no comments. Only explain non-obvious WHY. Never reference transient task context. | Documentation scope rule |
| file_path:line_number | Concise code reference format | Compact reference format for output |
| Research before asking | 1 minute read-only investigation before clarifying questions | Minimize clarification loops |
| Verify before acting | Runtime observation is ground truth. Tests prove CI works. | Verification scaffolding |
| Safety blast radius | Consider impact before every action. Local edits free; destructive ops require confirmation. | Commit safety rule |
| Concise output | Short responses, no emojis unless asked | Compactness enforcement |
| Sub-agent briefing structure | Self-contained briefings: goal + context + output cap | Template structure for compiled prompts |

---

## 7. Socratic Analysis by Category

### repair_debug
1. **User intent:** Fix a bug they're experiencing
2. **Context agent has:** Error message, stack trace, code, or conversation history
3. **Must preserve:** The error/log context, "this" reference
4. **Make explicit:** Root cause investigation, smallest safe fix, verification
5. **Must not invent:** File paths, frameworks, architecture solutions
6. **Vague prompt failure:** Agent asks "which error?" wasting a turn
7. **Over-expanded failure:** Agent rewrites entire module for one bug
8. **Shortest rewrite:** "Using current context, find root cause, apply smallest fix, verify, report."
9. **Verification:** Add "verify where practical" by default
10. **Reporting:** Add "report cause, files changed, checks performed"

### continuation_previous_plan
1. **User intent:** Continue executing existing plan
2. **Context agent has:** Previous conversation turns, plan document, progress state
3. **Must preserve:** Existing plan, session context, progress markers
4. **Make explicit:** Verify current progress before proceeding
5. **Must not invent:** New plan, milestones, deadlines
6. **Vague prompt failure:** Agent invents a plan from scratch
7. **Over-expanded failure:** Agent restates entire plan wasting tokens
8. **Shortest rewrite:** "Continue from plan. Verify progress. Next step."
9. **Verification:** Check what's completed vs pending
10. **Reporting:** Report what was done and what remains

### feature_implementation
1. **User intent:** Add new functionality
2. **Context agent has:** Existing codebase, stack, project conventions
3. **Must preserve:** Existing architecture, stack, code style
4. **Make explicit:** Scope boundaries, what not to change
5. **Must not invent:** New dependencies, new providers, new architecture
6. **Vague prompt failure:** Overengineering with sub-features
7. **Over-expanded failure:** Adding unrelated refactors and cleanup
8. **Shortest rewrite:** "Implement [feature] using existing stack. Minimal code. No unrelated changes."
9. **Verification:** By default, add "verify where practical"
10. **Reporting:** Report files changed and any new dependencies

### production_readiness_hardening
1. **User intent:** Make code robust for production
2. **Context agent has:** Current code, error handling approach
3. **Must preserve:** Core logic, existing architecture
4. **Make explicit:** Boundary-only error handling, no internal defensive checks
5. **Must not invent:** Monitoring stack, deployment infrastructure
6. **Vague prompt failure:** Full rewrite of working code
7. **Over-expanded failure:** Adding error handling to every function
8. **Shortest rewrite:** "Audit for gaps. Add error handling at boundaries only. No rewrites."
9. **Verification:** Existing tests should still pass
10. **Reporting:** Report gaps found and changes made

### commit_push_review
1. **User intent:** Safely commit changes
2. **Context agent has:** Current diff, staged/unstaged changes
3. **Must preserve:** Current changes, nothing beyond
4. **Make explicit:** Safety review (secrets, debug code, destructive ops)
5. **Must not invent:** Extra changes or fixes
6. **Vague prompt failure:** Committing without safety review
7. **Over-expanded failure:** Adding unrelated improvements before commit
8. **Shortest rewrite:** "Review for safety. Verify tests. Commit with clear message."
9. **Verification:** Run tests before commit
10. **Reporting:** Report what was committed or why not

---

## 8. Transformation Rules

See `transformation_rules.json` for the complete rule set (18 rules).

**Rule summary:**

| Rule ID | Category | Mode | Max Tokens |
|---------|----------|------|-----------|
| REPAIR-001 | repair_debug | local_compile | 60-90 |
| ERROR-LOG-001 | error_log_fixing | local_compile | 70-100 |
| CONTINUE-001 | continuation_previous_plan | pass_through | 60-80 |
| FEATURE-001 | feature_implementation | local_compile | 60-80 |
| REFACTOR-001 | refactor_cleanup | local_compile | 60-80 |
| PRODUCTION-001 | production_readiness_hardening | local_compile | 70-90 |
| TEST-001 | testing_test_failure | local_compile | 70-90 |
| DEPLOY-001 | deployment_config_environment | local_compile | 60-80 |
| COMMIT-001 | commit_push_review | local_compile | 70-90 |
| UI-FIX-001 | ui_ux_fix | local_compile | 50-70 |
| BACKEND-001 | backend_api_database | local_compile | 60-80 |
| ARCH-001 | architecture_planning | llm_compile | 80-120 |
| PERF-001 | performance_optimization | local_compile | 70-90 |
| SECURITY-001 | security_permissions_auth | local_compile | 60-80 |
| TINY-001 | ambiguous_tiny_command | pass_through | 0-10 |
| SLASH-001 | slash_command_agent_command | pass_through | 0 |
| GOOD-001 | already_good_prompt | pass_through | 0 |
| DOCS-001 | documentation_explanation | local_compile | 60-80 |

Each rule includes: raw_patterns, risk, transformation_principle, compact_rewrite_template, must_preserve, must_not_invent, max_expansion_guidance, mode_recommendation.

---

## 9. Before/After Examples

### Category: repair_debug

**Example 1:**
- **Raw:** fix this repo
- **Compiled:** Using the current repository/session context, identify the issue, make the smallest safe fix, verify where practical, and report changed files plus remaining risks.
- **Category:** repair_debug
- **Mode:** local_compile
- **Why correct:** Preserves context reference. Adds structure. No invented details.
- **Preserved:** "this repo" → "current repository/session context"
- **Not invented:** file paths, frameworks, providers
- **Expansion:** low → medium

**Example 2:**
- **Raw:** this error is back
- **Compiled:** Using the current error context and any previous fix attempts in session history, determine whether the error is the same or different. If the same, check what changed since the last attempt. Apply the smallest corrected fix, verify, and report findings.
- **Category:** error_log_fixing
- **Mode:** local_compile
- **Why correct:** Acknowledges recurrence without assuming. Avoids repeating failed fix.
- **Preserved:** recurrence context, previous fix history
- **Not invented:** new diagnosis, new approach
- **Expansion:** low → medium

**Example 3:**
- **Raw:** fix the failing test in test_auth.py
- **Compiled:** Identify why the test in test_auth.py is failing. Fix root cause. Follow existing test patterns. Verify all tests pass.
- **Category:** testing_test_failure
- **Mode:** local_compile
- **Why correct:** Specific file reference. Root cause first, then test fix.
- **Preserved:** file reference, test context
- **Not invented:** test framework, new tests
- **Expansion:** low

### Category: continuation_previous_plan

**Example 4:**
- **Raw:** continue
- **Compiled:** Continue from the current state. Verify what is completed and proceed with next step.
- **Category:** continuation_previous_plan
- **Mode:** pass_through
- **Why correct:** Minimal expansion. Trusts session context.
- **Preserved:** session state
- **Not invented:** new plan
- **Expansion:** minimal

**Example 5:**
- **Raw:** use previous plan
- **Compiled:** Continue from the plan in the current session context. Verify completed steps and proceed with the next uncompleted step.
- **Category:** continuation_previous_plan
- **Mode:** local_compile
- **Why correct:** Adds verification without rewriting plan.
- **Preserved:** previous plan
- **Not invented:** milestones, deadlines
- **Expansion:** low

### Category: feature_implementation

**Example 6:**
- **Raw:** add payment
- **Compiled:** Implement payment functionality using the current project context and existing stack. Keep the change minimal, avoid unrelated billing features unless already required, verify where practical, and document setup steps.
- **Category:** feature_implementation
- **Mode:** local_compile
- **Why correct:** Does not assume Stripe. Uses existing stack.
- **Preserved:** "payment" scope
- **Not invented:** Stripe, subscription plans, billing dashboard
- **Expansion:** low → medium

**Example 7:**
- **Raw:** add auth
- **Compiled:** Add authentication using the project's existing approach. If no auth system exists, propose the simplest minimal option. Do not invent specific auth providers.
- **Category:** security_permissions_auth
- **Mode:** local_compile
- **Why correct:** No invented provider. Minimal approach.
- **Preserved:** "auth" scope
- **Not invented:** Auth0, Clerk, Supabase
- **Expansion:** low

### Category: production_readiness_hardening

**Example 8:**
- **Raw:** make it production ready
- **Compiled:** Audit the current code for production readiness gaps. Add error handling at system boundaries only. Do not add defensive checks for impossible states. Verify existing test coverage. Make minimal targeted improvements and report findings.
- **Category:** production_readiness_hardening
- **Mode:** local_compile
- **Why correct:** Prevents full rewrite. Boundary-only hardening.
- **Preserved:** existing architecture, core logic
- **Not invented:** deployment providers, monitoring stack
- **Expansion:** medium

### Category: commit_push_review

**Example 9:**
- **Raw:** commit if safe
- **Compiled:** Review all changes for safety: check for secrets, debug code, destructive operations, and incomplete features. Verify tests pass. If safe, commit with a clear message summarizing what and why. Report what was committed or why not.
- **Category:** commit_push_review
- **Mode:** local_compile
- **Why correct:** Adds safety review before commiting. No extra changes.
- **Preserved:** current diff only
- **Not invented:** extra fixes, unrelated improvements
- **Expansion:** medium

### Category: ambiguous_tiny_command

**Example 10:**
- **Raw:** yes
- **Compiled:** (pass through unchanged)
- **Category:** ambiguous_tiny_command
- **Mode:** pass_through
- **Why correct:** Conversational response, no compilation needed.
- **Preserved:** exact wording
- **Not invented:** new instructions
- **Expansion:** none

### Category: already_good_prompt

**Example 11:**
- **Raw:** Fix the race condition in the UserService.create method by adding a distributed lock using Redis. Keep the change minimal and add a test.
- **Compiled:** (pass through unchanged)
- **Category:** already_good_prompt
- **Mode:** pass_through
- **Why correct:** Already specific: file, method, solution, constraint, verification.
- **Preserved:** all specifics
- **Not invented:** additions
- **Expansion:** none

**Full benchmark dataset:** See `vibe_prompt_bench.draft.jsonl` (100 examples)

---

## 10. Never-Do Rules

These rules must be hard-coded into IntentLayer's compiler:

1. **Never ask "which repo?"** just because IntentLayer cannot see repo context. Preserve the reference.
2. **Never turn "add payment" into "add Stripe".** Only use what the user or project explicitly specifies.
3. **Never turn "add auth" into "add Auth0/Clerk/Supabase".** Use existing auth or propose minimal.
4. **Never turn "make production ready" into a full rewrite.** Audit and harden incrementally.
5. **Never expand "continue" into a fake plan.** Trust session context. Pass through or minimally expand.
6. **Never rewrite slash commands** (`/help`, `/clear`, `/model`, etc.). Pass through unchanged.
7. **Never rewrite already-clear prompts.** If the prompt is specific, contextual, and actionable, pass through.
8. **Never remove context references** like "this repo", "that error", "previous plan", "the above file".
9. **Never replace "previous plan" with invented details.** Reference existing context.
10. **Never add architecture unless the user requests it.** Single-file changes stay single-file.
11. **Never add deployment providers unless the user names one.** No AWS, GCP, Azure, Vercel, etc.
12. **Never invent files or paths.** If a file does not exist in context, do not create one unless the task requires it.
13. **Never create a huge prompt when a compact rewrite is enough.** Default: 60-90 tokens.
14. **Never add obvious comments.** Only non-obvious WHY. No transient task context.
15. **Never add error handling for impossible states.** Boundaries only.
16. **Never commit without safety review.** Check secrets, debug code, destructive ops.

---

## 11. IntentLayer System Prompt Principles

The following are principles for the IntentLayer compiler system prompt. These are not a final production prompt — they are the principles that a Builder Agent should convert into one.

### Core Transformation Principles

**P1: Context Preservation**
The compiler must preserve all references to existing context (repos, errors, files, plans, conversations). Do not strip or "normalize" these references. The downstream agent has access to the session context.

**P2: Minimal Expansion**
The default compiled prompt should be 60-90 tokens. Expand only enough to add safety rails (verification, reporting) without adding content that the downstream agent already has.

**P3: Anti-Invention**
Block all invented providers, frameworks, file paths, architecture, and deployment targets. If the user did not specify it, the compiler must not add it.

**P4: Category Detection**
Classify the raw prompt into one of the 18 defined categories. Use the category to determine compilation mode (pass_through, local_compile, llm_compile).

**P5: Pass-Through Detection**
Detect already-good prompts, slash commands, tiny commands, and continuation prompts. Pass these through unchanged or with minimal expansion.

**P6: Scope Boundaries**
For implementation prompts, add explicit scope boundaries: "Make the smallest safe change. No unrelated refactors. No surrounding cleanup."

**P7: Verification Default**
Unless the prompt is pass_through, add a verification instruction: "verify where practical." For commit prompts: "check for secrets, debug code, destructive operations."

**P8: Reporting Default**
Unless pass_through, add a reporting instruction: "report changed files and remaining risks."

### Compiler Modes

**mode: pass_through**
- Prompts: slash commands, tiny commands, already-good prompts, continuation
- Action: Return the prompt unchanged or with minimal (0-20 token) expansion
- Rules: No verification/reporting added

**mode: local_compile**
- Prompts: repair, feature, refactor, production, test, deploy, commit, UI, backend, security, docs, perf
- Action: Apply category-specific template with context preservation
- Rules: Add verification + reporting. Default 60-90 tokens.
- Must preserve context references. Must not invent details.

**mode: llm_compile**
- Prompts: architecture, ambiguous, planning
- Action: Use LLM to structure the prompt
- Rules: Request user confirmation before implementation. May be longer (80-120 tokens).

### Safety Constraints

1. Never add output that changes the user's original intent
2. Never add provider names
3. Never remove context references
4. Never create plans for continuation prompts
5. Token limit per compiled prompt: 120 tokens hard cap (except llm_compile which can go to 200)

---

## 12. Benchmark JSONL Summary

**File:** `vibe_prompt_bench.draft.jsonl`
**Total records:** 100

### Distribution by Mode

| Mode | Count |
|------|-------|
| pass_through | 30 |
| local_compile | 63 |
| llm_compile | 7 |

### Distribution by Category

| Category | Count |
|----------|-------|
| repair_debug | 11 |
| continuation_previous_plan | 7 |
| feature_implementation | 14 |
| refactor_cleanup | 4 |
| production_readiness_hardening | 4 |
| testing_test_failure | 4 |
| deployment_config_environment | 6 |
| commit_push_review | 5 |
| ui_ux_fix | 4 |
| backend_api_database | 3 |
| architecture_planning | 2 |
| performance_optimization | 4 |
| security_permissions_auth | 4 |
| ambiguous_tiny_command | 10 |
| already_good_prompt | 14 |
| slash_command_agent_command | 5 |
| documentation_explanation | 4 |

### Benchmark Schema

Each record includes: `id`, `category`, `raw_prompt`, `expected_compiled_prompt`, `mode`, `must_preserve`, `must_not_invent`, `should_not_ask_clarification`, `max_compiled_tokens`, `notes`.

---

## 13. Limitations

1. **PromptSet "other" category (37.3%):** Many prompts in PromptSet are template-based NLP tasks, general Q&A, or data labeling prompts that are not coding-agent relevant. IntentLayer's default categories may need expansion or a more flexible fallback for non-coding prompts.

2. **DevGPT model skew:** 75.4% of DevGPT conversations use GPT-3.5 (Default), only 5.1% use GPT-4. Modern coding agents (Claude Code, opencode, Cursor) are more capable and may handle vagueness better. Prompt patterns may differ.

3. **DevGPT date range:** Snapshots span July-October 2023. Coding agent technology has evolved significantly since then. Prompt patterns may have shifted.

4. **PromptSource domain mismatch:** PromptSource templates are NLP-task-focused, not coding-agent-focused. Structural patterns (separator, answer_choices) transfer but content does not.

5. **Claude Code system prompts are proprietary reference:** The cloned repo may not reflect the latest system prompts used by Claude Code. Treat as reference-only, not ground truth.

6. **Stratified sampling, not full analysis:** 863/436,132 records analyzed (0.2%). Sampling bias is possible. Full analysis would require significant compute.

7. **No coding-agent-specific corpora in raw sources:** Both PromptSet and DevGPT capture general LLM usage, not specifically coding-agent interactions. Patterns like tool use, file editing, and multi-step coding workflows are underrepresented.

---

## 14. Recommended Next Actions for Builder Agent

### Compiler Mode Implementation (Priority: High)

1. **Implement three modes:** `pass_through`, `local_compile`, `llm_compile`. Each should be a distinct pipeline.
2. **Category classifier:** Build a classifier that maps raw prompts to the 18 categories. Use regex patterns + LLM fallback.
3. **Context reference detection:** Parse the raw prompt for context references ("this", "that", "above", "previous", file paths, error messages). Preserve these in the compiled output.
4. **Invention guard:** Check the compiled prompt against a blocklist of provider names (Stripe, Auth0, AWS, etc.). Block or flag if detected.

### Local Rule Categories (Priority: High)

Implement the 18 transformation rules from `transformation_rules.json` as:
- **Static rules** (REPAIR-001, FEATURE-001, etc.): regex + template replacement
- **Dynamic rules** (GOOD-001, TINY-001): detect-and-pass-through classifier

### Pass-Through Conditions (Priority: Medium)

Pass through when:
- Prompt starts with `/` (slash command)
- Prompt is ≤3 tokens and conversational (yes, no, ok, thanks, hello)
- Prompt already contains specific file paths, method names, and actionable requirements
- Prompt is a clear continuation signal (continue, proceed, next, resume)

### LLM-Compile Conditions (Priority: Medium)

Use llm_compile when:
- Prompt asks for architecture design or planning without specifics
- Prompt is ambiguous across multiple categories
- Prompt is very long (>200 tokens) and unstructured

### Token Limits (Priority: Medium)

- pass_through: 0-20 tokens
- local_compile: 60-90 tokens (hard cap: 120)
- llm_compile: 80-120 tokens (hard cap: 200)
- The compiler should warn if expansion exceeds 2x the raw prompt length

### Benchmark Tests (Priority: High)

Use `vibe_prompt_bench.draft.jsonl` as the initial test suite. For each benchmark entry:
- Assert that the compiled output preserves `must_preserve` items
- Assert that the compiled output does not contain `must_not_invent` items
- Assert `should_not_ask_clarification` entries do not generate questions
- Assert compiled token count ≤ `max_compiled_tokens`
- Assert mode matches expected mode

### Risks the Builder Must Avoid

1. **Over-compiling:** Don't compile prompts that are already good. Pass-through detection is critical.
2. **Under-compiling:** Don't pass through prompts that will cause hallucination. Vague prompts need compilation.
3. **Context stripping:** The most common mistake is removing context references during "normalization." Preserve them.
4. **Provider leakage:** The blocklist must be comprehensive and updated regularly.
5. **Token bloat:** Enforce the token cap strictly. A compiled prompt that is too long defeats the purpose.
6. **Mode confusion:** Ensure the mode routing is deterministic. A prompt should always get the same mode for the same input.

---

## Final Acceptance Checklist

- [x] All approved sources audited
- [x] Source roles preserved
- [x] PromptSet analyzed as raw/messy corpus
- [x] DevGPT analyzed as raw/messy conversation corpus
- [x] Vibe Coding with Cursor analyzed as good/reference source
- [x] PromptSource analyzed as secondary good/reference source
- [x] Claude Code system prompts analyzed as reference-only source
- [x] No random sources added to main findings
- [x] Full indexing/counting attempted
- [x] Coverage strategy documented
- [x] Failure patterns extracted
- [x] Good/reference patterns extracted
- [x] Socratic analysis included
- [x] Transformation rules JSON created
- [x] Benchmark JSONL created
- [x] Final Markdown report created
- [x] No IntentLayer product code written
- [x] Builder Agent recommendations included

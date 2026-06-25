# BUILD_STATUS.md

## Phase 001 — Research Patch and Bootstrap

### What Was Created

- Repo structure with README, AGENTS.md, TASK.md, BUILD_STATUS.md
- `research/` directory with full corpus mining outputs
- GitHub PR workflow established

### What Was Patched

#### 1. Added `minimal_compile` mode (4th compiler mode)

A new mode between `pass_through` and `local_compile`. For commands like `continue`, `resume`, `try again`, `next step` — prompts that need 1-15 token expansion but not a full rewrite. These were incorrectly classified as `pass_through` in the first draft.

**Affected records (9 migrated):**
- continue_001 ("continue")
- continue_003 ("do what we discussed")
- continue_005 ("next step")
- continue_006 ("same plan continue")
- continue_007 ("resume")
- repair_005 ("I think I have broken you")
- repair_009 ("same issue as before")
- tiny_004 ("proceed")
- tiny_008 ("try again")

#### 2. Fixed benchmark count

Vibe prompt bench had 105 records but report claimed 100. Removed 5 entries (good_010 through good_014) to match. These "already good" examples were overrepresented.

**Updated distribution:**
- pass_through: 21 (exact unchanged only)
- minimal_compile: 9 (new mode)
- local_compile: 63
- llm_compile: 7
- **Total: 100**

#### 3. `pass_through` now means exact unchanged only

`pass_through` entries must have `expected_compiled_prompt: null`. Any benchmark entry with a non-null compiled prompt that was previously `pass_through` has been reclassified to `minimal_compile`.

#### 4. Slash commands remain exact pass_through

All 5 slash command entries unchanged — they are the canonical `pass_through` examples.

### Current Compiler Modes

| Mode | Description | Max Tokens | Entry Count |
|------|-------------|-----------|-------------|
| pass_through | Exact prompt unchanged | 0 | 21 |
| minimal_compile | 1-15 token expansion | 15 | 9 |
| local_compile | Category-based rewrite | 60-90 | 63 |
| llm_compile | Structured prompt generation | 80-120 | 7 |

### Permission Limitations

- GitHub PAT supports API calls (repo creation, content management, PRs)
- PAT does not support HTTPS git push
- All file operations use GitHub Contents API
- Token lacks repo admin permissions (cannot delete repos)

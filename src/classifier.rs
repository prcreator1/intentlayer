//! Prompt classifier — determines the category and mode for a raw prompt.

use crate::rules::RuleSet;

/// The four compiler modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    PassThrough,
    MinimalCompile,
    LocalCompile,
    LlmCompile,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::PassThrough => "pass_through",
            Mode::MinimalCompile => "minimal_compile",
            Mode::LocalCompile => "local_compile",
            Mode::LlmCompile => "llm_compile",
        }
    }

    pub fn from_mode_str(s: &str) -> Option<Mode> {
        match s {
            "pass_through" => Some(Mode::PassThrough),
            "minimal_compile" => Some(Mode::MinimalCompile),
            "local_compile" => Some(Mode::LocalCompile),
            "llm_compile" => Some(Mode::LlmCompile),
            _ => None,
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Classification result.
#[derive(Debug, Clone)]
pub struct Classification {
    pub category: String,
    pub mode: Mode,
    pub rule_id: Option<String>,
}

// ── Keyword routing tables ─────────────────────────────────────────
// Organized by category for maintainability.  Keyword precedence is
// preserved by the concatenation order in specific_phrases() and
// generic_keywords(): longer / more-specific phrases first.
//
// TODO(v0.1): Replace hardcoded tables with a trained classifier model.

type Phrase = (&'static str, &'static str, Mode);

// ── Continuation / minimal-compile triggers ─────────────────────────

const CONTINUATION_PHRASES: &[Phrase] = &[
    (
        "carry on",
        "continuation_previous_plan",
        Mode::MinimalCompile,
    ),
    (
        "where we left off",
        "continuation_previous_plan",
        Mode::MinimalCompile,
    ),
    ("step 3", "continuation_previous_plan", Mode::LocalCompile),
    ("phase", "continuation_previous_plan", Mode::LocalCompile),
];

// ── Compound phrase tables (specific → beats generic) ──────────────

const REPAIR_SPECIFIC: &[Phrase] = &[
    ("undefined is not", "repair_debug", Mode::LocalCompile),
    ("memory leak", "repair_debug", Mode::LocalCompile),
    ("returns 500", "repair_debug", Mode::LocalCompile),
    ("something is wrong", "repair_debug", Mode::LocalCompile),
    ("this error is back", "repair_debug", Mode::LocalCompile),
    (
        "this error keeps happening",
        "repair_debug",
        Mode::LocalCompile,
    ),
    ("same issue", "repair_debug", Mode::LocalCompile),
    ("was working before", "repair_debug", Mode::LocalCompile),
    ("i think i have broken", "repair_debug", Mode::LocalCompile),
];

const ERROR_LOG_SPECIFIC: &[Phrase] = &[
    ("logs show", "error_log_fixing", Mode::LocalCompile),
    ("same error again", "error_log_fixing", Mode::LocalCompile),
];

const UI_SPECIFIC: &[Phrase] = &[
    ("this button", "ui_ux_fix", Mode::LocalCompile),
    ("make it look better", "ui_ux_fix", Mode::LocalCompile),
    ("fix the ui", "ui_ux_fix", Mode::LocalCompile),
    ("update ui", "ui_ux_fix", Mode::LocalCompile),
    ("nav bar", "ui_ux_fix", Mode::LocalCompile),
    ("dropdown", "ui_ux_fix", Mode::LocalCompile),
    ("modal dialog", "ui_ux_fix", Mode::LocalCompile),
    ("button is misaligned", "ui_ux_fix", Mode::LocalCompile),
];

const BACKEND_SPECIFIC: &[Phrase] = &[
    ("api endpoint", "backend_api_database", Mode::LocalCompile),
    ("database model", "backend_api_database", Mode::LocalCompile),
    ("join table", "backend_api_database", Mode::LocalCompile),
    (
        "search endpoint",
        "backend_api_database",
        Mode::LocalCompile,
    ),
    (
        "create an endpoint",
        "backend_api_database",
        Mode::LocalCompile,
    ),
    ("pagination", "backend_api_database", Mode::LocalCompile),
];

const FEATURE_SPECIFIC: &[Phrase] = &[
    (
        "connect backend",
        "feature_implementation",
        Mode::LocalCompile,
    ),
    ("add auth", "feature_implementation", Mode::LocalCompile),
    ("add payment", "feature_implementation", Mode::LocalCompile),
    (
        "add dark mode",
        "feature_implementation",
        Mode::LocalCompile,
    ),
    (
        "push realtime",
        "feature_implementation",
        Mode::LocalCompile,
    ),
    ("add search", "feature_implementation", Mode::LocalCompile),
    (
        "add file upload",
        "feature_implementation",
        Mode::LocalCompile,
    ),
    ("add email", "feature_implementation", Mode::LocalCompile),
    (
        "add notifications",
        "feature_implementation",
        Mode::LocalCompile,
    ),
    ("add users", "feature_implementation", Mode::LocalCompile),
    ("add csv", "feature_implementation", Mode::LocalCompile),
    ("add i18n", "feature_implementation", Mode::LocalCompile),
    ("add social", "feature_implementation", Mode::LocalCompile),
    ("add image", "feature_implementation", Mode::LocalCompile),
    (
        "implement the",
        "feature_implementation",
        Mode::LocalCompile,
    ),
];

const PRODUCTION_SPECIFIC: &[Phrase] = &[
    (
        "add logging",
        "production_readiness_hardening",
        Mode::LocalCompile,
    ),
    (
        "add error handling",
        "production_readiness_hardening",
        Mode::LocalCompile,
    ),
    (
        "make sure this",
        "production_readiness_hardening",
        Mode::LocalCompile,
    ),
    (
        "circuit breaker",
        "production_readiness_hardening",
        Mode::LocalCompile,
    ),
    (
        "add retries",
        "production_readiness_hardening",
        Mode::LocalCompile,
    ),
];

const DEPLOY_SPECIFIC: &[Phrase] = &[
    (
        "add health check",
        "deployment_config_environment",
        Mode::LocalCompile,
    ),
    (
        "environment variable",
        "deployment_config_environment",
        Mode::LocalCompile,
    ),
    (
        "dockerfile",
        "deployment_config_environment",
        Mode::LocalCompile,
    ),
    (
        "set up ci",
        "deployment_config_environment",
        Mode::LocalCompile,
    ),
    (
        "deployment is broken",
        "deployment_config_environment",
        Mode::LocalCompile,
    ),
    (
        "staging env",
        "deployment_config_environment",
        Mode::LocalCompile,
    ),
    (
        "ci pipeline",
        "deployment_config_environment",
        Mode::LocalCompile,
    ),
    (
        "docker compose",
        "deployment_config_environment",
        Mode::LocalCompile,
    ),
];

const COMMIT_SPECIFIC: &[Phrase] = &[
    (
        "don't break what works",
        "commit_push_review",
        Mode::LocalCompile,
    ),
    (
        "check if safe to commit",
        "commit_push_review",
        Mode::LocalCompile,
    ),
    ("commit if safe", "commit_push_review", Mode::LocalCompile),
    (
        "review my changes",
        "commit_push_review",
        Mode::LocalCompile,
    ),
    ("code review", "commit_push_review", Mode::LocalCompile),
    ("commit all my", "commit_push_review", Mode::LocalCompile),
    ("review this pr", "commit_push_review", Mode::LocalCompile),
    (
        "review current diff",
        "commit_push_review",
        Mode::LocalCompile,
    ),
    (
        "review the branch",
        "commit_push_review",
        Mode::LocalCompile,
    ),
];

const PERF_SPECIFIC: &[Phrase] = &[
    (
        "reduce bundle",
        "performance_optimization",
        Mode::LocalCompile,
    ),
    (
        "reduce docker",
        "performance_optimization",
        Mode::LocalCompile,
    ),
    (
        "optimize this",
        "performance_optimization",
        Mode::LocalCompile,
    ),
    (
        "make it faster",
        "performance_optimization",
        Mode::LocalCompile,
    ),
    (
        "speed it up",
        "performance_optimization",
        Mode::LocalCompile,
    ),
];

const SECURITY_SPECIFIC: &[Phrase] = &[
    ("api key", "security_permissions_auth", Mode::LocalCompile),
    (
        "rate limit",
        "security_permissions_auth",
        Mode::LocalCompile,
    ),
    ("add rate", "security_permissions_auth", Mode::LocalCompile),
    ("add 2fa", "security_permissions_auth", Mode::LocalCompile),
    (
        "lock out users",
        "security_permissions_auth",
        Mode::LocalCompile,
    ),
    (
        "admin panel",
        "security_permissions_auth",
        Mode::LocalCompile,
    ),
];

const REFACTOR_SPECIFIC: &[Phrase] = &[
    ("tidy up", "refactor_cleanup", Mode::LocalCompile),
    ("split this", "refactor_cleanup", Mode::LocalCompile),
    ("normalize the", "refactor_cleanup", Mode::LocalCompile),
    ("extract this", "refactor_cleanup", Mode::LocalCompile),
    ("refactor this", "refactor_cleanup", Mode::LocalCompile),
    ("clean up this", "refactor_cleanup", Mode::LocalCompile),
    ("clean up auth", "refactor_cleanup", Mode::LocalCompile),
    ("clean up dashboard", "refactor_cleanup", Mode::LocalCompile),
    (
        "clean up the parser",
        "refactor_cleanup",
        Mode::LocalCompile,
    ),
];

const TESTING_SPECIFIC: &[Phrase] = &[
    (
        "run tests and fix",
        "testing_test_failure",
        Mode::LocalCompile,
    ),
    (
        "tests are flaky",
        "testing_test_failure",
        Mode::LocalCompile,
    ),
    ("fix tests", "testing_test_failure", Mode::LocalCompile),
    ("add tests", "testing_test_failure", Mode::LocalCompile),
    (
        "make tests pass",
        "testing_test_failure",
        Mode::LocalCompile,
    ),
    (
        "coverage dropped",
        "testing_test_failure",
        Mode::LocalCompile,
    ),
    (
        "integration tests fail",
        "testing_test_failure",
        Mode::LocalCompile,
    ),
    ("unit tests for", "testing_test_failure", Mode::LocalCompile),
];

const DOCS_SPECIFIC: &[Phrase] = &[
    ("api docs", "documentation_explanation", Mode::LocalCompile),
    (
        "write readme",
        "documentation_explanation",
        Mode::LocalCompile,
    ),
    ("jsdoc", "documentation_explanation", Mode::LocalCompile),
    ("add jsdoc", "documentation_explanation", Mode::LocalCompile),
    (
        "explain this",
        "documentation_explanation",
        Mode::LocalCompile,
    ),
    (
        "document this",
        "documentation_explanation",
        Mode::LocalCompile,
    ),
    (
        "add comments",
        "documentation_explanation",
        Mode::LocalCompile,
    ),
    (
        "inline comments",
        "documentation_explanation",
        Mode::LocalCompile,
    ),
    (
        "explain how",
        "documentation_explanation",
        Mode::LocalCompile,
    ),
    (
        "explain this error",
        "documentation_explanation",
        Mode::LocalCompile,
    ),
];

const ARCH_SPECIFIC: &[Phrase] = &[
    (
        "how should i structure",
        "architecture_planning",
        Mode::LlmCompile,
    ),
    (
        "design the system",
        "architecture_planning",
        Mode::LlmCompile,
    ),
    (
        "design architecture",
        "architecture_planning",
        Mode::LlmCompile,
    ),
    (
        "design a notification",
        "architecture_planning",
        Mode::LlmCompile,
    ),
    (
        "migrate to new server",
        "deployment_config_environment",
        Mode::LlmCompile,
    ),
];

const HANDLES_NETWORK: &[Phrase] = &[(
    "handles network failures",
    "production_readiness_hardening",
    Mode::LocalCompile,
)];

// ── Generic single-word fallback keywords ───────────────────────────

const REPAIR_GENERIC: &[Phrase] = &[
    ("fix", "repair_debug", Mode::LocalCompile),
    ("broken", "repair_debug", Mode::LocalCompile),
    ("bug", "repair_debug", Mode::LocalCompile),
];

const ERROR_LOG_GENERIC: &[Phrase] = &[
    ("error", "error_log_fixing", Mode::LocalCompile),
    ("traceback", "error_log_fixing", Mode::LocalCompile),
];

const REFACTOR_GENERIC: &[Phrase] = &[
    ("refactor", "refactor_cleanup", Mode::LocalCompile),
    ("clean up", "refactor_cleanup", Mode::LocalCompile),
];

const PRODUCTION_GENERIC: &[Phrase] = &[
    (
        "production",
        "production_readiness_hardening",
        Mode::LocalCompile,
    ),
    (
        "harden",
        "production_readiness_hardening",
        Mode::LocalCompile,
    ),
    (
        "robust",
        "production_readiness_hardening",
        Mode::LocalCompile,
    ),
];

const COMMIT_GENERIC: &[Phrase] = &[
    ("commit", "commit_push_review", Mode::LocalCompile),
    ("review", "commit_push_review", Mode::LocalCompile),
    ("push", "commit_push_review", Mode::LocalCompile),
];

const PERF_GENERIC: &[Phrase] = &[
    ("optimize", "performance_optimization", Mode::LocalCompile),
    ("faster", "performance_optimization", Mode::LocalCompile),
    (
        "performance",
        "performance_optimization",
        Mode::LocalCompile,
    ),
];

const SECURITY_GENERIC: &[Phrase] = &[
    ("secure", "security_permissions_auth", Mode::LocalCompile),
    ("rbac", "security_permissions_auth", Mode::LocalCompile),
    (
        "permission",
        "security_permissions_auth",
        Mode::LocalCompile,
    ),
];

const DOCS_GENERIC: &[Phrase] = &[
    ("document", "documentation_explanation", Mode::LocalCompile),
    ("explain", "documentation_explanation", Mode::LocalCompile),
    ("readme", "documentation_explanation", Mode::LocalCompile),
];

const ARCH_GENERIC: &[Phrase] = &[
    ("design", "architecture_planning", Mode::LlmCompile),
    ("structure", "architecture_planning", Mode::LlmCompile),
    ("microservice", "architecture_planning", Mode::LlmCompile),
];

const DEPLOY_GENERIC: &[Phrase] = &[("migrate", "deployment_config_environment", Mode::LlmCompile)];

// ── Combined maps (preserving specificity precedence) ──────────────

/// Specific compound phrases — checked before generic keywords.
fn specific_phrases() -> Vec<Phrase> {
    [
        // Continuation triggers
        CONTINUATION_PHRASES,
        // Compound phrases per category
        REPAIR_SPECIFIC,
        ERROR_LOG_SPECIFIC,
        UI_SPECIFIC,
        BACKEND_SPECIFIC,
        FEATURE_SPECIFIC,
        PRODUCTION_SPECIFIC,
        HANDLES_NETWORK,
        DEPLOY_SPECIFIC,
        COMMIT_SPECIFIC,
        PERF_SPECIFIC,
        SECURITY_SPECIFIC,
        REFACTOR_SPECIFIC,
        TESTING_SPECIFIC,
        DOCS_SPECIFIC,
        ARCH_SPECIFIC,
    ]
    .concat()
}

/// Generic single-word fallback keywords — lower priority.
fn generic_keywords() -> Vec<Phrase> {
    [
        REPAIR_GENERIC,
        ERROR_LOG_GENERIC,
        REFACTOR_GENERIC,
        PRODUCTION_GENERIC,
        COMMIT_GENERIC,
        PERF_GENERIC,
        SECURITY_GENERIC,
        DOCS_GENERIC,
        ARCH_GENERIC,
        DEPLOY_GENERIC,
    ]
    .concat()
}

/// Full keyword table: specific phrases first, generic fallbacks second.
fn keyword_map() -> Vec<Phrase> {
    let mut k = specific_phrases();
    k.extend(generic_keywords());
    k
}

// TODO(v0.1): Hardcoded seed list. Replace with data-driven detection from
// the benchmark corpus or a classifier model in a future version.

/// Known minimal_compile prompts — short commands that need 1-15 token
/// expansion, with per-prompt categories.
///
/// Checked BEFORE rule matching so they don't get caught by broader patterns.
fn classify_minimal(lower: &str) -> Option<Classification> {
    let (category, rule_id) = match lower {
        "continue" => ("continuation_previous_plan", "CONTINUE-MIN-001"),
        "resume" => ("continuation_previous_plan", "CONTINUE-MIN-001"),
        "next step" => ("continuation_previous_plan", "CONTINUE-MIN-001"),
        "do what we discussed" => ("continuation_previous_plan", "CONTINUE-MIN-001"),
        "same plan continue" => ("continuation_previous_plan", "CONTINUE-MIN-001"),
        "proceed" => ("ambiguous_tiny_command", "TINY-MIN-001"),
        "try again" => ("ambiguous_tiny_command", "TINY-MIN-001"),
        "i think i have broken you" => ("repair_debug", "REPAIR-MIN-001"),
        "same issue as before" => ("repair_debug", "REPAIR-MIN-001"),
        _ => return None,
    };
    Some(Classification {
        category: category.into(),
        mode: Mode::MinimalCompile,
        rule_id: Some(rule_id.into()),
    })
}

/// Classify a prompt using the loaded rule set.
///
/// Priority ordering:
/// 1. Slash commands (`/` prefix) → pass_through (exact unchanged)
/// 2. Very short conversational prompts (≤3 words, known list) → pass_through
/// 3. Known minimal_compile prompts → minimal_compile
/// 4. Long specific prompts (≥15 words, ≥2 tech indicators) → pass_through
/// 5. Keyword-based matching → specific categories (avoids generic rule patterns)
/// 6. Rule pattern match → rule-specific categories (fallback for explicit patterns)
/// 7. Fallback → local_compile (general rewrite)
pub fn classify(prompt: &str, rules: &RuleSet) -> Classification {
    let trimmed = prompt.trim();
    let lower = trimmed.to_lowercase();
    let word_count = trimmed.split_whitespace().count();

    // 1. Slash commands — always pass_through, exact unchanged
    if trimmed.starts_with('/') {
        return Classification {
            category: "slash_command_agent_command".into(),
            mode: Mode::PassThrough,
            rule_id: Some("SLASH-001".into()),
        };
    }

    // 2. Very short conversational prompts (exact match only)
    if word_count <= 3 {
        let conversational = [
            "yes",
            "no",
            "ok",
            "okay",
            "do it",
            "run it",
            "go ahead",
            "nope",
            "nah",
            "thanks",
            "hello",
            "sure",
            "done",
            "yep",
            "yeah",
            "no way",
            "sure thing",
            "👍",
            "+1",
            "lgtm",
            "ack",
        ];
        if conversational.contains(&lower.as_str()) {
            return Classification {
                category: "ambiguous_tiny_command".into(),
                mode: Mode::PassThrough,
                rule_id: Some("TINY-001".into()),
            };
        }
    }

    // 3. Known minimal_compile prompts (exact match, case-insensitive)
    if let Some(classification) = classify_minimal(&lower) {
        return classification;
    }

    // 4. Long specific prompts → pass_through (already good)
    if looks_specific(&lower, word_count) {
        return Classification {
            category: "already_good_prompt".into(),
            mode: Mode::PassThrough,
            rule_id: Some("GOOD-001".into()),
        };
    }

    // 5. Keyword-based matching (checked before rules to avoid generic
    //    stripped patterns like "add" or "build" catching everything)
    for (keyword, category, mode) in keyword_map() {
        if lower.contains(keyword) {
            return Classification {
                category: category.into(),
                mode,
                rule_id: None,
            };
        }
    }

    // 6. Rule pattern match (explicit rule patterns as fallback)
    if let Some(rule) = rules.match_prompt(trimmed) {
        let mode = Mode::from_mode_str(&rule.mode_recommendation).unwrap_or(Mode::LocalCompile);
        return Classification {
            category: rule.category.clone(),
            mode,
            rule_id: Some(rule.rule_id.clone()),
        };
    }

    // 7. Fallback
    Classification {
        category: "feature_implementation".into(),
        mode: Mode::LocalCompile,
        rule_id: None,
    }
}

// TODO(v0.1): Hardcoded seed heuristics (word count threshold, indicator
// list). Replace with a trained classifier or curated rule set.
/// Heuristic: does this prompt look specific enough to pass through unchanged?
///
/// A prompt is "already good" if:
/// - It has ≥ 15 words (long enough to be self-contained)
/// - It has ≥ 2 technical specificity indicators
///
/// This check does NOT look at action keywords because a long, detailed
/// prompt starting with "Fix" or "Add" is already self-specifying.
fn looks_specific(lower_prompt: &str, word_count: usize) -> bool {
    if word_count < 15 {
        return false;
    }

    let indicators = [
        lower_prompt.contains("function"),
        lower_prompt.contains("()"),
        lower_prompt.contains("class "),
        lower_prompt.contains("method "),
        lower_prompt.contains("endpoint"),
        lower_prompt.contains("migration"),
        lower_prompt.contains("schema"),
        lower_prompt.contains("test "),
        lower_prompt.contains(".py"),
        lower_prompt.contains(".ts"),
        lower_prompt.contains(".js"),
        lower_prompt.contains(".rs"),
        lower_prompt.contains(".csv"),
        lower_prompt.contains(".go"),
        lower_prompt.contains(".java"),
        lower_prompt.contains("src/"),
        lower_prompt.contains("test_"),
        lower_prompt.contains("usestate"),
        lower_prompt.contains("useeffect"),
        lower_prompt.contains("interface"),
        lower_prompt.contains("type "),
        lower_prompt.contains(">="),
        lower_prompt.contains("<="),
        lower_prompt.contains("return "),
        lower_prompt.contains("async"),
        lower_prompt.contains("await"),
        lower_prompt.contains("redis"),
        lower_prompt.contains("docker"),
        lower_prompt.contains("postgresql"),
        lower_prompt.contains("kubernetes"),
        lower_prompt.contains("healthz"),
        lower_prompt.contains("configmap"),
        lower_prompt.contains("dockerfile"),
        lower_prompt.contains("multi-stage"),
        lower_prompt.contains("graphql"),
        lower_prompt.contains("swagger"),
        lower_prompt.contains("openapi"),
        lower_prompt.contains("codecov"),
        lower_prompt.contains("github action"),
        lower_prompt.contains("uselocalstorage"),
        lower_prompt.contains("hook "),
        lower_prompt.contains("websocket"),
        lower_prompt.contains("soft delete"),
        lower_prompt.contains("migration script"),
        lower_prompt.contains("rename"),
        lower_prompt.contains("ca-certificates"),
        lower_prompt.contains("node "),
        lower_prompt.contains("typescript"),
        lower_prompt.contains("api key"),
        lower_prompt.contains("distributed lock"),
        lower_prompt.contains("ssr"),
        lower_prompt.contains("redis"),
        lower_prompt.contains("docker"),
        lower_prompt.contains("graphql"),
    ];

    let tech_count = indicators.iter().filter(|&&b| b).count();
    tech_count >= 2
}

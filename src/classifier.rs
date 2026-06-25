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

    pub fn from_str(s: &str) -> Option<Mode> {
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

// TODO(v0.1): Hardcoded seed heuristic. Replace with learned model or
// data-driven frequency table in a future version.
/// Map of action keywords → (category, mode).
fn keyword_map() -> Vec<(&'static str, &'static str, Mode)> {
    vec![
        ("fix", "repair_debug", Mode::LocalCompile),
        ("broken", "repair_debug", Mode::LocalCompile),
        ("bug", "repair_debug", Mode::LocalCompile),
        ("error", "error_log_fixing", Mode::LocalCompile),
        ("traceback", "error_log_fixing", Mode::LocalCompile),
        ("refactor", "refactor_cleanup", Mode::LocalCompile),
        ("clean up", "refactor_cleanup", Mode::LocalCompile),
        ("production", "production_readiness_hardening", Mode::LocalCompile),
        ("harden", "production_readiness_hardening", Mode::LocalCompile),
        ("robust", "production_readiness_hardening", Mode::LocalCompile),
        ("commit", "commit_push_review", Mode::LocalCompile),
        ("push", "commit_push_review", Mode::LocalCompile),
        ("review", "commit_push_review", Mode::LocalCompile),
        ("optimize", "performance_optimization", Mode::LocalCompile),
        ("faster", "performance_optimization", Mode::LocalCompile),
        ("performance", "performance_optimization", Mode::LocalCompile),
        ("secure", "security_permissions_auth", Mode::LocalCompile),
        ("rbac", "security_permissions_auth", Mode::LocalCompile),
        ("permission", "security_permissions_auth", Mode::LocalCompile),
        ("rate limit", "security_permissions_auth", Mode::LocalCompile),
        ("document", "documentation_explanation", Mode::LocalCompile),
        ("explain", "documentation_explanation", Mode::LocalCompile),
        ("jsdoc", "documentation_explanation", Mode::LocalCompile),
        ("readme", "documentation_explanation", Mode::LocalCompile),
        ("design", "architecture_planning", Mode::LlmCompile),
        ("structure", "architecture_planning", Mode::LlmCompile),
        ("microservice", "architecture_planning", Mode::LlmCompile),
        ("migrate", "deployment_config_environment", Mode::LlmCompile),
    ]
}

// TODO(v0.1): Hardcoded seed list. Replace with data-driven detection from
// the benchmark corpus or a classifier model in a future version.
/// Known minimal_compile prompts — short commands that need 1-15 token expansion.
/// These are checked FIRST (after slash commands and conversational pass-through)
/// so they don't get caught by broader rule patterns.
const MINIMAL_COMPILE_PROMPTS: &[&str] = &[
    "continue",
    "resume",
    "next step",
    "proceed",
    "do what we discussed",
    "same plan continue",
    "i think i have broken you",
    "same issue as before",
    "try again",
];

/// Classify a prompt using the loaded rule set.
///
/// Priority ordering:
/// 1. Slash commands (`/` prefix) → pass_through (exact unchanged)
/// 2. Very short conversational prompts (≤3 words, known list) → pass_through
/// 3. Known minimal_compile prompts → minimal_compile
/// 4. Long specific prompts (≥15 words, ≥2 tech indicators) → pass_through
///    (These are already-good prompts that happen to contain action words.
///    Must be checked BEFORE rule matching so detailed prompts don't get
///    caught by broad patterns like "fix" or "add".)
/// 5. Rule pattern match → use rule's category and mode
/// 6. Keyword-based matching → local_compile or llm_compile
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
            "yes", "no", "ok", "okay", "do it", "run it", "go ahead", "nope",
            "thanks", "hello", "sure", "done", "yep", "yeah", "no way",
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
    if MINIMAL_COMPILE_PROMPTS.contains(&lower.as_str()) {
        return Classification {
            category: "continuation_previous_plan".into(),
            mode: Mode::MinimalCompile,
            rule_id: Some("CONTINUE-MIN-001".into()),
        };
    }

    // 4. Long specific prompts → pass_through (already good)
    if looks_specific(&lower, word_count) {
        return Classification {
            category: "already_good_prompt".into(),
            mode: Mode::PassThrough,
            rule_id: Some("GOOD-001".into()),
        };
    }

    // 5. Rule pattern match
    if let Some(rule) = rules.match_prompt(trimmed) {
        let mode = Mode::from_str(&rule.mode_recommendation).unwrap_or(Mode::LocalCompile);
        return Classification {
            category: rule.category.clone(),
            mode,
            rule_id: Some(rule.rule_id.clone()),
        };
    }

    // 6. Keyword-based matching
    for (keyword, category, mode) in keyword_map() {
        if lower.contains(keyword) {
            return Classification {
                category: category.into(),
                mode,
                rule_id: None,
            };
        }
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
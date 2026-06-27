//! Local routing policy — decides whether a prompt should be composed
//! via pass_through, local_compile, or llm_compile based on prompt
//! quality signals, risk domains, and user intent (--llm / --force-llm).
//!
//! Separates classification (what is this?) from routing (how much help
//! does this need?).

use crate::classifier::Mode;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RewriteStrategy {
    PassThrough,
    LocalCompile,
    LlmCompile,
}

impl RewriteStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            RewriteStrategy::PassThrough => "pass_through",
            RewriteStrategy::LocalCompile => "local_compile",
            RewriteStrategy::LlmCompile => "llm_compile",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingDecision {
    pub rewrite_strategy: RewriteStrategy,
    pub routing_score: i32,
    pub routing_signals: Vec<String>,
    pub llm_skip_reason: Option<String>,
}

// ── Signal dictionaries ───────────────────────────────────────────────

const VAGUE_TERMS: &[&str] = &[
    "weird",
    "idk",
    "stuff",
    "solid",
    "acting weird",
    "not sure",
    "feels slow",
    "somehow",
    "all that",
    "dont break",
    "don't break",
    "pls",
    "rn",
    "like ",
    "whatever",
    "i think",
    "something is wrong",
];

const HIGH_RISK_KEYWORDS: &[&str] = &[
    "auth",
    "login",
    "session",
    "cookie",
    "token",
    "password",
    "billing",
    "payment",
    "subscription",
    "database",
    "migration",
    "security",
    "permission",
    "role",
    "admin",
    "production",
    "deploy",
    "launch",
    "performance",
    "user data",
];

const HIGH_RISK_CATEGORIES: &[&str] = &[
    "security_permissions_auth",
    "production_readiness_hardening",
    "deployment_config_environment",
    "backend_api_database",
    "performance_optimization",
];

const BROAD_SCOPE_TERMS: &[&str] = &[
    "backend",
    "architecture",
    "system",
    "app",
    "scalable",
    "notification",
    "api",
    "dashboard",
    "service",
    "infrastructure",
    "scaling",
    "microservice",
];

const TINY_PASS_THROUGH: &[&str] = &[
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
    "continue",
    "resume",
    "next step",
    "proceed",
    "try again",
];

// ── Scoring ───────────────────────────────────────────────────────────

fn count_technical_anchors(lower: &str) -> usize {
    let mut count = 0;
    for c in lower.chars() {
        if c == '.' || c == '/' || c == ':' {
            count += 1;
        }
    }
    let anchors: &[&str] = &[
        "src/",
        "lib/",
        "mod.rs",
        ".ts",
        ".tsx",
        ".js",
        ".py",
        ".java",
        ".go",
        ".rs",
        "fn ",
        "def ",
        "class ",
        "import ",
        "from \"",
        " from '",
        "function ",
        "const ",
        "let ",
        ".env",
        "config",
        "dockerfile",
        "package.json",
        "cargo.toml",
        "unit test",
        "test for",
        "endpoint",
        "getcwd",
        "getcurrentworkingdirectory",
    ];
    for anchor in anchors {
        if lower.contains(anchor) {
            count += 1;
        }
    }
    count
}

fn count_multi_req(lower: &str) -> usize {
    let and_count = lower.matches(" and ").count();
    let also_count = lower.matches("also ").count();
    let maybe_count = lower.matches("maybe ").count();
    let comma_count = lower.matches(", ").count();
    and_count + also_count + maybe_count + comma_count
}

/// Compute a local routing score. Higher → more likely to need LLM.
pub fn compute_route_score(lower: &str, category: &str) -> i32 {
    let mut score: i32 = 0;

    // Vague/messy wording — each match adds 2
    for term in VAGUE_TERMS {
        if lower.contains(term) {
            score += 2;
        }
    }

    // High-risk domain — adds 3
    if HIGH_RISK_CATEGORIES.contains(&category) {
        score += 3;
    }
    // High-risk keywords anywhere in prompt — adds 3
    for kw in HIGH_RISK_KEYWORDS {
        if lower.contains(kw) {
            score += 3;
            break; // only count once
        }
    }

    // Broad/system scope — adds 2
    for term in BROAD_SCOPE_TERMS {
        if lower.contains(term) {
            score += 2;
            break; // only count once
        }
    }

    // Multi-requirement — adds 2 if ≥3 signals
    if count_multi_req(lower) >= 3 {
        score += 2;
    }

    // Missing context — no technical anchors — adds 2
    let tech = count_technical_anchors(lower);
    if tech == 0 {
        score += 2;
    }

    // Already well-structured — subtracts 5
    if tech >= 2 {
        let has_vague = VAGUE_TERMS.iter().any(|t| lower.contains(t));
        if !has_vague {
            score -= 5;
        }
    }

    scope_score_if_negative(&mut score, category);

    score
}

/// Prevent over-correction: if score would go negative just because
/// there are no vague terms, give a slight bump for broad-scope
/// categories that benefit from LLM even in the absence of messiness.
fn scope_score_if_negative(score: &mut i32, category: &str) {
    if *score >= 0 {
        return;
    }
    // Architecture prompts always benefit from LLM
    if category == "architecture_planning" {
        *score = 5;
    }
    // Security auth with missing context — neutral at worst
    if category == "security_permissions_auth" && *score < 0 {
        *score = 1;
    }
}

// ── Routing ───────────────────────────────────────────────────────────

const LLM_THRESHOLD: i32 = 4;

/// Decide how to compile a prompt.
pub fn route_prompt(
    prompt: &str,
    category: &str,
    _classification_mode: Mode,
    llm_requested: bool,
    force_llm: bool,
) -> RoutingDecision {
    let trimmed = prompt.trim();
    let lower = trimmed.to_lowercase();

    // ── Always pass-through ──────────────────────────────────────
    if trimmed.starts_with('/') {
        return RoutingDecision {
            rewrite_strategy: RewriteStrategy::PassThrough,
            routing_score: 0,
            routing_signals: vec!["slash_command".into()],
            llm_skip_reason: None,
        };
    }
    if trimmed.is_empty() {
        return RoutingDecision {
            rewrite_strategy: RewriteStrategy::PassThrough,
            routing_score: 0,
            routing_signals: vec!["empty_prompt".into()],
            llm_skip_reason: None,
        };
    }
    if TINY_PASS_THROUGH.contains(&lower.as_str()) {
        return RoutingDecision {
            rewrite_strategy: RewriteStrategy::PassThrough,
            routing_score: 0,
            routing_signals: vec!["tiny_conversational".into()],
            llm_skip_reason: None,
        };
    }

    // ── Force LLM ────────────────────────────────────────────────
    if force_llm && llm_requested {
        return RoutingDecision {
            rewrite_strategy: RewriteStrategy::LlmCompile,
            routing_score: 100,
            routing_signals: vec!["force_llm".into()],
            llm_skip_reason: None,
        };
    }

    // ── No LLM requested → local ─────────────────────────────────
    if !llm_requested {
        return RoutingDecision {
            rewrite_strategy: RewriteStrategy::LocalCompile,
            routing_score: 0,
            routing_signals: vec!["llm_not_requested".into()],
            llm_skip_reason: None,
        };
    }

    // ── Smart routing ────────────────────────────────────────────
    let score = compute_route_score(&lower, category);
    let mut signals: Vec<String> = Vec::new();

    for term in VAGUE_TERMS {
        if lower.contains(term) {
            signals.push("vague_wording".into());
            break;
        }
    }
    if HIGH_RISK_CATEGORIES.contains(&category)
        || HIGH_RISK_KEYWORDS.iter().any(|k| lower.contains(k))
    {
        signals.push("high_risk_domain".into());
    }
    for term in BROAD_SCOPE_TERMS {
        if lower.contains(term) {
            signals.push("broad_scope".into());
            break;
        }
    }
    if count_multi_req(&lower) >= 3 {
        signals.push("multi_requirement".into());
    }
    if count_technical_anchors(&lower) == 0 {
        signals.push("missing_context".into());
    }
    // High-risk overrides — force LLM regardless of score
    let high_risk_override = is_high_risk_override(&lower, category, &signals);
    let threshold_met = score >= LLM_THRESHOLD || high_risk_override;

    if threshold_met {
        if high_risk_override && !signals.contains(&"high_risk_override".to_string()) {
            signals.push("high_risk_override".into());
        }
        RoutingDecision {
            rewrite_strategy: RewriteStrategy::LlmCompile,
            routing_score: score,
            routing_signals: signals,
            llm_skip_reason: None,
        }
    } else {
        RoutingDecision {
            rewrite_strategy: RewriteStrategy::LocalCompile,
            routing_score: score,
            routing_signals: if signals.is_empty() {
                vec!["simple_low_risk".into()]
            } else {
                signals
            },
            llm_skip_reason: Some("local_rule_sufficient".into()),
        }
    }
}

/// High-risk overrides: certain domain+signal combos force LLM.
fn is_high_risk_override(lower: &str, category: &str, signals: &[String]) -> bool {
    let vague = signals.contains(&"vague_wording".to_string())
        || signals.contains(&"missing_context".to_string());
    let broad = signals.contains(&"broad_scope".to_string());

    let auth_related = lower.contains("auth")
        || lower.contains("login")
        || lower.contains("session")
        || category == "security_permissions_auth";

    let billing_related = lower.contains("billing") || lower.contains("payment");

    let prod_related =
        lower.contains("production") || lower.contains("deploy") || lower.contains("launch");

    let db_related = lower.contains("database") || lower.contains("migration");

    let auth_billing_db = auth_related || billing_related || db_related;
    let sec_vague = lower.contains("security") && vague;
    let prod_broad = prod_related && broad;
    let prod_harden = category == "production_readiness_hardening" && vague;

    (auth_billing_db && vague) || prod_broad || sec_vague || prod_harden
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn decide(
        prompt: &str,
        category: &str,
        llm_requested: bool,
        force_llm: bool,
    ) -> RoutingDecision {
        route_prompt(
            prompt,
            category,
            Mode::LocalCompile,
            llm_requested,
            force_llm,
        )
    }

    // ── Pass-through tests ───────────────────────────────────────

    #[test]
    fn test_slash_is_pass_through() {
        let d = decide("/review this", "any", false, false);
        assert_eq!(d.rewrite_strategy, RewriteStrategy::PassThrough);
    }

    #[test]
    fn test_empty_is_pass_through() {
        let d = decide("", "any", true, false);
        assert_eq!(d.rewrite_strategy, RewriteStrategy::PassThrough);
    }

    #[test]
    fn test_yes_is_pass_through() {
        let d = decide("yes", "ambiguous_tiny_command", true, false);
        assert_eq!(d.rewrite_strategy, RewriteStrategy::PassThrough);
    }

    #[test]
    fn test_slash_is_pass_through_even_with_force_llm() {
        let d = decide("/review this", "any", true, true);
        assert_eq!(d.rewrite_strategy, RewriteStrategy::PassThrough);
    }

    // ── Force LLM tests ──────────────────────────────────────────

    #[test]
    fn test_force_llm_routes_to_llm() {
        let d = decide("fix parser bug", "repair_debug", true, true);
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LlmCompile);
        assert!(d.routing_signals.contains(&"force_llm".to_string()));
    }

    #[test]
    fn test_force_llm_without_llm_does_not_force() {
        let d = decide("fix parser bug", "repair_debug", false, true);
        // llm_requested is false, so routing falls to local
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LocalCompile);
    }

    // ── Simple prompts stay local ─────────────────────────────────

    #[test]
    fn test_fix_parser_bug_stays_local_with_llm() {
        let d = decide("fix parser bug", "repair_debug", true, false);
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LocalCompile);
        assert!(d.llm_skip_reason.is_some());
    }

    #[test]
    fn test_add_unit_test_stays_local() {
        let d = decide(
            "add unit test for login endpoint",
            "testing_test_failure",
            true,
            false,
        );
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LocalCompile);
    }

    #[test]
    fn test_rename_stays_local() {
        let d = decide(
            "rename getCwd to getCurrentWorkingDirectory across the project",
            "refactor_cleanup",
            true,
            false,
        );
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LocalCompile);
    }

    #[test]
    fn test_add_spinner_stays_local() {
        let d = decide(
            "add a spinner to the loading state in Dashboard.tsx",
            "ui_ux_fix",
            true,
            false,
        );
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LocalCompile);
    }

    #[test]
    fn test_cleanup_imports_stays_local() {
        let d = decide(
            "clean up unused imports in src/components/",
            "refactor_cleanup",
            true,
            false,
        );
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LocalCompile);
    }

    // ── Messy prompts route to LLM ────────────────────────────────

    #[test]
    fn test_messy_notifications_routes_to_llm() {
        let d = decide(
            "yo add notifications to my app like emails and push and maybe sms later idk just make the backend solid so users get stuff and it dont double send",
            "feature_implementation",
            true,
            false,
        );
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LlmCompile);
        assert!(d.routing_score >= 4);
        let signals = d.routing_signals;
        assert!(signals.iter().any(|s| s.contains("vague")));
        assert!(
            signals.iter().any(|s| s.contains("broad"))
                || signals.iter().any(|s| s.contains("missing"))
        );
    }

    #[test]
    fn test_messy_auth_routes_to_llm() {
        let d = decide(
            "login is acting weird after refresh sometimes it logs me out and sometimes it still shows dashboard pls fix auth dont break anything",
            "security_permissions_auth",
            true,
            false,
        );
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LlmCompile);
        assert!(
            d.routing_signals
                .contains(&"high_risk_override".to_string())
                || d.routing_signals.contains(&"high_risk_domain".to_string())
        );
    }

    #[test]
    fn test_production_ready_vague_routes_to_llm() {
        let d = decide(
            "make it production ready before launch like errors logs security speed all that but dont rewrite the whole thing",
            "production_readiness_hardening",
            true,
            false,
        );
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LlmCompile);
    }

    #[test]
    fn test_teams_api_routes_to_llm() {
        let d = decide(
            "need api for teams invite people roles admin member stuff maybe billing later make db changes if needed but dont mess existing users",
            "feature_implementation",
            true,
            false,
        );
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LlmCompile);
    }

    #[test]
    fn test_app_feels_slow_routes_to_llm() {
        let d = decide(
            "app feels slow when dashboard loads lots of stuff maybe api is bad or db idk can you make it fast",
            "performance_optimization",
            true,
            false,
        );
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LlmCompile);
    }

    // ── Edge cases ───────────────────────────────────────────────

    #[test]
    fn test_no_llm_flag_uses_local() {
        let d = decide("add auth", "security_permissions_auth", false, false);
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LocalCompile);
    }

    #[test]
    fn test_architecture_still_llm_even_clean() {
        let d = decide(
            "design architecture for a notification system",
            "architecture_planning",
            true,
            false,
        );
        assert_eq!(d.rewrite_strategy, RewriteStrategy::LlmCompile);
    }
}

//! LLM compile boundary — future contract for LLM-assisted prompt synthesis.
//!
//! IntentLayer's `llm_compile` mode is for prompts that need deeper
//! synthesis (architecture/planning).  Real model calls are NOT enabled
//! yet.  This module defines the data contract and a safe no-op default
//! provider so the boundary is clear when real integration begins.

/// Request for LLM-assisted prompt compilation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmCompileRequest {
    /// The redacted user-authored prompt.
    pub original_prompt: String,
    /// Classified category.
    pub category: String,
    /// The full safety instruction (includes JSON response contract).
    pub instruction: String,
    /// Elements the LLM must preserve.
    pub must_preserve: Vec<String>,
    /// Elements the LLM must never invent.
    pub must_not_invent: Vec<String>,
}

/// Response from LLM-assisted prompt compilation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmCompileResponse {
    /// The synthesized compiled prompt.
    pub compiled_prompt: String,
    /// Any warnings about invented providers / frameworks.
    pub warnings: Vec<String>,
}

/// Error returned when LLM compilation cannot proceed.
#[derive(Debug, Clone)]
pub enum LlmError {
    /// No provider configured (default state).
    NoProvider,
    /// Provider rejected the request.
    ProviderError(String),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::NoProvider => write!(f, "LLM compile not available: no provider configured"),
            LlmError::ProviderError(msg) => write!(f, "LLM compile provider error: {}", msg),
        }
    }
}

/// A provider that can compile prompts using an LLM.
///
/// Implement this trait to wire a real model call.
pub trait LlmProvider {
    fn compile(&self, request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError>;
}

/// Default no-op provider.  Always returns [`LlmError::NoProvider`].
///
/// - No network calls
/// - No API key required
/// - Deterministic (always the same result)
pub struct NoopLlmCompiler;

impl LlmProvider for NoopLlmCompiler {
    fn compile(&self, _request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
        Err(LlmError::NoProvider)
    }
}

// ── Safety envelope ──────────────────────────────────────────────────

/// Options for building an LLM prompt envelope.
#[derive(Debug, Clone, Default)]
pub struct LlmEnvelopeOptions {
    /// Enable unsafe local secret passthrough.
    /// When true, the marker
    /// [[INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]...[[/INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]
    /// bypasses redaction and returns a local-only result.
    /// Raw secrets are never placed in an upstream LLM envelope.
    #[allow(dead_code)]
    pub allow_local_secret_passthrough: bool,
    /// Optional research rule context for the classified category.
    /// When provided, the LLM instruction is enriched with category-specific
    /// guidance from transformation_rules.json.
    pub rule_context: Option<RuleContext>,
}

/// Research rule context passed into the LLM envelope for enriched compilation.
#[derive(Debug, Clone)]
pub struct RuleContext {
    pub rule_id: String,
    pub category: String,
    pub risk: String,
    pub transformation_principle: String,
    pub compact_rewrite_template: Option<String>,
    pub must_preserve: Vec<String>,
    pub must_not_invent: Vec<String>,
    pub max_expansion_guidance: String,
}

/// Result of building an LLM envelope.
#[derive(Clone)]
pub enum LlmEnvelopeBuildResult {
    /// Normal envelope for upstream LLM.
    Envelope(LlmPromptEnvelope),
    /// Local-only secret passthrough — upstream LLM envelope bypassed.
    LocalSecretPassthrough {
        prompt: String,
        warnings: Vec<String>,
    },
}

impl std::fmt::Debug for LlmEnvelopeBuildResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmEnvelopeBuildResult::Envelope(env) => f.debug_tuple("Envelope").field(env).finish(),
            LlmEnvelopeBuildResult::LocalSecretPassthrough { warnings, .. } => f
                .debug_struct("LocalSecretPassthrough")
                .field("prompt", &"[REDACTED_LOCAL_SECRET_PASSTHROUGH]")
                .field("warnings", warnings)
                .finish(),
        }
    }
}

/// The safe instruction envelope sent to a future LLM provider.
///
/// Contains only the latest user-authored prompt plus constraints.
/// Secrets are redacted before serialization.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LlmPromptEnvelope {
    /// The original user-authored prompt (redacted if secrets detected).
    pub original_prompt: String,
    /// Classified category (e.g. "architecture_planning").
    pub category: String,
    /// Mode ("llm_compile").
    pub mode: String,
    /// Explicit instruction to the LLM provider.
    pub instruction: String,
    /// Elements the LLM must preserve from the original prompt.
    pub must_preserve: Vec<String>,
    /// Elements the LLM must never invent in the output.
    pub must_not_invent: Vec<String>,
    /// Warnings (e.g. secret redaction notices).
    pub warnings: Vec<String>,
}

/// Expected response contract for LLM-assisted compilation.
///
/// The provider must return only a rewritten prompt plus warnings.
/// It must not execute tasks, modify files, run shell commands,
/// or invent stack choices / services / tools / providers / scope.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LlmResponseContract {
    pub compiled_prompt: String,
    pub warnings: Vec<String>,
}

// ── Secret redaction ─────────────────────────────────────────────────

/// Redact secret-like values from a prompt string.
///
/// Returns the redacted string and a list of warnings for each redaction.
pub fn redact_secret_like_values(input: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let mut result = input.to_string();

    // Redact API-key assignment patterns: VAR_NAME=value
    let patterns: &[(&str, &str)] = &[
        ("api_key=", "[REDACTED_SECRET]"),
        ("apikey=", "[REDACTED_SECRET]"),
        ("_secret=", "[REDACTED_SECRET]"),
        ("_token=", "[REDACTED_SECRET]"),
    ];
    for (prefix, replacement) in patterns {
        let lower = result.to_lowercase();
        if let Some(pos) = lower.find(prefix) {
            let start = pos + prefix.len();
            if let Some(end) = result[start..].find(|c: char| c.is_whitespace() || c == '"') {
                result.replace_range(start..start + end, replacement);
            } else {
                result.replace_range(start.., replacement);
            }
            warnings.push(
                "Secret-like value redacted from prompt before LLM envelope serialization".into(),
            );
        }
    }

    // Redact Bearer token patterns
    let bearer_prefix = "bearer ";
    if let Some(pos) = result.to_lowercase().find(bearer_prefix) {
        let start = pos + bearer_prefix.len();
        let end = result[start..]
            .find(|c: char| c.is_whitespace() || c == '"')
            .map(|i| start + i)
            .unwrap_or(result.len());
        result.replace_range(start..end, "[REDACTED_SECRET]");
        warnings.push(
            "Secret-like value redacted from prompt before LLM envelope serialization".into(),
        );
    }

    // Redact sk- prefixed tokens
    let sk_prefix = "sk-";
    if let Some(pos) = result.to_lowercase().find(sk_prefix) {
        let start = pos;
        let end = result[start..]
            .find(|c: char| c.is_whitespace() || c == '"')
            .map(|i| start + i)
            .unwrap_or(result.len());
        result.replace_range(start..end, "[REDACTED_SECRET]");
        warnings.push(
            "Secret-like value redacted from prompt before LLM envelope serialization".into(),
        );
    }

    (result, warnings)
}

/// Local secret passthrough marker.
const PASSTHROUGH_OPEN: &str = "[[INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]";
const PASSTHROUGH_CLOSE: &str = "[[/INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]";

/// Check if the prompt contains the local secret passthrough marker.
fn has_passthrough_marker(input: &str) -> bool {
    input.contains(PASSTHROUGH_OPEN) && input.contains(PASSTHROUGH_CLOSE)
}

/// Strip the passthrough marker tags from the prompt.
fn strip_passthrough_marker(input: &str) -> String {
    input
        .replace(PASSTHROUGH_OPEN, "")
        .replace(PASSTHROUGH_CLOSE, "")
        .trim()
        .to_string()
}

// ── Envelope builder ─────────────────────────────────────────────────

/// Build a safe [`LlmPromptEnvelope`] or [`LlmEnvelopeBuildResult`]
/// for a given raw prompt and category.
pub fn build_llm_prompt_envelope(
    original_prompt: &str,
    category: &str,
    options: &LlmEnvelopeOptions,
) -> LlmEnvelopeBuildResult {
    // Check for local secret passthrough marker
    if has_passthrough_marker(original_prompt) {
        if options.allow_local_secret_passthrough {
            return LlmEnvelopeBuildResult::LocalSecretPassthrough {
                prompt: strip_passthrough_marker(original_prompt),
                warnings: vec![
                    "Local secret passthrough used; upstream LLM envelope bypassed".into(),
                ],
            };
        }
        // Marker present but opt-in disabled: redact
        let stripped = strip_passthrough_marker(original_prompt);
        let (redacted, mut warnings) = redact_secret_like_values(&stripped);
        warnings.push(
            "Local secret passthrough marker ignored because unsafe opt-in is disabled".into(),
        );
        let instruction = build_instruction(&redacted, category, options);
        return LlmEnvelopeBuildResult::Envelope(LlmPromptEnvelope {
            original_prompt: redacted,
            category: category.to_string(),
            mode: "llm_compile".to_string(),
            instruction,
            must_preserve: preservation_list(options),
            must_not_invent: no_invention_list(options),
            warnings,
        });
    }

    // Normal path: redact secrets before building envelope
    let (redacted, warnings) = redact_secret_like_values(original_prompt);
    let instruction = build_instruction(&redacted, category, options);

    LlmEnvelopeBuildResult::Envelope(LlmPromptEnvelope {
        original_prompt: redacted,
        category: category.to_string(),
        mode: "llm_compile".to_string(),
        instruction,
        must_preserve: preservation_list(options),
        must_not_invent: no_invention_list(options),
        warnings,
    })
}

fn build_instruction(prompt: &str, category: &str, options: &LlmEnvelopeOptions) -> String {
    let senior = "\
You are a senior prompt-engineering compiler for coding agents.

Rewrite weak, vague, or underspecified prompts into compact, execution-grade prompts.

You may add implied professional structure that a senior engineer would expect for this task: scope, constraints, verification, edge cases, risks, deliverables, safe implementation guidance, tests/checks, rollback/rollout notes, observability, idempotency/retry/backoff where relevant, and failure modes.

You must not invent concrete implementation details not provided by the user or project context: frameworks, cloud providers, databases, queues, payment/auth providers, deployment targets, file paths, branch names, repo names, vendors, new architecture choices, dependencies, or scope beyond the request.

Prefer useful specificity over mere rephrasing.

If the original prompt is already strong and specific, preserve it with minimal changes.

Return only valid JSON matching this exact shape:
{\"compiled_prompt\":\"...\",\"warnings\":[]}

Do not return markdown.
Do not return prose outside JSON.
";

    let mut instruction = senior.to_string();

    // Rule context: inject category-specific research guidance
    if let Some(ref rule) = options.rule_context {
        instruction.push_str("\nResearch rule:\n");
        instruction.push_str(&format!("- rule_id: {}\n", rule.rule_id));
        instruction.push_str(&format!("- category: {}\n", rule.category));
        instruction.push_str(&format!("- risk: {}\n", rule.risk));
        instruction.push_str(&format!(
            "- transformation_principle: {}\n",
            rule.transformation_principle
        ));
        if let Some(ref tmpl) = rule.compact_rewrite_template {
            instruction.push_str(&format!("- compact_rewrite_template: {}\n", tmpl));
        }
        instruction.push_str(&format!(
            "- must_preserve: {}\n",
            rule.must_preserve.join(", ")
        ));
        instruction.push_str(&format!(
            "- must_not_invent: {}\n",
            rule.must_not_invent.join(", ")
        ));
        instruction.push_str(&format!(
            "- max_expansion_guidance: {}\n",
            rule.max_expansion_guidance
        ));
    } else {
        // Universal senior compiler rules when no category-specific rule exists
        instruction.push_str("\nUniversal rules:\n");
        instruction.push_str(
            "- Preserve all context references (repo, error, file, plan, branch, etc.)\n",
        );
        instruction.push_str("- Do not execute tasks, modify files, or run commands\n");
        instruction.push_str("- Prefer minimal safe changes over comprehensive rewrites\n");
    }

    instruction.push_str(&format!("\nOriginal prompt ({}):\n{}", category, prompt));

    instruction
}

fn preservation_list(options: &LlmEnvelopeOptions) -> Vec<String> {
    if let Some(ref rule) = options.rule_context {
        let mut items = rule.must_preserve.clone();
        if !items.iter().any(|s| s.contains("context")) {
            items.push("original context references".into());
        }
        items.push("stated user intent".into());
        items
    } else {
        vec![
            "original context references".into(),
            "stated user intent".into(),
            "existing project constraints".into(),
        ]
    }
}

fn no_invention_list(options: &LlmEnvelopeOptions) -> Vec<String> {
    if let Some(ref rule) = options.rule_context {
        let mut items = rule.must_not_invent.clone();
        items.push("frameworks not mentioned".into());
        items.push("providers not mentioned".into());
        items.push("scope beyond request".into());
        items
    } else {
        vec![
            "frameworks".into(),
            "providers".into(),
            "file paths".into(),
            "databases".into(),
            "architecture".into(),
            "deployment targets".into(),
            "auth/payment providers".into(),
            "scope beyond request".into(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_opts() -> LlmEnvelopeOptions {
        LlmEnvelopeOptions::default()
    }

    fn envelope(prompt: &str, cat: &str) -> LlmPromptEnvelope {
        match build_llm_prompt_envelope(prompt, cat, &default_opts()) {
            LlmEnvelopeBuildResult::Envelope(e) => e,
            LlmEnvelopeBuildResult::LocalSecretPassthrough { .. } => {
                panic!("Unexpected local passthrough")
            }
        }
    }

    #[test]
    fn test_types_compile() {
        let req = LlmCompileRequest {
            original_prompt: "design the system".into(),
            category: "architecture_planning".into(),
            instruction: "rewrite this".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        assert_eq!(req.original_prompt, "design the system");
        assert_eq!(req.category, "architecture_planning");
    }

    #[test]
    fn test_noop_compiler_returns_no_provider() {
        let provider = NoopLlmCompiler;
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "architecture_planning".into(),
            instruction: "rewrite".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let result = provider.compile(&req);
        assert!(result.is_err(), "Noop must return error");
        match result {
            Err(LlmError::NoProvider) => {}
            _ => panic!("Expected NoProvider error"),
        }
    }

    #[test]
    fn test_noop_compiler_is_deterministic() {
        let provider = NoopLlmCompiler;
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "architecture_planning".into(),
            instruction: "rewrite".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let r1 = provider.compile(&req);
        let r2 = provider.compile(&req);
        assert_eq!(
            format!("{:?}", r1),
            format!("{:?}", r2),
            "Noop must be deterministic"
        );
    }

    #[test]
    fn test_noop_no_network() {
        let provider = NoopLlmCompiler;
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "architecture_planning".into(),
            instruction: "rewrite".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let _ = provider.compile(&req);
    }

    // ── Issue 1 — JSON response instruction ─────────────────────

    #[test]
    fn test_instruction_requires_json() {
        let env = envelope("design", "architecture_planning");
        let inst = env.instruction.to_lowercase();
        assert!(inst.contains("json"), "Instruction must require JSON");
        assert!(
            inst.contains("compiled_prompt"),
            "Must mention compiled_prompt"
        );
        assert!(inst.contains("warnings"), "Must mention warnings");
    }

    #[test]
    fn test_instruction_forbids_markdown() {
        let env = envelope("test", "repair_debug");
        let inst = env.instruction.to_lowercase();
        assert!(
            inst.contains("do not return markdown"),
            "Must forbid markdown"
        );
        assert!(
            inst.contains("do not return prose"),
            "Must forbid prose outside JSON"
        );
    }

    #[test]
    fn test_response_contract_still_deserializes() {
        let json = r#"{"compiled_prompt":"safe prompt","warnings":[]}"#;
        let resp: LlmResponseContract = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(resp.compiled_prompt, "safe prompt");
        assert!(resp.warnings.is_empty());
    }

    // ── Issue 2 — Secret redaction ──────────────────────────────

    #[test]
    fn test_sk_token_redacted_from_envelope_original_prompt() {
        let env = envelope("use sk-abc123xyz for auth", "security_permissions_auth");
        assert!(!env.original_prompt.contains("abc123xyz"));
        assert!(env.original_prompt.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn test_sk_token_redacted_from_instruction() {
        let env = envelope("use sk-abc123xyz for auth", "security_permissions_auth");
        assert!(!env.instruction.contains("abc123xyz"));
        assert!(env.instruction.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn test_bearer_token_redacted() {
        let env = envelope(
            "auth with Bearer my-token-here plz",
            "security_permissions_auth",
        );
        assert!(!env.original_prompt.contains("my-token-here"));
        assert!(env.original_prompt.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn test_env_style_api_key_redacted() {
        let env = envelope(
            "set OPENAI_API_KEY=sk-test-key in .env",
            "deployment_config_environment",
        );
        assert!(!env.original_prompt.contains("sk-test-key"));
        assert!(env.original_prompt.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn test_redaction_adds_warning() {
        let env = envelope("use sk-123 for api", "feature_implementation");
        assert!(
            env.warnings
                .iter()
                .any(|w| w.contains("Secret-like value redacted")),
            "Must have redaction warning"
        );
    }

    #[test]
    fn test_normal_non_secret_prompt_unchanged() {
        let env = envelope("fix this repo", "repair_debug");
        assert_eq!(env.original_prompt, "fix this repo");
        assert!(env.warnings.is_empty());
    }

    #[test]
    fn test_normal_quotes_do_not_bypass_redaction() {
        let env = envelope(
            r#"set "OPENAI_API_KEY=sk-abc" in config"#,
            "deployment_config_environment",
        );
        assert!(!env.original_prompt.contains("sk-abc"));
        assert!(env.original_prompt.contains("[REDACTED_SECRET]"));
    }

    // ── Issue 3 — Local secret passthrough ──────────────────────

    #[test]
    fn test_marker_alone_does_not_bypass_when_optin_disabled() {
        let prompt = "[[INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]\nAdd MY_TOKEN=abc123 to .env\n[[/INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]";
        let result =
            build_llm_prompt_envelope(prompt, "deployment_config_environment", &default_opts());
        match result {
            LlmEnvelopeBuildResult::Envelope(env) => {
                assert!(!env.original_prompt.contains("abc123"));
                assert!(env.warnings.iter().any(|w| w.contains("marker ignored")));
            }
            _ => panic!("Expected Envelope when opt-in disabled"),
        }
    }

    #[test]
    fn test_marker_plus_optin_returns_local_passthrough() {
        let prompt = "[[INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]\nAdd MY_TOKEN=abc123 to .env\n[[/INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]";
        let opts = LlmEnvelopeOptions {
            allow_local_secret_passthrough: true,
            ..Default::default()
        };
        let result = build_llm_prompt_envelope(prompt, "deployment_config_environment", &opts);
        match result {
            LlmEnvelopeBuildResult::LocalSecretPassthrough {
                prompt: p,
                warnings: w,
            } => {
                assert!(p.contains("MY_TOKEN=abc123"), "Must contain raw token");
                assert!(!p.contains("INTENTLAYER"), "Marker must be stripped");
                assert!(
                    w.iter().any(|x| x.contains("bypassed")),
                    "Must have bypass warning"
                );
            }
            _ => panic!("Expected LocalSecretPassthrough with opt-in enabled"),
        }
    }

    #[test]
    fn test_local_passthrough_never_builds_llm_envelope_with_raw_secret() {
        let prompt = "[[INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]\nuse sk-secret-key\n[[/INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]";
        let opts = LlmEnvelopeOptions {
            allow_local_secret_passthrough: true,
            ..Default::default()
        };
        let result = build_llm_prompt_envelope(prompt, "repair_debug", &opts);
        // Must NOT return an Envelope containing the raw secret
        match result {
            LlmEnvelopeBuildResult::LocalSecretPassthrough { .. } => {} // expected
            LlmEnvelopeBuildResult::Envelope(env) => {
                panic!(
                    "Must not build envelope with raw secret; got: {}",
                    env.original_prompt
                );
            }
        }
    }

    #[test]
    fn test_local_passthrough_warning_present() {
        let prompt = "[[INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]\ntest\n[[/INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]";
        let opts = LlmEnvelopeOptions {
            allow_local_secret_passthrough: true,
            ..Default::default()
        };
        let result = build_llm_prompt_envelope(prompt, "repair_debug", &opts);
        match result {
            LlmEnvelopeBuildResult::LocalSecretPassthrough { warnings, .. } => {
                assert!(
                    warnings.iter().any(|w| w.contains("bypassed")),
                    "Missing bypass warning"
                );
            }
            _ => panic!("Expected local passthrough"),
        }
    }

    // ── Original envelope tests (updated) ──────────────────────

    #[test]
    fn test_envelope_includes_original_prompt() {
        let env = envelope("design the system", "architecture_planning");
        assert_eq!(env.original_prompt, "design the system");
    }

    #[test]
    fn test_envelope_includes_category() {
        let env = envelope("test", "testing_test_failure");
        assert_eq!(env.category, "testing_test_failure");
    }

    #[test]
    fn test_envelope_includes_no_invention_rules() {
        let env = envelope("add payment", "feature_implementation");
        assert!(env.must_not_invent.iter().any(|r| r.contains("provider")));
        assert!(env.must_not_invent.iter().any(|r| r.contains("framework")));
        assert!(env.must_not_invent.iter().any(|r| r.contains("scope")));
    }

    #[test]
    fn test_envelope_includes_rewrite_only_instruction() {
        let env = envelope("test", "repair_debug");
        assert!(env.instruction.to_lowercase().contains("rewrite"));
        assert!(env.instruction.to_lowercase().contains("must not invent"));
        assert!(env.instruction.to_lowercase().contains("do not execute"));
    }

    // ── Debug redaction tests ──────────────────────────────────

    #[test]
    fn test_local_passthrough_debug_does_not_expose_prompt() {
        let result = LlmEnvelopeBuildResult::LocalSecretPassthrough {
            prompt: "MY_TOKEN=fake_value".into(),
            warnings: vec!["bypass".into()],
        };
        let debug = format!("{:?}", result);
        assert!(
            !debug.contains("fake_value"),
            "Debug must not expose raw prompt: {}",
            debug
        );
        assert!(
            debug.contains("[REDACTED_LOCAL_SECRET_PASSTHROUGH]"),
            "Must show redacted marker"
        );
    }

    #[test]
    fn test_envelope_debug_still_works_no_secrets() {
        let env = envelope("fix this repo", "repair_debug");
        let debug = format!("{:?}", env);
        assert!(
            debug.contains("fix this repo"),
            "Normal envelope debug should still work"
        );
    }

    #[test]
    fn test_passthrough_prompt_field_still_returns_raw() {
        let result = LlmEnvelopeBuildResult::LocalSecretPassthrough {
            prompt: "MY_TOKEN=fake_value".into(),
            warnings: vec![],
        };
        match result {
            LlmEnvelopeBuildResult::LocalSecretPassthrough { prompt, .. } => {
                assert_eq!(
                    prompt, "MY_TOKEN=fake_value",
                    "Raw prompt must be accessible via field"
                );
            }
            _ => panic!("Expected LocalSecretPassthrough"),
        }
    }
}

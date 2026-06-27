//! LLM compile orchestration — wires safety envelope, provider, and parser
//! into one controlled compilation path.
//!
//! No real LLM/API calls are made.  Default compiler behavior is unchanged.
//! This is an explicit opt-in path for future LLM-assisted compilation.

use crate::compiler::CompileOutput;
use crate::llm::{
    build_llm_prompt_envelope, LlmCompileRequest, LlmEnvelopeBuildResult, LlmEnvelopeOptions,
    LlmProvider,
};
use crate::llm_parser::{parse_llm_response, LlmParseOutcome};

/// Strip sensitive data from provider error strings before exposing them
/// in stderr, JSON warnings, or provider_error fields.
///
/// Removes:
/// - API keys (sk-..., gsk_..., sg_, oa_...)
/// - URLs (https://...)
/// - Authorization header patterns
fn sanitize_provider_error(raw: &str) -> String {
    let s = sanitize_api_keys(raw);
    let s = sanitize_urls(&s);
    sanitize_auth_headers(&s)
}

fn sanitize_api_keys(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let remaining = &s[i..];
        if (remaining.starts_with("sk-")
            && remaining
                .chars()
                .skip(3)
                .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .count()
                >= 5)
            || (remaining.starts_with("gsk_")
                && remaining
                    .chars()
                    .skip(4)
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .count()
                    >= 5)
            || (remaining.starts_with("sg_")
                && remaining
                    .chars()
                    .skip(3)
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .count()
                    >= 5)
            || (remaining.starts_with("oa_")
                && remaining
                    .chars()
                    .skip(3)
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .count()
                    >= 5)
        {
            result.push_str("[REDACTED_KEY]");
            // Skip the key
            i += remaining
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .map(|c| c.len_utf8())
                .sum::<usize>();
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

fn sanitize_urls(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        let remaining = &s[i..];
        if remaining.starts_with("https://") || remaining.starts_with("http://") {
            result.push_str("[REDACTED_URL]");
            // Skip until whitespace or punctuation delimiter
            i += remaining
                .chars()
                .take_while(|c| {
                    !c.is_whitespace()
                        && *c != ','
                        && *c != ';'
                        && *c != ')'
                        && *c != ']'
                        && *c != '}'
                })
                .map(|c| c.len_utf8())
                .sum::<usize>();
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

fn sanitize_auth_headers(s: &str) -> String {
    let lower = s.to_lowercase();
    for prefix in &["authorization:", "bearer "] {
        if let Some(pos) = lower.find(prefix) {
            let start = pos + prefix.len();
            let token: String = s[start..]
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != ',' && *c != ';')
                .collect();
            if token.len() > 1 {
                let before = &s[..pos + prefix.len()];
                let after = &s[pos + prefix.len() + token.len()..];
                return format!("{}{}{}", before, "[REDACTED_TOKEN]", after);
            }
        }
    }
    s.to_string()
}

/// Orchestrate the full LLM-assisted compile path.
///
/// 1. Build safety envelope (Phase 014)
/// 2. Respect local secret passthrough
/// 3. Call provider trait with full envelope (Issue 3 fix)
/// 4. Parse provider output using redacted prompt for fallback (Issue 1 fix)
/// 5. Run invention guard on final output (Issue 2 fix)
pub fn compile_with_llm_orchestration(
    raw_original_prompt: &str,
    category: &str,
    provider: &dyn LlmProvider,
    envelope_options: &LlmEnvelopeOptions,
    rules: Option<&crate::rules::RuleSet>,
) -> CompileOutput {
    // 0. Enrich envelope options with rule context from research rules.
    //    For LLM compilation, prefer rules with mode_recommendation == "llm_compile".
    //    Never inject a local_compile rule into an llm_compile prompt.
    //    If no llm_compile rule matches, fall back to universal senior compiler rules.
    let mut options = envelope_options.clone();
    if let Some(rules) = rules {
        if options.rule_context.is_none() {
            let llm_rule = rules.find_by_category_and_mode(category, "llm_compile");
            if let Some(rule) = llm_rule {
                options.rule_context = Some(crate::llm::RuleContext {
                    rule_id: rule.rule_id.clone(),
                    category: rule.category.clone(),
                    risk: rule.risk.clone(),
                    transformation_principle: rule.transformation_principle.clone(),
                    compact_rewrite_template: rule.compact_rewrite_template.clone(),
                    must_preserve: rule.must_preserve.clone(),
                    must_not_invent: rule.must_not_invent.clone(),
                    max_expansion_guidance: rule.max_expansion_guidance.clone(),
                });
            }
        }
    }

    // 1. Build envelope
    let envelope_result = build_llm_prompt_envelope(raw_original_prompt, category, &options);

    match envelope_result {
        // Local secret passthrough — bypasses provider entirely
        LlmEnvelopeBuildResult::LocalSecretPassthrough { prompt, warnings } => CompileOutput {
            original_prompt: raw_original_prompt.to_string(),
            compiled_prompt: prompt,
            mode: "llm_compile".to_string(),
            category: category.to_string(),
            changed: true,
            warnings,
            provider_error: None,
            routing: None,
        },

        // Normal envelope — call provider with full envelope request
        LlmEnvelopeBuildResult::Envelope(env) => {
            let envelope_warnings = env.warnings.clone();
            let safe_prompt = env.original_prompt.clone(); // redacted, for fallback

            let request = LlmCompileRequest {
                original_prompt: env.original_prompt,
                category: env.category,
                instruction: env.instruction,
                must_preserve: env.must_preserve,
                must_not_invent: env.must_not_invent,
            };

            // 3. Call provider
            match provider.compile(&request) {
                Ok(resp) => {
                    // 4. Parse output using REDACTED prompt for fallback (Issue 1)
                    let parse_result = parse_llm_response(&resp.compiled_prompt, &safe_prompt);

                    let (compiled, parse_warnings) = extract_from_parse(parse_result);

                    let mut all_warnings = envelope_warnings;
                    all_warnings.extend(resp.warnings);
                    all_warnings.extend(parse_warnings);

                    // 5. Run invention guard (Issue 2)
                    let guard_warnings =
                        crate::guard::check_invention(raw_original_prompt, &compiled);
                    all_warnings.extend(guard_warnings);

                    CompileOutput {
                        original_prompt: raw_original_prompt.to_string(),
                        compiled_prompt: compiled,
                        mode: "llm_compile".to_string(),
                        category: category.to_string(),
                        changed: true,
                        warnings: all_warnings,
                        provider_error: None,
                        routing: None,
                    }
                }
                Err(err) => {
                    // Provider failed — fallback locally using redacted prompt
                    let sanitized_error = sanitize_provider_error(&err.to_string());
                    let mut warnings = envelope_warnings;
                    warnings.push(format!(
                        "LLM provider failed; fell back to local compilation: {}",
                        sanitized_error
                    ));
                    CompileOutput {
                        original_prompt: raw_original_prompt.to_string(),
                        compiled_prompt: format!(
                            "Using the provided prompt in the current project context: {}",
                            safe_prompt
                        ),
                        mode: "llm_compile".to_string(),
                        category: category.to_string(),
                        changed: true,
                        warnings,
                        provider_error: Some(sanitized_error),
                        routing: None,
                    }
                }
            }
        }
    }
}

fn extract_from_parse(parse_result: LlmParseOutcome) -> (String, Vec<String>) {
    match parse_result {
        LlmParseOutcome::Parsed(r) => (r.compiled_prompt, r.warnings),
        LlmParseOutcome::Repaired { response, warnings } => {
            let mut w = warnings;
            w.extend(response.warnings);
            (response.compiled_prompt, w)
        }
        LlmParseOutcome::BestEffort {
            compiled_prompt,
            warnings,
        } => (compiled_prompt, warnings),
        LlmParseOutcome::Fallback {
            compiled_prompt,
            warnings,
        } => (compiled_prompt, warnings),
    }
}

// ── Mock providers (test only, no real API calls) ────────────────────

use crate::llm::{LlmCompileResponse, LlmError};

/// Mock provider that returns strict JSON.
pub struct MockProviderReturnsJson;

impl LlmProvider for MockProviderReturnsJson {
    fn compile(&self, _request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
        Ok(LlmCompileResponse {
            compiled_prompt:
                r#"{"compiled_prompt":"Restructure the payment flow using existing stack","warnings":[]}"#
                    .into(),
            warnings: vec![],
        })
    }
}

/// Mock provider that returns fenced JSON.
pub struct MockProviderReturnsFencedJson;

impl LlmProvider for MockProviderReturnsFencedJson {
    fn compile(&self, _request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
        Ok(LlmCompileResponse {
            compiled_prompt:
                "```json\n{\"compiled_prompt\":\"Refactor with existing patterns\",\"warnings\":[]}\n```"
                    .into(),
            warnings: vec![],
        })
    }
}

/// Mock provider that returns bare text.
pub struct MockProviderReturnsBareText;

impl LlmProvider for MockProviderReturnsBareText {
    fn compile(&self, _request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
        Ok(LlmCompileResponse {
            compiled_prompt: "Fix the thing using current patterns".into(),
            warnings: vec![],
        })
    }
}

/// Mock provider that returns empty/invalid output.
pub struct MockProviderReturnsEmpty;

impl LlmProvider for MockProviderReturnsEmpty {
    fn compile(&self, _request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
        Ok(LlmCompileResponse {
            compiled_prompt: "".into(),
            warnings: vec![],
        })
    }
}

/// Mock provider that always fails.
pub struct MockProviderFails;

impl LlmProvider for MockProviderFails {
    fn compile(&self, _request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
        Err(LlmError::ProviderError("simulated failure".into()))
    }
}

/// Mock provider that invents Stripe (for invention guard test).
pub struct MockProviderInventsStripe;

impl LlmProvider for MockProviderInventsStripe {
    fn compile(&self, _request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
        Ok(LlmCompileResponse {
            compiled_prompt: r#"{"compiled_prompt":"Add Stripe payments","warnings":[]}"#.into(),
            warnings: vec![],
        })
    }
}

/// Mock provider that inspects its request (for safety envelope test).
pub struct MockProviderInspectsRequest {
    pub received: std::cell::RefCell<Option<LlmCompileRequest>>,
}

impl MockProviderInspectsRequest {
    pub fn new() -> Self {
        MockProviderInspectsRequest {
            received: std::cell::RefCell::new(None),
        }
    }
}

impl LlmProvider for MockProviderInspectsRequest {
    fn compile(&self, request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
        *self.received.borrow_mut() = Some(request.clone());
        Ok(LlmCompileResponse {
            compiled_prompt: r#"{"compiled_prompt":"safe prompt","warnings":[]}"#.into(),
            warnings: vec![],
        })
    }
}

impl Default for MockProviderInspectsRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Compiler;
    use crate::rules::RuleSet;
    use std::path::Path;

    fn make_compiler() -> Compiler {
        let rules = RuleSet::load(Path::new("research/transformation_rules.json"))
            .expect("Failed to load rules");
        Compiler::new(rules)
    }

    fn default_opts() -> LlmEnvelopeOptions {
        LlmEnvelopeOptions::default()
    }

    /// Convenience wrapper that passes `None` for the optional RuleSet.
    fn orch(
        prompt: &str,
        category: &str,
        provider: &dyn LlmProvider,
        opts: &LlmEnvelopeOptions,
    ) -> CompileOutput {
        compile_with_llm_orchestration(prompt, category, provider, opts, None)
    }

    #[test]
    fn test_orchestration_uses_strict_json_provider_output() {
        let provider = MockProviderReturnsJson;
        let output = orch(
            "design payment system",
            "architecture_planning",
            &provider,
            &default_opts(),
        );
        assert!(output.compiled_prompt.contains("Restructure"));
    }

    #[test]
    fn test_orchestration_uses_fenced_json_provider_output() {
        let provider = MockProviderReturnsFencedJson;
        let output = orch(
            "refactor parser",
            "refactor_cleanup",
            &provider,
            &default_opts(),
        );
        assert!(output.compiled_prompt.contains("Refactor"));
    }

    #[test]
    fn test_orchestration_handles_bare_text_with_warning() {
        let provider = MockProviderReturnsBareText;
        let output = orch("fix the thing", "repair_debug", &provider, &default_opts());
        assert!(output.compiled_prompt.contains("Fix the thing"));
        assert!(
            output
                .warnings
                .iter()
                .any(|w| w.to_lowercase().contains("bare text")),
            "Bare text must get a parser warning"
        );
    }

    #[test]
    fn test_orchestration_falls_back_on_empty_output() {
        let provider = MockProviderReturnsEmpty;
        let output = orch(
            "original user prompt",
            "repair_debug",
            &provider,
            &default_opts(),
        );
        assert!(
            output.compiled_prompt.contains("original user prompt"),
            "Fallback must include original prompt"
        );
    }

    #[test]
    fn test_orchestration_falls_back_on_provider_error() {
        let provider = MockProviderFails;
        let output = orch(
            "restructure microservice",
            "architecture_planning",
            &provider,
            &default_opts(),
        );
        assert!(output
            .warnings
            .iter()
            .any(|w| w.contains("provider failed")));
        assert!(output.compiled_prompt.contains("restructure microservice"));
        assert!(output.provider_error.is_some());
    }

    #[test]
    fn test_envelope_warnings_preserved() {
        // Use a prompt with a secret-like value that gets redacted
        let provider = MockProviderReturnsJson;
        let output = orch(
            "use sk-redact-me for auth",
            "security_permissions_auth",
            &provider,
            &default_opts(),
        );
        assert!(
            output
                .warnings
                .iter()
                .any(|w| w.contains("Secret-like value redacted")),
            "Redaction warning must be preserved"
        );
    }

    #[test]
    fn test_local_secret_passthrough_bypasses_provider() {
        let provider = MockProviderFails; // would fail if called
        let opts = LlmEnvelopeOptions {
            allow_local_secret_passthrough: true,
            ..Default::default()
        };
        let output = orch(
            "[[INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]\nAdd TOKEN=abc to .env\n[[/INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]",
            "deployment_config_environment",
            &provider,
            &opts,
        );
        assert!(
            output.compiled_prompt.contains("TOKEN=abc"),
            "Local passthrough must preserve raw token"
        );
        assert!(
            output.warnings.iter().any(|w| w.contains("bypassed")),
            "Must have bypass warning"
        );
        // Provider was never called (it would have panicked/failed)
        assert!(!output.compiled_prompt.contains("provider failed"));
    }

    #[test]
    fn test_local_secret_passthrough_only_when_optin_enabled() {
        let provider = MockProviderReturnsJson;
        let opts = LlmEnvelopeOptions::default(); // opt-in disabled
        let output = orch(
            "[[INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]\nAdd TOKEN=abc to .env\n[[/INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]",
            "deployment_config_environment",
            &provider,
            &opts,
        );
        // With opt-in disabled, the secret is redacted and provider is called
        assert!(
            !output.compiled_prompt.contains("TOKEN=abc"),
            "Secret must be redacted when opt-in disabled"
        );
    }

    #[test]
    fn test_provider_never_receives_raw_secret_in_normal_path() {
        let provider = MockProviderReturnsJson;
        let output = orch(
            "set OPENAI_API_KEY=sk-secret in config",
            "deployment_config_environment",
            &provider,
            &default_opts(),
        );
        // Secret was redacted before envelope was built, provider got clean prompt
        assert!(!output.compiled_prompt.contains("sk-secret"));
    }

    #[test]
    fn test_default_compile_behavior_unchanged() {
        let compiler = make_compiler();
        let input = crate::compiler::CompileInput {
            prompt: "fix this repo".into(),
        };
        let output = crate::compiler::compile(&input, &compiler);
        assert_eq!(output.mode, "local_compile");
        assert!(output.compiled_prompt.contains("context"));
    }

    #[test]
    fn test_no_network_api_call_made() {
        // All mock providers are local — no I/O
        let provider = MockProviderReturnsJson;
        let output = orch("test", "architecture_planning", &provider, &default_opts());
        assert!(!output.compiled_prompt.is_empty());
    }

    // ── Issue 1 — fallback must not re-leak raw secrets ───────────

    #[test]
    fn test_empty_provider_output_does_not_leak_raw_secret_in_fallback() {
        let provider = MockProviderReturnsEmpty;
        let output = orch(
            "set OPENAI_API_KEY=sk-secret-123 in config",
            "deployment_config_environment",
            &provider,
            &default_opts(),
        );
        assert!(
            !output.compiled_prompt.contains("sk-secret-123"),
            "Fallback must not contain raw secret"
        );
        assert!(
            output.compiled_prompt.contains("[REDACTED_SECRET]"),
            "Fallback should contain redacted marker"
        );
    }

    #[test]
    fn test_provider_failure_fallback_does_not_leak_raw_secret() {
        let provider = MockProviderFails;
        let output = orch(
            "set MY_TOKEN=abc123 in .env",
            "deployment_config_environment",
            &provider,
            &default_opts(),
        );
        assert!(!output.compiled_prompt.contains("abc123"));
        assert!(output.compiled_prompt.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn test_local_passthrough_still_returns_raw_secret_only_when_optin() {
        let provider = MockProviderReturnsJson;
        let opts = LlmEnvelopeOptions {
            allow_local_secret_passthrough: true,
            ..Default::default()
        };
        let output = orch(
            "[[INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]\nuse TOKEN=mysecret\n[[/INTENTLAYER_LOCAL_SECRET_PASSTHROUGH]]",
            "deployment_config_environment",
            &provider,
            &opts,
        );
        assert!(output.compiled_prompt.contains("TOKEN=mysecret"));
    }

    // ── Issue 2 — invention guard on final LLM output ────────────

    #[test]
    fn test_provider_invents_stripe_produces_guard_warning() {
        let provider = MockProviderInventsStripe;
        let output = orch(
            "add payment",
            "feature_implementation",
            &provider,
            &default_opts(),
        );
        assert!(
            output.warnings.iter().any(|w| w.contains("Stripe")),
            "Must warn about invented Stripe"
        );
    }

    #[test]
    fn test_normal_non_invented_provider_output_has_no_invention_warning() {
        let provider = MockProviderReturnsJson;
        let output = orch(
            "add payment",
            "feature_implementation",
            &provider,
            &default_opts(),
        );
        // No Stripe warning expected
        assert!(!output.warnings.iter().any(|w| w.contains("Stripe")));
    }

    #[test]
    fn test_default_compile_remains_unchanged_after_invention_guard() {
        let compiler = make_compiler();
        let input = crate::compiler::CompileInput {
            prompt: "add payment".into(),
        };
        let output = crate::compiler::compile(&input, &compiler);
        // Default compile does not invent Stripe
        assert!(!output.compiled_prompt.contains("Stripe"));
    }

    // ── Issue 3 — provider receives safety envelope ───────────────

    #[test]
    fn test_provider_received_json_response_instruction() {
        let provider = MockProviderInspectsRequest::new();
        let _ = orch(
            "design the system",
            "architecture_planning",
            &provider,
            &default_opts(),
        );
        let req = provider.received.borrow();
        let instruction = &req.as_ref().unwrap().instruction;
        assert!(
            instruction.contains("compiled_prompt"),
            "Provider must receive JSON contract"
        );
    }

    #[test]
    fn test_provider_received_no_invention_constraints() {
        let provider = MockProviderInspectsRequest::new();
        let _ = orch(
            "add payment",
            "feature_implementation",
            &provider,
            &default_opts(),
        );
        let req = provider.received.borrow();
        assert!(!req.as_ref().unwrap().must_not_invent.is_empty());
    }

    #[test]
    fn test_provider_received_preservation_constraints() {
        let provider = MockProviderInspectsRequest::new();
        let _ = orch(
            "continue from plan",
            "continuation_previous_plan",
            &provider,
            &default_opts(),
        );
        let req = provider.received.borrow();
        assert!(!req.as_ref().unwrap().must_preserve.is_empty());
    }

    #[test]
    fn test_provider_did_not_receive_raw_secret() {
        let provider = MockProviderInspectsRequest::new();
        let _ = orch(
            "use sk-xyz-secret for api",
            "security_permissions_auth",
            &provider,
            &default_opts(),
        );
        let req = provider.received.borrow();
        let prompt = &req.as_ref().unwrap().original_prompt;
        assert!(
            !prompt.contains("sk-xyz-secret"),
            "Provider must not receive raw secret"
        );
    }

    #[test]
    fn test_provider_error_is_populated_on_failure() {
        let provider = MockProviderFails;
        let output = orch("test", "architecture_planning", &provider, &default_opts());
        assert!(output.provider_error.is_some());
        assert!(output
            .provider_error
            .as_ref()
            .unwrap()
            .contains("simulated failure"));
    }

    #[test]
    fn test_provider_error_propagates_clean_message() {
        let provider = MockProviderFails;
        let output = orch("test", "architecture_planning", &provider, &default_opts());
        assert!(output.provider_error.is_some());
        let err = output.provider_error.as_ref().unwrap();
        assert!(err.contains("LLM compile provider error"));
        assert!(err.contains("simulated failure"));
    }

    // ── Phase 028: Error sanitization tests ──────────────────────

    #[test]
    fn test_sanitize_provider_error_redacts_api_key() {
        let raw = "LLM compile provider error: failed with key sk-abc123xyz for auth";
        let clean = sanitize_provider_error(raw);
        assert!(!clean.contains("sk-abc123xyz"));
        assert!(clean.contains("[REDACTED_KEY]"));
    }

    #[test]
    fn test_sanitize_provider_error_redacts_url() {
        let raw = "transport error: connection to https://api.example.com/v1/chat failed";
        let clean = sanitize_provider_error(raw);
        assert!(!clean.contains("https://api.example.com"));
        assert!(clean.contains("[REDACTED_URL]"));
    }

    #[test]
    fn test_sanitize_provider_error_redacts_bearer_token() {
        let raw = "Bearer abcdef1234567890abcdef1234567890 is invalid";
        let clean = sanitize_provider_error(raw);
        assert!(!clean.contains("abcdef1234567890abcdef1234567890"));
        assert!(clean.contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn test_sanitize_provider_error_preserves_safe_text() {
        let raw = "HTTP 401 Unauthorized";
        let clean = sanitize_provider_error(raw);
        assert_eq!(clean, raw);
    }

    #[test]
    fn test_provider_error_in_output_is_sanitized() {
        struct MockFailsWithKeyUrl;
        impl LlmProvider for MockFailsWithKeyUrl {
            fn compile(
                &self,
                _request: &LlmCompileRequest,
            ) -> Result<LlmCompileResponse, LlmError> {
                Err(LlmError::ProviderError(
                    "HTTP 401 from https://api.openai.com/v1: key sk-test-key-abc is invalid"
                        .into(),
                ))
            }
        }
        let output = orch(
            "test",
            "architecture_planning",
            &MockFailsWithKeyUrl,
            &default_opts(),
        );
        let err = output.provider_error.as_ref().unwrap();
        assert!(!err.contains("sk-test-key-abc"), "Key must be redacted");
        assert!(!err.contains("api.openai.com"), "URL must be redacted");
        // Error still contains useful info
        assert!(
            err.contains("HTTP 401"),
            "Should preserve status code: {}",
            err
        );
    }

    // ── Phase 028: Compiled-only fallback tests ────────────────────

    /// Simulates `--compiled-only --llm --allow-llm-fallback` when provider fails.
    /// The orchestration output must carry a non-empty compiled_prompt suitable for
    /// compiled-only handoff, a provider_error marker, and sanitized fallback warnings.
    #[test]
    fn test_compiled_only_fallback_produces_handoff_ready_output() {
        let provider = MockProviderFails;
        let output = orch(
            "fix the parser bug in src/parser.rs",
            "repair_debug",
            &provider,
            &default_opts(),
        );

        // Provider failure marker must be present (triggers --allow-llm-fallback path)
        assert!(
            output.provider_error.is_some(),
            "Must have provider_error for fallback detection"
        );

        // Fallback compiled_prompt must be non-empty (content for --compiled-only handoff)
        assert!(
            !output.compiled_prompt.trim().is_empty(),
            "Compiled prompt must not be empty for compiled-only handoff"
        );

        // Fallback text should reference the original prompt topic
        assert!(
            output.compiled_prompt.contains("fix the parser bug")
                || output.compiled_prompt.contains("[REDACTED")
                || output.compiled_prompt.contains("parser"),
            "Fallback should preserve prompt context: {}",
            output.compiled_prompt
        );

        // Warnings must carry the provider failure notice (shows in stderr with [fallback])
        assert!(
            output
                .warnings
                .iter()
                .any(|w| w.contains("provider failed")),
            "Warnings must document provider failure: {:?}",
            output.warnings
        );
    }

    /// Provider failure with secret-laden error + compiled-only fallback must not
    /// leak keys or URLs anywhere in the output (provider_error, warnings, or
    /// compiled_prompt).
    #[test]
    fn test_compiled_only_fallback_sanitized_no_secret_leak() {
        struct MockFailsSensitive;
        impl LlmProvider for MockFailsSensitive {
            fn compile(
                &self,
                _request: &LlmCompileRequest,
            ) -> Result<LlmCompileResponse, LlmError> {
                Err(LlmError::ProviderError(
                    "HTTP 401 from https://api.groq.com/openai/v1: key gsk_test12345abc is invalid"
                        .into(),
                ))
            }
        }
        let output = orch(
            "fix auth bug",
            "security_permissions_auth",
            &MockFailsSensitive,
            &default_opts(),
        );

        let all_text = format!("{} {:?}", output.compiled_prompt, output.warnings,);
        let err_text = output.provider_error.as_deref().unwrap_or("");

        // No raw key in any output field
        assert!(!all_text.contains("gsk_test12345abc"));
        assert!(!err_text.contains("gsk_test12345abc"));

        // No internal URL in any output field
        assert!(!all_text.contains("api.groq.com"));
        assert!(!err_text.contains("api.groq.com"));

        // Sanitized key marker should appear somewhere (proves redaction fired)
        assert!(
            all_text.contains("[REDACTED_KEY]") || err_text.contains("[REDACTED_KEY]"),
            "Key must be redacted"
        );

        // URL must be redacted
        assert!(
            all_text.contains("[REDACTED_URL]") || err_text.contains("[REDACTED_URL]"),
            "URL must be redacted"
        );
    }

    // ── Phase 030A: Research-backed LLM envelope tests ─────────────

    fn load_rules() -> RuleSet {
        RuleSet::load(Path::new("research/transformation_rules.json"))
            .expect("Failed to load test rules")
    }

    #[test]
    fn test_rule_context_enriches_architecture_prompt() {
        let rules = load_rules();
        let provider = MockProviderReturnsJson;
        let opts = default_opts();
        let output = compile_with_llm_orchestration(
            "Design architecture for a notification system.",
            "architecture_planning",
            &provider,
            &opts,
            Some(&rules),
        );
        // The ARCH rule should be found and the output should not be empty
        assert!(!output.compiled_prompt.is_empty());
    }

    #[test]
    fn test_senior_compiler_instruction_is_used() {
        struct MockCapturesInstruction {
            captured: std::cell::RefCell<Option<LlmCompileRequest>>,
        }
        impl LlmProvider for MockCapturesInstruction {
            fn compile(&self, request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
                *self.captured.borrow_mut() = Some(request.clone());
                Ok(LlmCompileResponse {
                    compiled_prompt: r#"{"compiled_prompt":"dummy","warnings":[]}"#.into(),
                    warnings: vec![],
                })
            }
        }

        let rules = load_rules();
        let provider = MockCapturesInstruction {
            captured: std::cell::RefCell::new(None),
        };
        let opts = default_opts();
        let _ = compile_with_llm_orchestration(
            "Design architecture for a notification system.",
            "architecture_planning",
            &provider,
            &opts,
            Some(&rules),
        );

        let req = provider.captured.borrow();
        let instruction = &req.as_ref().unwrap().instruction;
        assert!(
            instruction.contains("senior prompt-engineering compiler"),
            "Instruction must include senior compiler role: {:.100}",
            instruction
        );
        assert!(
            instruction.contains("ARCH-001"),
            "Instruction must reference the ARCH rule ID"
        );
        assert!(
            instruction.contains("architecture_planning"),
            "Instruction must include the category"
        );
        assert!(
            instruction.contains("transformation_principle"),
            "Instruction must include transformation principle heading"
        );
    }

    #[test]
    fn test_instruction_allows_professional_structure_forbids_invention() {
        struct MockCapturesInstruction {
            captured: std::cell::RefCell<Option<LlmCompileRequest>>,
        }
        impl LlmProvider for MockCapturesInstruction {
            fn compile(&self, request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
                *self.captured.borrow_mut() = Some(request.clone());
                Ok(LlmCompileResponse {
                    compiled_prompt: r#"{"compiled_prompt":"dummy","warnings":[]}"#.into(),
                    warnings: vec![],
                })
            }
        }

        let rules = load_rules();
        let provider = MockCapturesInstruction {
            captured: std::cell::RefCell::new(None),
        };
        let opts = default_opts();
        let _ = compile_with_llm_orchestration(
            "Design architecture for a notification system.",
            "architecture_planning",
            &provider,
            &opts,
            Some(&rules),
        );

        let req = provider.captured.borrow();
        let inst = &req.as_ref().unwrap().instruction;
        let lower = inst.to_lowercase();

        // Must allow professional structure
        assert!(
            lower.contains("scope") || lower.contains("verification"),
            "Must allow professional structure dimensions"
        );

        // Must forbid concrete inventions
        assert!(
            lower.contains("must not invent")
                || lower.contains("cloud providers")
                || lower.contains("framework"),
            "Must forbid concrete invention"
        );
    }

    #[test]
    fn test_rule_context_populated_for_matching_category() {
        let rules = load_rules();
        let arch_rule = rules.find_by_category("architecture_planning");
        assert!(arch_rule.is_some(), "ARCH-001 should exist in rules");
        assert_eq!(
            arch_rule.unwrap().rule_id,
            "ARCH-001",
            "ARCH category should match ARCH-001"
        );

        // repair category should also have a rule
        let repair_rule = rules.find_by_category("repair_debug");
        assert!(repair_rule.is_some(), "REPAIR-001 should exist in rules");
        assert_eq!(repair_rule.unwrap().rule_id, "REPAIR-001");
    }

    #[test]
    fn test_universal_rules_used_when_no_category_rule_matches() {
        struct MockCapturesInstruction {
            captured: std::cell::RefCell<Option<LlmCompileRequest>>,
        }
        impl LlmProvider for MockCapturesInstruction {
            fn compile(&self, request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
                *self.captured.borrow_mut() = Some(request.clone());
                Ok(LlmCompileResponse {
                    compiled_prompt: r#"{"compiled_prompt":"dummy","warnings":[]}"#.into(),
                    warnings: vec![],
                })
            }
        }

        let rules = load_rules();
        let provider = MockCapturesInstruction {
            captured: std::cell::RefCell::new(None),
        };
        let opts = default_opts();
        let _ = compile_with_llm_orchestration(
            "some nonexistent category prompt",
            "nonexistent_category_xyz",
            &provider,
            &opts,
            Some(&rules),
        );

        let req = provider.captured.borrow();
        let inst = &req.as_ref().unwrap().instruction;
        assert!(
            inst.contains("senior prompt-engineering compiler"),
            "Universal senior rules must still appear when no category rule matches"
        );
        assert!(
            inst.contains("Universal rules"),
            "Must include universal rules section when no category match: {:.200}",
            inst
        );
        // Must not contain specific rule data
        assert!(
            !inst.contains("ARCH-001"),
            "Must not contain specific rule when category does not match"
        );
    }

    #[test]
    fn test_mock_provider_can_produce_architecture_output() {
        struct MockSeniorArchitect;
        impl LlmProvider for MockSeniorArchitect {
            fn compile(
                &self,
                _request: &LlmCompileRequest,
            ) -> Result<LlmCompileResponse, LlmError> {
                Ok(LlmCompileResponse {
                    compiled_prompt: r#"{"compiled_prompt":"Design a minimal viable notification system. Cover message flow, delivery channels, retry/backoff, idempotency, failure handling, observability, scaling tradeoffs, and safe rollout. Do not assume specific cloud providers, databases, queues, or frameworks.","warnings":[]}"#.into(),
                    warnings: vec![],
                })
            }
        }

        let output = orch(
            "Design architecture for a notification system.",
            "architecture_planning",
            &MockSeniorArchitect,
            &default_opts(),
        );

        let lower = output.compiled_prompt.to_lowercase();
        // Must include several generic architecture dimensions
        let dimensions = [
            "message flow",
            "delivery",
            "retry",
            "idempotenc",
            "failure",
            "observab",
            "scaling",
            "tradeoff",
            "rollout",
        ];
        let match_count = dimensions.iter().filter(|d| lower.contains(*d)).count();
        assert!(
            match_count >= 3,
            "Must include at least 3 architecture dimensions, got {}. Prompt: {}",
            match_count,
            output.compiled_prompt
        );

        // Must not invent specific vendors/stack
        let forbidden = [
            "aws",
            "redis",
            "kafka",
            "postgres",
            "stripe",
            "auth0",
            "kubernetes",
            "docker",
        ];
        for word in &forbidden {
            assert!(
                !lower.contains(word),
                "Must not invent '{}': {}",
                word,
                output.compiled_prompt
            );
        }
    }

    #[test]
    fn test_repair_rule_context_works() {
        let rules = load_rules();
        let repair_rule = rules.find_by_category("repair_debug");
        assert!(repair_rule.is_some());
        let rule = repair_rule.unwrap();
        assert!(rule
            .transformation_principle
            .to_lowercase()
            .contains("root"));
        assert!(rule.must_preserve.contains(&"current context".to_string()));
    }

    #[test]
    fn test_existing_secret_redaction_still_works() {
        let rules = load_rules();
        let output = compile_with_llm_orchestration(
            "use sk-abc123xyz for auth",
            "security_permissions_auth",
            &MockProviderReturnsJson,
            &default_opts(),
            Some(&rules),
        );
        assert!(!output.compiled_prompt.contains("abc123xyz"));
    }

    #[test]
    fn test_existing_provider_failure_behavior_unchanged() {
        let rules = load_rules();
        let output = compile_with_llm_orchestration(
            "restructure microservice",
            "architecture_planning",
            &MockProviderFails,
            &default_opts(),
            Some(&rules),
        );
        assert!(output.provider_error.is_some());
        assert!(output
            .warnings
            .iter()
            .any(|w| w.contains("provider failed")));
    }

    #[test]
    fn test_llm_mode_does_not_inject_local_compile_rules() {
        // DEPLOY-001 has mode_recommendation: "local_compile"
        // It must NOT be injected into an llm_compile prompt for its category.
        // When no llm_compile rule exists for deployment_config_environment,
        // universal senior compiler rules should be used instead.

        struct MockCapturesInstruction {
            captured: std::cell::RefCell<Option<LlmCompileRequest>>,
        }
        impl LlmProvider for MockCapturesInstruction {
            fn compile(&self, request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
                *self.captured.borrow_mut() = Some(request.clone());
                Ok(LlmCompileResponse {
                    compiled_prompt: r#"{"compiled_prompt":"dummy","warnings":[]}"#.into(),
                    warnings: vec![],
                })
            }
        }

        let rules = load_rules();

        // Verify DEPLOY-001 exists and is local_compile
        let deploy_rule = rules.find_by_category("deployment_config_environment");
        assert!(deploy_rule.is_some(), "DEPLOY-001 should exist");
        assert_eq!(
            deploy_rule.unwrap().mode_recommendation,
            "local_compile",
            "DEPLOY-001 must be local_compile for this test to be meaningful"
        );

        let provider = MockCapturesInstruction {
            captured: std::cell::RefCell::new(None),
        };
        let opts = default_opts();
        let _ = compile_with_llm_orchestration(
            "migrate to new server",
            "deployment_config_environment",
            &provider,
            &opts,
            Some(&rules),
        );

        let req = provider.captured.borrow();
        let inst = &req.as_ref().unwrap().instruction;

        // Must NOT inject DEPLOY-001 (local_compile rule) into LLM prompt
        assert!(
            !inst.contains("DEPLOY-001"),
            "Must not inject local_compile rule DEPLOY-001 into llm_compile prompt"
        );

        // Must use universal senior compiler rules instead
        assert!(
            inst.contains("senior prompt-engineering compiler"),
            "Must use senior compiler persona"
        );
        assert!(
            inst.contains("Universal rules"),
            "Must use universal rules when no llm_compile rule matches: {:.200}",
            inst
        );
    }

    #[test]
    fn test_architecture_llm_compile_rule_still_works() {
        // ARCH-001 has mode_recommendation: "llm_compile" — must still be injected

        struct MockCapturesInstruction {
            captured: std::cell::RefCell<Option<LlmCompileRequest>>,
        }
        impl LlmProvider for MockCapturesInstruction {
            fn compile(&self, request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
                *self.captured.borrow_mut() = Some(request.clone());
                Ok(LlmCompileResponse {
                    compiled_prompt: r#"{"compiled_prompt":"dummy","warnings":[]}"#.into(),
                    warnings: vec![],
                })
            }
        }

        let rules = load_rules();
        let provider = MockCapturesInstruction {
            captured: std::cell::RefCell::new(None),
        };
        let opts = default_opts();
        let _ = compile_with_llm_orchestration(
            "Design architecture for a notification system.",
            "architecture_planning",
            &provider,
            &opts,
            Some(&rules),
        );

        let req = provider.captured.borrow();
        let inst = &req.as_ref().unwrap().instruction;

        // ARCH-001 is llm_compile — must be present
        assert!(inst.contains("ARCH-001"), "Must include ARCH-001 rule");
        assert!(
            inst.contains("senior prompt-engineering compiler"),
            "Must use senior compiler persona"
        );
    }
}

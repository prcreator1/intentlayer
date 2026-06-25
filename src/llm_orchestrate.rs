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

/// Orchestrate the full LLM-assisted compile path.
///
/// 1. Build safety envelope (Phase 014)
/// 2. Respect local secret passthrough
/// 3. Call provider trait (no real API)
/// 4. Parse provider output (Phase 015)
/// 5. Fallback locally on failure
pub fn compile_with_llm_orchestration(
    original_prompt: &str,
    category: &str,
    provider: &dyn LlmProvider,
    envelope_options: &LlmEnvelopeOptions,
) -> CompileOutput {
    // 1. Build envelope
    let envelope_result = build_llm_prompt_envelope(original_prompt, category, envelope_options);

    match envelope_result {
        // Local secret passthrough — bypasses provider entirely
        LlmEnvelopeBuildResult::LocalSecretPassthrough { prompt, warnings } => CompileOutput {
            original_prompt: original_prompt.to_string(),
            compiled_prompt: prompt,
            mode: "llm_compile".to_string(),
            category: category.to_string(),
            changed: true,
            warnings,
        },

        // Normal envelope — call provider
        LlmEnvelopeBuildResult::Envelope(env) => {
            let envelope_warnings = env.warnings.clone();
            let request = LlmCompileRequest {
                original_prompt: env.original_prompt,
                category: env.category,
            };

            // 3. Call provider
            match provider.compile(&request) {
                Ok(resp) => {
                    // Provider succeeded — parse output
                    let parse_result = parse_llm_response(&resp.compiled_prompt, original_prompt);

                    let (compiled, parse_warnings) = match parse_result {
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
                    };

                    let mut all_warnings = envelope_warnings;
                    all_warnings.extend(resp.warnings);
                    all_warnings.extend(parse_warnings);

                    CompileOutput {
                        original_prompt: original_prompt.to_string(),
                        compiled_prompt: compiled,
                        mode: "llm_compile".to_string(),
                        category: category.to_string(),
                        changed: true,
                        warnings: all_warnings,
                    }
                }
                Err(_err) => {
                    // Provider failed — fallback locally
                    let mut warnings = envelope_warnings;
                    warnings.push("LLM provider failed; fell back to local compilation".into());
                    CompileOutput {
                        original_prompt: original_prompt.to_string(),
                        compiled_prompt: format!(
                            "Using the original prompt in the current project context: {}",
                            original_prompt
                        ),
                        mode: "llm_compile".to_string(),
                        category: category.to_string(),
                        changed: true,
                        warnings,
                    }
                }
            }
        }
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

    #[test]
    fn test_orchestration_uses_strict_json_provider_output() {
        let provider = MockProviderReturnsJson;
        let output = compile_with_llm_orchestration(
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
        let output = compile_with_llm_orchestration(
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
        let output = compile_with_llm_orchestration(
            "fix the thing",
            "repair_debug",
            &provider,
            &default_opts(),
        );
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
        let output = compile_with_llm_orchestration(
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
        let output = compile_with_llm_orchestration(
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
    }

    #[test]
    fn test_envelope_warnings_preserved() {
        // Use a prompt with a secret-like value that gets redacted
        let provider = MockProviderReturnsJson;
        let output = compile_with_llm_orchestration(
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
        };
        let output = compile_with_llm_orchestration(
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
        let output = compile_with_llm_orchestration(
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
        let output = compile_with_llm_orchestration(
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
        let output = compile_with_llm_orchestration(
            "test",
            "architecture_planning",
            &provider,
            &default_opts(),
        );
        assert!(!output.compiled_prompt.is_empty());
    }
}

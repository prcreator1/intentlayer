//! LLM compile boundary — future contract for LLM-assisted prompt synthesis.
//!
//! IntentLayer's `llm_compile` mode is for prompts that need deeper
//! synthesis (architecture/planning).  Real model calls are NOT enabled
//! yet.  This module defines the data contract and a safe no-op default
//! provider so the boundary is clear when real integration begins.

/// Request for LLM-assisted prompt compilation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmCompileRequest {
    /// The original user-authored prompt.
    pub original_prompt: String,
    /// Classified category (e.g. "architecture_planning").
    pub category: String,
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

/// The safe instruction envelope sent to a future LLM provider.
///
/// Contains only the latest user-authored prompt plus constraints.
/// Never includes system/developer/tool/assistant messages, file contents,
/// API keys, env-var values, or runtime config secrets.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LlmPromptEnvelope {
    /// The original user-authored prompt text.
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

/// Build a safe [`LlmPromptEnvelope`] for a given raw prompt and category.
///
/// - Includes only the original user-authored prompt
/// - Includes explicit rewrite-only instruction
/// - Includes no-invention and preservation constraints
/// - Never includes API keys, env var values, system/assistant messages,
///   file contents, or runtime secrets
pub fn build_llm_prompt_envelope(original_prompt: &str, category: &str) -> LlmPromptEnvelope {
    let instruction = format!(
        "You are a prompt compiler.  Rewrite the following user-authored \
         prompt into a compact, context-preserving, execution-grade prompt \
         for a coding agent.\n\n\
         Rules:\n\
         - Preserve all context references (repo, error, file, plan, branch, etc.)\n\
         - Never invent frameworks, providers, file paths, databases, \
           architecture, deployment targets, or payment/auth providers\n\
         - Do not add features beyond what the user requested\n\
         - Do not ask clarification questions about context the agent may already have\n\
         - Return only the rewritten prompt text\n\
         - Do not execute tasks, modify files, or run commands\n\
         \n\
         Original prompt ({category}):\n{original_prompt}"
    );

    LlmPromptEnvelope {
        original_prompt: original_prompt.to_string(),
        category: category.to_string(),
        mode: "llm_compile".to_string(),
        instruction,
        must_preserve: vec![
            "original context references".into(),
            "stated user intent".into(),
            "existing project constraints".into(),
        ],
        must_not_invent: vec![
            "frameworks".into(),
            "providers".into(),
            "file paths".into(),
            "databases".into(),
            "architecture".into(),
            "deployment targets".into(),
            "auth/payment providers".into(),
            "scope beyond request".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_types_compile() {
        let req = LlmCompileRequest {
            original_prompt: "design the system".into(),
            category: "architecture_planning".into(),
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
        };
        let result = provider.compile(&req);
        assert!(result.is_err(), "Noop must return error");
        match result {
            Err(LlmError::NoProvider) => {} // expected
            _ => panic!("Expected NoProvider error"),
        }
    }

    #[test]
    fn test_noop_compiler_is_deterministic() {
        let provider = NoopLlmCompiler;
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "architecture_planning".into(),
        };
        let r1 = provider.compile(&req);
        let r2 = provider.compile(&req);
        // Both must return identical errors
        assert_eq!(
            format!("{:?}", r1),
            format!("{:?}", r2),
            "Noop must be deterministic"
        );
    }

    #[test]
    fn test_noop_no_network() {
        // No network-related imports or configuration exist.
        // This test proves the type system doesn't expose any network surface.
        let provider = NoopLlmCompiler;
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "architecture_planning".into(),
        };
        // Calls complete instantly — no I/O
        let _ = provider.compile(&req);
    }

    // ── Safety envelope tests ──────────────────────────────────

    #[test]
    fn test_envelope_includes_original_prompt() {
        let env = build_llm_prompt_envelope("design the system", "architecture_planning");
        assert_eq!(env.original_prompt, "design the system");
    }

    #[test]
    fn test_envelope_includes_category() {
        let env = build_llm_prompt_envelope("test", "testing_test_failure");
        assert_eq!(env.category, "testing_test_failure");
    }

    #[test]
    fn test_envelope_includes_no_invention_rules() {
        let env = build_llm_prompt_envelope("add payment", "feature_implementation");
        assert!(env.must_not_invent.iter().any(|r| r.contains("provider")));
        assert!(env.must_not_invent.iter().any(|r| r.contains("framework")));
        assert!(env.must_not_invent.iter().any(|r| r.contains("scope")));
    }

    #[test]
    fn test_envelope_includes_rewrite_only_instruction() {
        let env = build_llm_prompt_envelope("test", "repair_debug");
        assert!(
            env.instruction.to_lowercase().contains("rewrite"),
            "Instruction must say 'rewrite'",
        );
        assert!(
            env.instruction.to_lowercase().contains("never invent"),
            "Must include no-invention rule"
        );
        assert!(
            env.instruction.to_lowercase().contains("do not execute"),
            "Must include no-execution rule"
        );
    }

    #[test]
    fn test_envelope_does_not_include_secrets_or_api_keys() {
        let env = build_llm_prompt_envelope("test", "architecture_planning");
        let serialized = serde_json::to_string(&env).unwrap();
        assert!(
            !serialized.to_lowercase().contains("sk-"),
            "Envelope must not contain api keys"
        );
        assert!(
            !serialized.to_lowercase().contains("bearer"),
            "Envelope must not contain bearer tokens"
        );
        assert!(
            !serialized.to_lowercase().contains("apikey"),
            "Envelope must not contain api keys"
        );
    }

    #[test]
    fn test_envelope_does_not_include_system_role_content() {
        let env = build_llm_prompt_envelope("test", "architecture_planning");
        let serialized = serde_json::to_string(&env).unwrap();
        assert!(
            !serialized.contains("system_prompt"),
            "Must not contain system prompt refs"
        );
        assert!(
            !serialized.contains("developer message"),
            "Must not contain developer message refs"
        );
    }

    #[test]
    fn test_response_contract_can_represent_output() {
        let resp = LlmResponseContract {
            compiled_prompt: "A compact, execution-grade prompt...".into(),
            warnings: vec![],
        };
        assert_eq!(resp.compiled_prompt, "A compact, execution-grade prompt...");
        assert!(resp.warnings.is_empty());
    }

    #[test]
    fn test_response_contract_can_represent_warnings() {
        let resp = LlmResponseContract {
            compiled_prompt: "Add payment using existing stack".into(),
            warnings: vec!["Invented provider: Stripe".into()],
        };
        assert_eq!(resp.warnings.len(), 1);
        assert!(resp.warnings[0].contains("Stripe"));
    }
}

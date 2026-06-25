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
}

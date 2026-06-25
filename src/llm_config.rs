//! Runtime LLM provider configuration — future LLM-assist support.
//!
//! Defines typed provider configs with a strict security rule:
//! raw API keys are read at runtime from environment variables only.
//! Config files may store env-var names, never raw secret values.
//!
//! No real API calls are made yet.  This module is future-ready
//! scaffolding.

use std::env;

// ── Configuration types ──────────────────────────────────────────────

/// Static provider configuration as it might appear in a config file.
/// Never stores raw secrets.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmProviderConfig {
    /// Provider identifier (e.g. "openai-compatible", "ollama").
    pub provider: String,
    /// Optional base URL override.
    pub base_url: Option<String>,
    /// Model name to request.
    pub model: String,
    /// Name of environment variable holding the API key, if any.
    /// For local providers (Ollama), this may be absent.
    pub api_key_env: Option<String>,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
    /// Maximum output tokens.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
}

/// Resolved configuration with optional API key loaded at runtime.
///
/// Debug output intentionally REDACTS the api_key field.
#[derive(Clone, serde::Serialize)]
pub struct ResolvedLlmProviderConfig {
    pub provider: String,
    pub base_url: Option<String>,
    pub model: String,
    /// Resolved from env at runtime.  NEVER serialized or logged.
    #[serde(skip)]
    pub api_key: Option<String>,
    pub timeout_seconds: u64,
    pub max_tokens: u32,
    pub temperature: f32,
}

/// Custom Debug: never prints the API key.
impl std::fmt::Debug for ResolvedLlmProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedLlmProviderConfig")
            .field("provider", &self.provider)
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("api_key", &redact_key(&self.api_key))
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_tokens", &self.max_tokens)
            .field("temperature", &self.temperature)
            .finish()
    }
}

// ── Errors ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum LlmConfigError {
    /// The named environment variable is not set.
    MissingEnvVar(String),
    /// The env var was set but empty.
    EmptyEnvVar(String),
}

impl std::fmt::Display for LlmConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmConfigError::MissingEnvVar(name) => {
                write!(f, "Missing environment variable: {}", name)
            }
            LlmConfigError::EmptyEnvVar(name) => {
                write!(f, "Environment variable '{}' is set but empty", name)
            }
        }
    }
}

// ── Runtime resolution ───────────────────────────────────────────────

/// Resolve a static config at runtime by reading API keys from
/// environment variables.  Never stores raw keys in the config struct.
pub fn resolve_from_env(
    config: &LlmProviderConfig,
) -> Result<ResolvedLlmProviderConfig, LlmConfigError> {
    let api_key = match &config.api_key_env {
        Some(env_name) => {
            let val =
                env::var(env_name).map_err(|_| LlmConfigError::MissingEnvVar(env_name.clone()))?;
            let trimmed = val.trim().to_string();
            if trimmed.is_empty() {
                return Err(LlmConfigError::EmptyEnvVar(env_name.clone()));
            }
            Some(trimmed)
        }
        None => None,
    };

    Ok(ResolvedLlmProviderConfig {
        provider: config.provider.clone(),
        base_url: config.base_url.clone(),
        model: config.model.clone(),
        api_key,
        timeout_seconds: config.timeout_seconds,
        max_tokens: config.max_tokens,
        temperature: config.temperature,
    })
}

// ── Redaction helpers ────────────────────────────────────────────────

/// Redact an optional API key for safe display.
fn redact_key(key: &Option<String>) -> &'static str {
    match key {
        Some(_) => "[REDACTED]",
        None => "none",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn openai_config() -> LlmProviderConfig {
        LlmProviderConfig {
            provider: "openai-compatible".into(),
            base_url: Some("https://api.openai.com/v1".into()),
            model: "gpt-4.1-mini".into(),
            api_key_env: Some("OPENAI_API_KEY".into()),
            timeout_seconds: 30,
            max_tokens: 800,
            temperature: 0.1,
        }
    }

    fn ollama_config() -> LlmProviderConfig {
        LlmProviderConfig {
            provider: "ollama".into(),
            base_url: Some("http://localhost:11434/v1".into()),
            model: "qwen2.5-coder".into(),
            api_key_env: None,
            timeout_seconds: 30,
            max_tokens: 800,
            temperature: 0.1,
        }
    }

    #[test]
    fn test_config_represents_openai_compatible() {
        let cfg = openai_config();
        assert_eq!(cfg.provider, "openai-compatible");
        assert_eq!(cfg.api_key_env.as_deref(), Some("OPENAI_API_KEY"));
    }

    #[test]
    fn test_config_represents_local_no_key_provider() {
        let cfg = ollama_config();
        assert_eq!(cfg.provider, "ollama");
        assert!(
            cfg.api_key_env.is_none(),
            "Local provider should have no key env"
        );
    }

    #[test]
    fn test_api_key_env_resolves_from_environment() {
        // Use a unique env var name to avoid collisions
        env::set_var("INTENTLAYER_TEST_KEY_013", "test-secret-value");
        let cfg = LlmProviderConfig {
            api_key_env: Some("INTENTLAYER_TEST_KEY_013".into()),
            ..ollama_config()
        };
        let resolved = resolve_from_env(&cfg).expect("Should resolve");
        assert_eq!(resolved.api_key.as_deref(), Some("test-secret-value"));
        env::remove_var("INTENTLAYER_TEST_KEY_013");
    }

    #[test]
    fn test_missing_api_key_env_returns_typed_error() {
        let cfg = LlmProviderConfig {
            api_key_env: Some("DOES_NOT_EXIST_ZZZZ".into()),
            ..ollama_config()
        };
        let err = resolve_from_env(&cfg).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("DOES_NOT_EXIST_ZZZZ"),
            "Error should name the env var: {}",
            msg
        );
        assert!(!msg.contains("sk-"), "Error must not contain example keys");
    }

    #[test]
    fn test_resolved_debug_does_not_expose_raw_key() {
        env::set_var("INTENTLAYER_DEBUG_TEST_KEY", "secret-abc-123");
        let cfg = LlmProviderConfig {
            api_key_env: Some("INTENTLAYER_DEBUG_TEST_KEY".into()),
            ..ollama_config()
        };
        let resolved = resolve_from_env(&cfg).unwrap();
        let debug_str = format!("{:?}", resolved);
        assert!(
            !debug_str.contains("secret-abc-123"),
            "Debug must not contain raw key"
        );
        assert!(
            debug_str.contains("[REDACTED]"),
            "Debug must show redacted marker"
        );
        env::remove_var("INTENTLAYER_DEBUG_TEST_KEY");
    }

    #[test]
    fn test_error_messages_do_not_expose_raw_key() {
        // Error from missing env var should only mention the env var name
        let cfg = LlmProviderConfig {
            api_key_env: Some("MISSING_KEY_ZZZ".into()),
            ..ollama_config()
        };
        let err = resolve_from_env(&cfg).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("MISSING_KEY_ZZZ"), "Error should name env var");
        assert!(
            !msg.to_lowercase().contains("sk-"),
            "Error must not look like a key"
        );
        assert!(
            !msg.to_lowercase().contains("bearer"),
            "Error must not contain token words"
        );
    }

    #[test]
    fn test_no_network_api_call_made() {
        // Resolution only reads env — no I/O beyond that
        let cfg = ollama_config(); // no api_key_env
        let resolved = resolve_from_env(&cfg).expect("Should resolve without network");
        assert!(resolved.api_key.is_none());
    }

    #[test]
    fn test_existing_llm_boundary_tests_still_pass() {
        use crate::llm::LlmProvider;
        // Re-verify the NoopLlmCompiler from the llm module
        // This test import ensures the boundary module is still reachable.
        let req = crate::llm::LlmCompileRequest {
            original_prompt: "test".into(),
            category: "architecture_planning".into(),
        };
        let provider = crate::llm::NoopLlmCompiler;
        assert!(provider.compile(&req).is_err());
    }
}

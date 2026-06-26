//! Provider registry — central source of truth for LLM provider metadata,
//! validation, and routing helpers.
//!
//! Avoids scattering `if provider == "openrouter"` / `match provider`
//! across the CLI as new providers are added.

/// Known LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    OpenRouter,
    Groq,
}

impl ProviderKind {
    /// CLI provider name string.
    pub fn name(&self) -> &'static str {
        match self {
            ProviderKind::OpenRouter => "openrouter",
            ProviderKind::Groq => "groq",
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderKind::OpenRouter => "OpenRouter",
            ProviderKind::Groq => "Groq",
        }
    }

    /// Default API base URL.
    pub fn default_base_url(&self) -> &'static str {
        match self {
            ProviderKind::OpenRouter => "https://openrouter.ai/api/v1",
            ProviderKind::Groq => "https://api.groq.com/openai/v1",
        }
    }

    /// Default model.
    pub fn default_model(&self) -> &'static str {
        match self {
            ProviderKind::OpenRouter => "gpt-4.1-mini",
            ProviderKind::Groq => "llama-3.3-70b-versatile",
        }
    }

    /// Cargo feature name required for HTTP transport.
    pub fn feature_name(&self) -> &'static str {
        match self {
            ProviderKind::OpenRouter => "openrouter-http",
            ProviderKind::Groq => "groq-http",
        }
    }
}

// ── Registry helpers ─────────────────────────────────────────────────

/// Parse a CLI provider string into a [`ProviderKind`].
pub fn parse_provider(name: &str) -> Result<ProviderKind, ProviderRegistryError> {
    match name {
        "openrouter" => Ok(ProviderKind::OpenRouter),
        "groq" => Ok(ProviderKind::Groq),
        other => Err(ProviderRegistryError::UnsupportedProvider(other.into())),
    }
}

/// List of all supported provider name strings.
pub fn supported_provider_names() -> &'static [&'static str] {
    &["openrouter", "groq"]
}

/// Formatted error string listing supported providers.
pub fn supported_provider_list_for_error() -> String {
    supported_provider_names().join(", ")
}

// ── Error ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ProviderRegistryError {
    UnsupportedProvider(String),
}

impl std::fmt::Display for ProviderRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderRegistryError::UnsupportedProvider(name) => {
                write!(
                    f,
                    "unsupported LLM provider '{}'. Supported providers: {}",
                    name,
                    supported_provider_list_for_error()
                )
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_list_contains_both() {
        let names = supported_provider_names();
        assert!(names.contains(&"openrouter"));
        assert!(names.contains(&"groq"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_parse_accepts_openrouter() {
        assert_eq!(
            parse_provider("openrouter").unwrap(),
            ProviderKind::OpenRouter
        );
    }

    #[test]
    fn test_parse_accepts_groq() {
        assert_eq!(parse_provider("groq").unwrap(), ProviderKind::Groq);
    }

    #[test]
    fn test_parse_rejects_typo() {
        let err = parse_provider("typo").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unsupported"));
        assert!(msg.contains("openrouter"));
        assert!(msg.contains("groq"));
    }

    #[test]
    fn test_error_includes_unsupported_name() {
        let err = parse_provider("bad-provider-123").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("bad-provider-123"),
            "Error must name the unsupported provider"
        );
        assert!(
            msg.contains("openrouter"),
            "Error must list supported providers"
        );
    }
}

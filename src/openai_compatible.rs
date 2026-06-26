//! Shared OpenAI-compatible provider types and helpers.
//!
//! Used by OpenRouter, Groq, and future OpenAI-compatible providers.
//! Each provider keeps its own request struct (different token fields,
//! optional fields) but shares response types, extraction, and sanitization.

use serde::{Deserialize, Serialize};

// ── Shared response types ────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub message: ResponseMessage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseMessage {
    pub content: Option<String>,
}

// ── Shared message type ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// ── Shared error type ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ProviderError {
    MissingApiKey(String),
    TransportFailed(String),
    InvalidResponse(String),
    EmptyChoices,
    EmptyMessageContent,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::MissingApiKey(name) => {
                write!(f, "{} API key not configured", name)
            }
            ProviderError::TransportFailed(msg) => write!(f, "transport failed: {}", msg),
            ProviderError::InvalidResponse(msg) => write!(f, "invalid response: {}", msg),
            ProviderError::EmptyChoices => write!(f, "response contained no choices"),
            ProviderError::EmptyMessageContent => write!(f, "response message content is empty"),
        }
    }
}

// ── Shared helpers ────────────────────────────────────────────────────

/// Extract content from first choice, returning a sanitized error on failure.
pub fn extract_choice_content(resp: &ChatResponse) -> Result<String, ProviderError> {
    let choice = resp.choices.first().ok_or(ProviderError::EmptyChoices)?;
    let content = choice
        .message
        .content
        .as_deref()
        .ok_or(ProviderError::EmptyMessageContent)?;
    Ok(content.to_string())
}

/// Build system + user messages from an LlmCompileRequest envelope.
pub fn build_envelope_messages(
    category: &str,
    instruction: &str,
    original_prompt: &str,
    must_preserve: &[String],
    must_not_invent: &[String],
    json_instruction: &str,
) -> Vec<Message> {
    let system_msg = format!(
        "You are IntentLayer, a prompt compiler. Your only job is to rewrite \
         the user prompt into a compact, context-preserving, execution-grade prompt.\n\n\
         Category: {}\n\n\
         Must preserve:\n{}\n\n\
         Must never invent:\n{}\n\n\
         {}",
        category,
        must_preserve.join("\n- "),
        must_not_invent.join("\n- "),
        json_instruction,
    );

    let user_msg = format!(
        "Instruction:\n{}\n\nRedacted original prompt:\n{}",
        instruction, original_prompt
    );

    vec![
        Message {
            role: "system".into(),
            content: system_msg,
        },
        Message {
            role: "user".into(),
            content: user_msg,
        },
    ]
}

/// Sanitize an HTTP status code into a short provider error.
pub fn sanitize_http_status(status: u16) -> ProviderError {
    match status {
        401 => ProviderError::TransportFailed("unauthorized (401)".into()),
        402 => ProviderError::TransportFailed("payment required (402)".into()),
        408 => ProviderError::TransportFailed("request timed out (408)".into()),
        413 => ProviderError::TransportFailed("request too large (413)".into()),
        429 => ProviderError::TransportFailed("rate limited (429)".into()),
        500..=599 => ProviderError::TransportFailed("upstream service unavailable (5xx)".into()),
        _ => ProviderError::TransportFailed(format!("HTTP {}", status)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_choice_content_returns_content() {
        let resp = ChatResponse {
            model: "t".into(),
            choices: vec![Choice {
                message: ResponseMessage {
                    content: Some("safe output".into()),
                },
            }],
        };
        assert_eq!(extract_choice_content(&resp).unwrap(), "safe output");
    }

    #[test]
    fn test_empty_choices_returns_error() {
        let resp = ChatResponse {
            model: "t".into(),
            choices: vec![],
        };
        assert!(extract_choice_content(&resp).is_err());
    }

    #[test]
    fn test_empty_content_returns_error() {
        let resp = ChatResponse {
            model: "t".into(),
            choices: vec![Choice {
                message: ResponseMessage { content: None },
            }],
        };
        assert!(extract_choice_content(&resp).is_err());
    }

    #[test]
    fn test_sanitize_http_401_returns_unauthorized() {
        let err = sanitize_http_status(401);
        assert!(err.to_string().contains("401"));
        assert!(!err.to_string().contains("Bearer"));
        assert!(!err.to_string().contains("api_key"));
    }

    #[test]
    fn test_sanitize_http_5xx() {
        let err = sanitize_http_status(502);
        assert!(err.to_string().contains("5xx"));
    }

    #[test]
    fn test_missing_api_key_error_includes_provider_name() {
        let err = ProviderError::MissingApiKey("test-provider".into());
        assert!(err.to_string().contains("test-provider"));
    }

    #[test]
    fn test_build_envelope_messages_includes_category() {
        let msgs = build_envelope_messages("repair_debug", "rewrite", "fix bug", &[], &[], "");
        assert!(msgs[0].content.contains("repair_debug"));
    }

    #[test]
    fn test_build_envelope_messages_includes_prompt() {
        let msgs = build_envelope_messages("test", "rewrite", "fix bug", &[], &[], "");
        assert!(msgs[1].content.contains("fix bug"));
    }
}

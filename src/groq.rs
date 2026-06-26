//! Groq provider adapter — Groq API integration for LLM-assisted compilation.
//!
//! Groq is OpenAI-compatible. Base URL: https://api.groq.com/openai/v1
//! Implements [`LlmProvider`] via a transport abstraction.
//! No live API calls in tests. No API keys committed.

use crate::llm::{LlmCompileRequest, LlmCompileResponse, LlmError, LlmProvider};
use crate::llm_config::ResolvedLlmProviderConfig;
#[allow(unused_imports)]
use crate::openai_compatible::build_envelope_messages;
use serde::Serialize;

// ── Groq request/response types ──────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct GroqChatRequest {
    pub model: String,
    pub messages: Vec<GroqMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    pub stream: bool,
}

pub use crate::openai_compatible::Message as GroqMessage;

pub use crate::openai_compatible::{
    ChatResponse as GroqChatResponse, Choice as GroqChoice, ResponseMessage as GroqResponseMessage,
};

// ── Errors ───────────────────────────────────────────────────────────

pub use crate::openai_compatible::ProviderError as GroqError;

// ── Transport ────────────────────────────────────────────────────────

pub trait GroqTransport {
    fn send(&self, request: &GroqChatRequest, api_key: &str)
        -> Result<GroqChatResponse, GroqError>;
}

// ── Request builder ──────────────────────────────────────────────────

pub fn build_groq_request(
    llm_req: &LlmCompileRequest,
    config: &ResolvedLlmProviderConfig,
) -> GroqChatRequest {
    let messages = build_envelope_messages(
        &llm_req.category,
        &llm_req.instruction,
        &llm_req.original_prompt,
        &llm_req.must_preserve,
        &llm_req.must_not_invent,
        "Return only valid JSON: {\"compiled_prompt\":\"...\",\"warnings\":[]}",
    );

    GroqChatRequest {
        model: config.model.clone(),
        messages,
        temperature: Some(config.temperature),
        max_completion_tokens: Some(config.max_tokens),
        stream: false,
    }
}

// ── Provider ─────────────────────────────────────────────────────────

pub struct GroqProvider<T: GroqTransport> {
    config: ResolvedLlmProviderConfig,
    pub transport: T,
}

impl<T: GroqTransport> GroqProvider<T> {
    pub fn new(config: ResolvedLlmProviderConfig, transport: T) -> Self {
        GroqProvider { config, transport }
    }
}

impl<T: GroqTransport> LlmProvider for GroqProvider<T> {
    fn compile(&self, request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
        let api_key = self.config.api_key.as_deref().ok_or_else(|| {
            LlmError::ProviderError(GroqError::MissingApiKey("Groq".into()).to_string())
        })?;

        let groq_request = build_groq_request(request, &self.config);

        match self.transport.send(&groq_request, api_key) {
            Ok(resp) => {
                let choice = resp
                    .choices
                    .first()
                    .ok_or_else(|| LlmError::ProviderError(GroqError::EmptyChoices.to_string()))?;
                let content = choice.message.content.as_deref().ok_or_else(|| {
                    LlmError::ProviderError(GroqError::EmptyMessageContent.to_string())
                })?;
                Ok(LlmCompileResponse {
                    compiled_prompt: content.to_string(),
                    warnings: vec![],
                })
            }
            Err(e) => Err(LlmError::ProviderError(e.to_string())),
        }
    }
}

// ── HTTP transport (feature-gated) ──────────────────────────────────

#[cfg(feature = "groq-http")]
pub use http_transport::ReqwestGroqTransport;

#[cfg(feature = "groq-http")]
mod http_transport {
    use super::*;

    pub struct ReqwestGroqTransport {
        client: reqwest::blocking::Client,
        base_url: String,
    }

    impl ReqwestGroqTransport {
        pub fn new(config: &ResolvedLlmProviderConfig) -> Result<Self, GroqError> {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(config.timeout_seconds))
                .build()
                .map_err(|e| GroqError::TransportFailed(format!("transport init failed: {}", e)))?;
            let base_url = config
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.groq.com/openai/v1".into());
            Ok(ReqwestGroqTransport { client, base_url })
        }
    }

    impl GroqTransport for ReqwestGroqTransport {
        fn send(
            &self,
            request: &GroqChatRequest,
            api_key: &str,
        ) -> Result<GroqChatResponse, GroqError> {
            let url = format!("{}/chat/completions", self.base_url);
            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(request)
                .send()
                .map_err(|e| GroqError::TransportFailed(format!("request failed: {}", e)))?;

            let status = resp.status();
            if status.is_client_error() || status.is_server_error() {
                return Err(GroqError::TransportFailed(format!(
                    "HTTP {}",
                    status.as_u16()
                )));
            }

            resp.json()
                .map_err(|e| GroqError::InvalidResponse(format!("parse failed: {}", e)))
        }
    }
}

// ── Mock transport ──────────────────────────────────────────────────

pub struct MockGroqTransport {
    pub response: std::cell::RefCell<Result<GroqChatResponse, GroqError>>,
    pub captured_request: std::cell::RefCell<Option<GroqChatRequest>>,
}

impl MockGroqTransport {
    pub fn new(response: Result<GroqChatResponse, GroqError>) -> Self {
        MockGroqTransport {
            response: std::cell::RefCell::new(response),
            captured_request: std::cell::RefCell::new(None),
        }
    }
}

impl GroqTransport for MockGroqTransport {
    fn send(
        &self,
        request: &GroqChatRequest,
        _api_key: &str,
    ) -> Result<GroqChatResponse, GroqError> {
        *self.captured_request.borrow_mut() = Some(request.clone());
        self.response.borrow().clone()
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_config() -> ResolvedLlmProviderConfig {
        ResolvedLlmProviderConfig {
            provider: "groq".into(),
            base_url: Some("https://api.groq.com/openai/v1".into()),
            model: "llama-3.3-70b-versatile".into(),
            api_key: Some("fake-groq-key".into()),
            timeout_seconds: 30,
            max_tokens: 800,
            temperature: 0.1,
        }
    }

    fn make_req() -> LlmCompileRequest {
        LlmCompileRequest {
            original_prompt: "design retry wrapper".into(),
            category: "architecture_planning".into(),
            instruction: "rewrite this".into(),
            must_preserve: vec!["context".into()],
            must_not_invent: vec!["frameworks".into()],
        }
    }

    #[test]
    fn test_groq_request_uses_correct_base_url() {
        let req = make_req();
        let gr = build_groq_request(&req, &mock_config());
        assert_eq!(gr.model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn test_groq_request_includes_messages() {
        let req = make_req();
        let gr = build_groq_request(&req, &mock_config());
        assert_eq!(gr.messages.len(), 2);
        assert!(gr.messages[0].content.contains("context"));
        assert!(gr.messages[1].content.contains("design retry wrapper"));
    }

    #[test]
    fn test_groq_request_no_unsupported_fields() {
        let req = make_req();
        let gr = build_groq_request(&req, &mock_config());
        let body = serde_json::to_string(&gr).unwrap();
        assert!(!body.contains("logprobs"));
        assert!(!body.contains("top_logprobs"));
        assert!(!body.contains("response_format"));
        assert!(!body.contains("provider"));
    }

    #[test]
    fn test_groq_request_uses_max_completion_tokens() {
        let req = make_req();
        let gr = build_groq_request(&req, &mock_config());
        assert_eq!(gr.max_completion_tokens, Some(800));
    }

    #[test]
    fn test_groq_parses_choices_content() {
        let transport = MockGroqTransport::new(Ok(GroqChatResponse {
            model: "test".into(),
            choices: vec![GroqChoice {
                message: GroqResponseMessage {
                    content: Some("compiled output".into()),
                },
            }],
        }));
        let provider = GroqProvider::new(mock_config(), transport);
        let resp = provider.compile(&make_req()).unwrap();
        assert_eq!(resp.compiled_prompt, "compiled output");
    }

    #[test]
    fn test_groq_missing_key_returns_error() {
        let cfg = ResolvedLlmProviderConfig {
            api_key: None,
            ..mock_config()
        };
        let transport = MockGroqTransport::new(Ok(GroqChatResponse {
            model: "t".into(),
            choices: vec![GroqChoice {
                message: GroqResponseMessage {
                    content: Some("x".into()),
                },
            }],
        }));
        let provider = GroqProvider::new(cfg, transport);
        let err = provider.compile(&make_req()).unwrap_err();
        assert!(err.to_string().contains("not configured"));
        assert!(!err.to_string().contains("fake-groq-key"));
    }

    #[test]
    fn test_groq_error_sanitized() {
        let transport = MockGroqTransport::new(Err(GroqError::TransportFailed("timeout".into())));
        let provider = GroqProvider::new(mock_config(), transport);
        let err = provider.compile(&make_req()).unwrap_err();
        assert!(err.to_string().contains("timeout"));
        assert!(!err.to_string().contains("fake-groq-key"));
    }
}

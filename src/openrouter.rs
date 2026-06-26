//! OpenRouter provider adapter — concrete LLM provider using the OpenRouter API.
//!
//! Implements [`LlmProvider`] via a transport abstraction.
//! No live API calls are made in tests.  No API keys are committed.
//! This is an explicit opt-in provider for Phase 016 orchestration.

use crate::llm::{LlmCompileRequest, LlmCompileResponse, LlmError, LlmProvider};
use crate::llm_config::ResolvedLlmProviderConfig;
#[allow(unused_imports)]
use crate::openai_compatible::build_envelope_messages;

// ── OpenRouter request/response types ────────────────────────────────

/// OpenRouter chat completion request body.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpenRouterChatRequest {
    pub model: String,
    pub messages: Vec<OpenRouterMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<OpenRouterResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<OpenRouterProviderConfig>,
}

pub use crate::openai_compatible::Message as OpenRouterMessage;

#[derive(Debug, Clone, serde::Serialize)]
pub struct OpenRouterResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    pub json_schema: Option<OpenRouterJsonSchema>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OpenRouterJsonSchema {
    pub name: String,
    pub strict: bool,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OpenRouterProviderConfig {
    pub require_parameters: bool,
}

// ── Re-exports from shared core ──────────────────────────────────────
pub use crate::openai_compatible::ProviderError as OpenRouterError;
pub use crate::openai_compatible::{
    ChatResponse as OpenRouterChatResponse, Choice as OpenRouterChoice,
    ResponseMessage as OpenRouterResponseMessage,
};

// ── Transport abstraction ───────────────────────────────────────────

/// Transport for sending OpenRouter requests.
/// Mock this for tests; real HTTP implementation deferred to future phase.
pub trait OpenRouterTransport {
    fn send(
        &self,
        request: &OpenRouterChatRequest,
        api_key: &str,
    ) -> Result<OpenRouterChatResponse, OpenRouterError>;
}

// ── Request builder ──────────────────────────────────────────────────

/// Build an OpenRouter chat request from an [`LlmCompileRequest`].
/// The request preserves the full safety envelope: instruction, redacted
/// original prompt, category, constraints.
pub fn build_openrouter_request(
    llm_req: &LlmCompileRequest,
    config: &ResolvedLlmProviderConfig,
) -> OpenRouterChatRequest {
    let messages = build_envelope_messages(
        &llm_req.category,
        &llm_req.instruction,
        &llm_req.original_prompt,
        &llm_req.must_preserve,
        &llm_req.must_not_invent,
        "Return only valid JSON matching: {\"compiled_prompt\":\"...\",\"warnings\":[]}. No markdown.",
    );

    let response_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "compiled_prompt": {
                "type": "string",
                "description": "The rewritten, compact, execution-grade prompt"
            },
            "warnings": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Any warnings about unsafe or invented content"
            }
        },
        "required": ["compiled_prompt", "warnings"],
        "additionalProperties": false
    });

    OpenRouterChatRequest {
        model: config.model.clone(),
        messages,
        temperature: Some(config.temperature),
        max_tokens: Some(config.max_tokens),
        stream: false,
        response_format: Some(OpenRouterResponseFormat {
            format_type: "json_schema".into(),
            json_schema: Some(OpenRouterJsonSchema {
                name: "intentlayer_compiled_prompt".into(),
                strict: true,
                schema: response_schema,
            }),
        }),
        provider: Some(OpenRouterProviderConfig {
            require_parameters: true,
        }),
    }
}

// ── Provider implementation ──────────────────────────────────────────

pub struct OpenRouterProvider<T: OpenRouterTransport> {
    config: ResolvedLlmProviderConfig,
    pub transport: T, // public so tests can inspect call state
}

impl<T: OpenRouterTransport> OpenRouterProvider<T> {
    pub fn new(config: ResolvedLlmProviderConfig, transport: T) -> Self {
        OpenRouterProvider { config, transport }
    }
}

impl<T: OpenRouterTransport> LlmProvider for OpenRouterProvider<T> {
    fn compile(&self, request: &LlmCompileRequest) -> Result<LlmCompileResponse, LlmError> {
        let api_key = self.config.api_key.as_deref().ok_or_else(|| {
            LlmError::ProviderError(OpenRouterError::MissingApiKey("OpenRouter".into()).to_string())
        })?;

        let or_request = build_openrouter_request(request, &self.config);

        match self.transport.send(&or_request, api_key) {
            Ok(resp) => {
                let content = crate::openai_compatible::extract_choice_content(&resp)
                    .map_err(|e| LlmError::ProviderError(e.to_string()))?;
                Ok(LlmCompileResponse {
                    compiled_prompt: content,
                    warnings: vec![],
                })
            }
            Err(e) => Err(LlmError::ProviderError(e.to_string())),
        }
    }
}

// ── Mock transport for tests ─────────────────────────────────────────

pub struct MockTransport {
    pub response: std::cell::RefCell<Result<OpenRouterChatResponse, OpenRouterError>>,
    pub captured_request: std::cell::RefCell<Option<OpenRouterChatRequest>>,
    pub captured_api_key: std::cell::RefCell<Option<String>>,
}

impl MockTransport {
    pub fn new(response: Result<OpenRouterChatResponse, OpenRouterError>) -> Self {
        MockTransport {
            response: std::cell::RefCell::new(response),
            captured_request: std::cell::RefCell::new(None),
            captured_api_key: std::cell::RefCell::new(None),
        }
    }
}

impl OpenRouterTransport for MockTransport {
    fn send(
        &self,
        request: &OpenRouterChatRequest,
        api_key: &str,
    ) -> Result<OpenRouterChatResponse, OpenRouterError> {
        *self.captured_request.borrow_mut() = Some(request.clone());
        *self.captured_api_key.borrow_mut() = Some(api_key.to_string());
        self.response.borrow().clone()
    }
}

// ── Feature-gated HTTP transport ─────────────────────────────────────

#[cfg(feature = "openrouter-http")]
pub use http_transport::ReqwestOpenRouterTransport;

#[cfg(feature = "openrouter-http")]
mod http_transport {
    use super::*;

    pub struct ReqwestOpenRouterTransport {
        client: reqwest::blocking::Client,
        base_url: String,
    }

    impl ReqwestOpenRouterTransport {
        pub fn new(config: &ResolvedLlmProviderConfig) -> Result<Self, OpenRouterError> {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(config.timeout_seconds))
                .build()
                .map_err(|e| OpenRouterError::TransportFailed(sanitize_reqwest_error(&e)))?;
            let base_url = config
                .base_url
                .clone()
                .unwrap_or_else(|| "https://openrouter.ai/api/v1".into());
            Ok(ReqwestOpenRouterTransport { client, base_url })
        }
    }

    impl OpenRouterTransport for ReqwestOpenRouterTransport {
        fn send(
            &self,
            request: &OpenRouterChatRequest,
            api_key: &str,
        ) -> Result<OpenRouterChatResponse, OpenRouterError> {
            let url = format!("{}/chat/completions", self.base_url);
            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(request)
                .send()
                .map_err(|e| OpenRouterError::TransportFailed(sanitize_reqwest_error(&e)))?;

            let status = resp.status();
            if status.is_client_error() || status.is_server_error() {
                return Err(map_http_status(status));
            }

            resp.json::<OpenRouterChatResponse>()
                .map_err(|e| OpenRouterError::InvalidResponse(sanitize_json_error(&e)))
        }
    }

    fn sanitize_reqwest_error(e: &reqwest::Error) -> String {
        if e.is_timeout() {
            "request timed out".into()
        } else if e.is_connect() {
            "connection failed".into()
        } else if e.is_decode() {
            "invalid response format".into()
        } else {
            "transport error".into()
        }
    }

    fn sanitize_json_error(_e: &reqwest::Error) -> String {
        "response could not be parsed as valid JSON".into()
    }

    fn map_http_status(status: reqwest::StatusCode) -> OpenRouterError {
        crate::openai_compatible::sanitize_http_status(status.as_u16())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Test that the filtered error messages contain safe HTTP statuses but no URL.
        #[test]
        fn test_map_http_status_401_returns_unauthorized() {
            let err = map_http_status(reqwest::StatusCode::UNAUTHORIZED);
            let msg = err.to_string();
            assert!(msg.contains("401"));
            assert!(!msg.contains("Bearer"));
        }

        #[test]
        fn test_map_http_status_429_returns_rate_limited() {
            let err = map_http_status(reqwest::StatusCode::TOO_MANY_REQUESTS);
            assert!(err.to_string().contains("429"));
        }

        #[test]
        fn test_map_http_status_5xx_returns_upstream_unavailable() {
            let err = map_http_status(reqwest::StatusCode::BAD_GATEWAY);
            assert!(err.to_string().contains("upstream"));
        }
    }
}

// ── Tests (mock, always compiled) ────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_config() -> ResolvedLlmProviderConfig {
        ResolvedLlmProviderConfig {
            provider: "openai-compatible".into(),
            base_url: Some("https://openrouter.ai/api/v1".into()),
            model: "test-model".into(),
            api_key: Some("fake-test-key".into()),
            timeout_seconds: 30,
            max_tokens: 400,
            temperature: 0.1,
        }
    }

    #[test]
    fn test_request_includes_model() {
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "repair_debug".into(),
            instruction: "rewrite".into(),
            must_preserve: vec!["ctx".into()],
            must_not_invent: vec!["fw".into()],
        };
        let or = build_openrouter_request(&req, &mock_config());
        assert_eq!(or.model, "test-model");
    }

    #[test]
    fn test_request_includes_instruction() {
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "repair_debug".into(),
            instruction: "rewrite this prompt correctly".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &mock_config());
        let user_msg = &or.messages[1].content;
        assert!(user_msg.contains("rewrite this prompt correctly"));
    }

    #[test]
    fn test_request_includes_redacted_original_prompt() {
        let req = LlmCompileRequest {
            original_prompt: "[REDACTED_SECRET] fix the API".into(),
            category: "repair_debug".into(),
            instruction: "Original prompt: [REDACTED_SECRET] fix the API".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &mock_config());
        let sys = &or.messages[0].content;
        let user = &or.messages[1].content;
        assert!(
            !sys.contains("SECRET"),
            "System msg must not have raw: {}",
            sys
        );
        // Redacted prompt should appear in instruction
        assert!(
            user.contains("[REDACTED_SECRET]"),
            "User msg must contain redacted marker: {}",
            user
        );
    }

    #[test]
    fn test_request_includes_category() {
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "architecture_planning".into(),
            instruction: "rewrite".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &mock_config());
        assert!(or.messages[0].content.contains("architecture_planning"));
    }

    #[test]
    fn test_request_includes_must_preserve() {
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec!["context".into(), "existing stack".into()],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &mock_config());
        assert!(or.messages[0].content.contains("context"));
        assert!(or.messages[0].content.contains("existing stack"));
    }

    #[test]
    fn test_request_includes_must_not_invent() {
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec!["Stripe".into(), "Auth0".into()],
        };
        let or = build_openrouter_request(&req, &mock_config());
        assert!(or.messages[0].content.contains("Stripe"));
        assert!(or.messages[0].content.contains("Auth0"));
    }

    #[test]
    fn test_request_includes_response_format_json_schema() {
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &mock_config());
        let fmt = or.response_format.as_ref().unwrap();
        assert_eq!(fmt.format_type, "json_schema");
        let schema = fmt.json_schema.as_ref().unwrap();
        assert_eq!(schema.name, "intentlayer_compiled_prompt");
        assert!(schema.strict);
    }

    #[test]
    fn test_schema_requires_compiled_prompt_and_warnings() {
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &mock_config());
        let schema = &or.response_format.unwrap().json_schema.unwrap().schema;
        let required: Vec<String> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert!(required.contains(&"compiled_prompt".to_string()));
        assert!(required.contains(&"warnings".to_string()));
    }

    #[test]
    fn test_request_provider_require_parameters_true() {
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &mock_config());
        assert!(or.provider.as_ref().unwrap().require_parameters);
    }

    #[test]
    fn test_request_uses_temperature_from_config() {
        let cfg = ResolvedLlmProviderConfig {
            temperature: 0.3,
            ..mock_config()
        };
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &cfg);
        assert_eq!(or.temperature, Some(0.3));
    }

    #[test]
    fn test_request_uses_max_tokens_from_config() {
        let cfg = ResolvedLlmProviderConfig {
            max_tokens: 1200,
            ..mock_config()
        };
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &cfg);
        assert_eq!(or.max_tokens, Some(1200));
    }

    #[test]
    fn test_missing_api_key_returns_safe_error() {
        let cfg = ResolvedLlmProviderConfig {
            api_key: None,
            ..mock_config()
        };
        let transport = MockTransport::new(Ok(mock_chat_response("ok")));
        let provider = OpenRouterProvider::new(cfg, transport);
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let err = provider.compile(&req).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not configured"));
        assert!(!msg.to_lowercase().contains("fake-test-key"));
    }

    #[test]
    fn test_transport_failure_returns_safe_error() {
        let transport = MockTransport::new(Err(OpenRouterError::TransportFailed("timeout".into())));
        let provider = OpenRouterProvider::new(mock_config(), transport);
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let err = provider.compile(&req).unwrap_err();
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_empty_choices_returns_safe_error() {
        let mut resp = mock_chat_response("");
        resp.choices.clear();
        let transport = MockTransport::new(Ok(resp));
        let provider = OpenRouterProvider::new(mock_config(), transport);
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let err = provider.compile(&req).unwrap_err();
        assert!(err.to_string().contains("no choices"));
    }

    #[test]
    fn test_empty_message_content_returns_safe_error() {
        let resp = mock_chat_response_no_content();
        let transport = MockTransport::new(Ok(resp));
        let provider = OpenRouterProvider::new(mock_config(), transport);
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let err = provider.compile(&req).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_api_key_not_exposed_in_errors() {
        let cfg = ResolvedLlmProviderConfig {
            api_key: None,
            ..mock_config()
        };
        let transport = MockTransport::new(Ok(mock_chat_response("ok")));
        let provider = OpenRouterProvider::new(cfg, transport);
        let req = LlmCompileRequest {
            original_prompt: "test".into(),
            category: "test".into(),
            instruction: "x".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let err = provider.compile(&req).unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("fake-test-key"));
    }

    #[test]
    fn test_request_never_contains_raw_secret() {
        let req = LlmCompileRequest {
            original_prompt: "use [REDACTED_SECRET] for auth".into(),
            category: "security_permissions_auth".into(),
            instruction: "Original prompt: [REDACTED_SECRET]".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &mock_config());
        let body = serde_json::to_string(&or).unwrap();
        assert!(!body.contains("sk-abc123"));
        assert!(body.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn test_default_compile_behavior_unchanged() {
        let compiler =
            crate::from_rules_file(std::path::Path::new("research/transformation_rules.json"))
                .unwrap();
        let input = crate::compiler::CompileInput {
            prompt: "fix this repo".into(),
        };
        let output = crate::compiler::compile(&input, &compiler);
        assert_eq!(output.mode, "local_compile");
    }

    fn mock_chat_response(content: &str) -> OpenRouterChatResponse {
        OpenRouterChatResponse {
            model: "test-model".into(),
            choices: vec![OpenRouterChoice {
                message: OpenRouterResponseMessage {
                    content: Some(content.into()),
                },
            }],
        }
    }

    fn mock_chat_response_no_content() -> OpenRouterChatResponse {
        OpenRouterChatResponse {
            model: "test-model".into(),
            choices: vec![OpenRouterChoice {
                message: OpenRouterResponseMessage { content: None },
            }],
        }
    }

    #[test]
    fn test_request_includes_original_prompt_when_instruction_is_minimal() {
        let req = LlmCompileRequest {
            original_prompt: "fix parser bug".into(),
            category: "repair_debug".into(),
            instruction: "rewrite".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let or = build_openrouter_request(&req, &mock_config());
        let user_msg = &or.messages[1].content;
        assert!(
            user_msg.contains("fix parser bug"),
            "User message must contain the original prompt even when instruction is minimal: {}",
            user_msg
        );
    }

    #[test]
    fn test_request_sent_through_transport_contains_prompt() {
        let req = LlmCompileRequest {
            original_prompt: "refactor the parser".into(),
            category: "refactor_cleanup".into(),
            instruction: "rewrite".into(),
            must_preserve: vec![],
            must_not_invent: vec![],
        };
        let transport = MockTransport::new(Ok(mock_chat_response("ok")));
        let provider = OpenRouterProvider::new(mock_config(), transport);
        let _ = provider.compile(&req);
        let captured = provider.transport.captured_request.borrow();
        let body = serde_json::to_string(captured.as_ref().unwrap()).unwrap();
        assert!(body.contains("refactor the parser"));
    }
}

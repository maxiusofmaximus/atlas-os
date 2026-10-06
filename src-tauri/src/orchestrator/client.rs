// Atlas OS — Provider HTTP client (RFC 20 Fase 25 v25.0; plan research/54).
//
// The missing leaf of the orchestrator: nothing performed an actual model HTTP
// call. This module provides the `ProviderClient` trait + an OpenAI-compatible
// `HttpProviderClient` (reqwest), plus pure, offline-testable codecs
// (`build_chat_body` / `parse_chat_response` / `chat_endpoint`). The network
// call is a thin shell over the pure codecs so tests never touch the network.

use serde::{Deserialize, Serialize};

use crate::orchestrator::provider::Deployment;

/// One chat message (role + content). OpenAI-compatible shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

/// A chat completion request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// Token accounting from the provider response.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

/// A chat completion response.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub content: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("http transport error: {0}")]
    Http(String),
    #[error("provider returned status {status}: {body}")]
    Status { status: u16, body: String },
    #[error("response decode error: {0}")]
    Decode(String),
    #[error("missing API key env var `{0}`")]
    MissingApiKey(String),
}

pub type ClientResult<T> = Result<T, ClientError>;

/// Pure: build the OpenAI-compatible `/chat/completions` request body.
pub fn build_chat_body(req: &ChatRequest) -> serde_json::Value {
    let messages: Vec<serde_json::Value> = req
        .messages
        .iter()
        .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
        .collect();
    let mut body = serde_json::json!({ "model": req.model, "messages": messages });
    if let Some(t) = req.temperature {
        body["temperature"] = serde_json::json!(t);
    }
    if let Some(m) = req.max_tokens {
        body["max_tokens"] = serde_json::json!(m);
    }
    body
}

/// Pure: parse an OpenAI-compatible response into a `ChatResponse`.
pub fn parse_chat_response(value: &serde_json::Value) -> ClientResult<ChatResponse> {
    let model = value
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or_default()
        .to_string();
    let content = value
        .pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
        .ok_or_else(|| ClientError::Decode("missing choices[0].message.content".into()))?
        .to_string();
    let usage = value.get("usage").and_then(|u| {
        let prompt = u.get("prompt_tokens").and_then(|x| x.as_u64());
        let completion = u.get("completion_tokens").and_then(|x| x.as_u64());
        match (prompt, completion) {
            (Some(p), Some(c)) => Some(Usage {
                prompt_tokens: p,
                completion_tokens: c,
            }),
            _ => None,
        }
    });
    Ok(ChatResponse {
        model,
        content,
        usage,
    })
}

/// Pure: `{api_base}/chat/completions`, tolerating a trailing slash.
pub fn chat_endpoint(api_base: &str) -> String {
    format!("{}/chat/completions", api_base.trim_end_matches('/'))
}

/// A model-call client. Uses AFIT/RPITIT (not `dyn`): the host is generic over
/// the client, so prod uses `HttpProviderClient` and tests use a scripted mock.
pub trait ProviderClient: Send + Sync {
    fn chat(
        &self,
        deployment: &Deployment,
        request: &ChatRequest,
    ) -> impl std::future::Future<Output = ClientResult<ChatResponse>> + Send;
}

/// OpenAI-compatible HTTP client.
pub struct HttpProviderClient {
    http: reqwest::Client,
}

impl HttpProviderClient {
    pub fn new() -> Self {
        // Identify as Atlas OS, not a generic HTTP library (OpenCode Go asks
        // clients to send a self-identifying User-Agent).
        let http = reqwest::Client::builder()
            .user_agent(concat!("atlas-os/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { http }
    }
}

impl Default for HttpProviderClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve a deployment's API key. Env var first (unchanged behaviour), then
/// the OS keychain (account = the slot name) — RFC 25 §3.10 / RFC 18. This is
/// what lets `atlas secrets set <slot>` drive a deployment without exporting
/// the key into the process environment.
fn resolve_api_key(slot: &str) -> ClientResult<String> {
    if let Ok(value) = std::env::var(slot) {
        // Trim: a Windows CRLF `.env` leaves a trailing `\r` on the value,
        // which corrupts the bearer token (401). Keys never contain spaces.
        let value = value.trim();
        if !value.is_empty() {
            return Ok(value.to_string());
        }
    }
    match crate::secrets::get(slot) {
        Ok(Some(value)) if !value.trim().is_empty() => Ok(value.trim().to_string()),
        Ok(_) => Err(ClientError::MissingApiKey(slot.to_string())),
        Err(e) => Err(ClientError::Http(format!(
            "keychain lookup for `{slot}` failed: {e}"
        ))),
    }
}

impl ProviderClient for HttpProviderClient {
    async fn chat(
        &self,
        deployment: &Deployment,
        request: &ChatRequest,
    ) -> ClientResult<ChatResponse> {
        let mut builder = self
            .http
            .post(chat_endpoint(&deployment.api_base))
            .json(&build_chat_body(request));
        if let Some(env_var) = &deployment.api_key_env {
            let key = resolve_api_key(env_var)?;
            if !key.trim().is_empty() {
                builder = builder.bearer_auth(key);
            }
        }
        // Provider-mandated headers (e.g. OpenCode Go's `x-opencode-session`).
        for (name, value) in &deployment.extra_headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        let response = builder
            .send()
            .await
            .map_err(|e| ClientError::Http(e.to_string()))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| ClientError::Http(e.to_string()))?;
        if !status.is_success() {
            return Err(ClientError::Status {
                status: status.as_u16(),
                body: text.chars().take(500).collect(),
            });
        }
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| ClientError::Decode(e.to_string()))?;
        parse_chat_response(&json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> ChatRequest {
        ChatRequest {
            model: "gpt-test".into(),
            messages: vec![ChatMessage::system("be terse"), ChatMessage::user("hi")],
            temperature: Some(0.3),
            max_tokens: Some(128),
        }
    }

    #[test]
    fn build_chat_body_carries_model_messages_and_options() {
        let body = build_chat_body(&req());
        assert_eq!(body["model"], "gpt-test");
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
        assert_eq!(body["messages"][0]["role"], "system");
        // `temperature` is f32; compare with tolerance (f32→f64 rounding).
        let temp = body["temperature"].as_f64().unwrap();
        assert!((temp - 0.3).abs() < 1e-6, "temperature was {temp}");
        assert_eq!(body["max_tokens"], 128);
    }

    #[test]
    fn build_chat_body_omits_absent_options() {
        let body = build_chat_body(&ChatRequest {
            model: "m".into(),
            messages: vec![ChatMessage::user("x")],
            temperature: None,
            max_tokens: None,
        });
        assert!(body.get("temperature").is_none());
        assert!(body.get("max_tokens").is_none());
    }

    #[test]
    fn parse_chat_response_reads_content_and_usage() {
        let canned = serde_json::json!({
            "model": "gpt-test",
            "choices": [{ "message": { "role": "assistant", "content": "hello" } }],
            "usage": { "prompt_tokens": 11, "completion_tokens": 4 }
        });
        let parsed = parse_chat_response(&canned).unwrap();
        assert_eq!(parsed.model, "gpt-test");
        assert_eq!(parsed.content, "hello");
        assert_eq!(
            parsed.usage,
            Some(Usage {
                prompt_tokens: 11,
                completion_tokens: 4
            })
        );
    }

    #[test]
    fn parse_chat_response_without_usage_is_ok() {
        let canned = serde_json::json!({
            "choices": [{ "message": { "content": "x" } }]
        });
        let parsed = parse_chat_response(&canned).unwrap();
        assert_eq!(parsed.content, "x");
        assert!(parsed.usage.is_none());
    }

    #[test]
    fn parse_chat_response_missing_content_errors() {
        let canned = serde_json::json!({ "choices": [] });
        assert!(matches!(
            parse_chat_response(&canned),
            Err(ClientError::Decode(_))
        ));
    }

    #[test]
    fn chat_endpoint_trims_trailing_slash() {
        assert_eq!(
            chat_endpoint("https://api.example/v1/"),
            "https://api.example/v1/chat/completions"
        );
        assert_eq!(
            chat_endpoint("http://localhost:8080/v1"),
            "http://localhost:8080/v1/chat/completions"
        );
    }

    #[tokio::test]
    async fn http_client_reports_missing_api_key_before_any_network() {
        let mut deployment = Deployment::new("m", "http://127.0.0.1:1");
        deployment.api_key_env = Some("ATLAS_TEST_ENV_THAT_DOES_NOT_EXIST".into());
        let err = HttpProviderClient::new()
            .chat(&deployment, &req())
            .await
            .unwrap_err();
        assert!(matches!(err, ClientError::MissingApiKey(_)));
    }

    static KEY_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn resolve_api_key_trims_trailing_whitespace_from_env() {
        let _guard = KEY_ENV_LOCK.lock().unwrap();
        // A Windows CRLF .env leaves `\r` on the value; it must not reach the
        // bearer token.
        std::env::set_var("ATLAS_TEST_TRIM_KEY", "sk-abc\r\n");
        assert_eq!(resolve_api_key("ATLAS_TEST_TRIM_KEY").unwrap(), "sk-abc");
        std::env::remove_var("ATLAS_TEST_TRIM_KEY");
    }

    struct ScriptedClient {
        reply: ChatResponse,
    }

    impl ProviderClient for ScriptedClient {
        async fn chat(
            &self,
            _deployment: &Deployment,
            _request: &ChatRequest,
        ) -> ClientResult<ChatResponse> {
            Ok(self.reply.clone())
        }
    }

    #[tokio::test]
    async fn scripted_client_is_usable_through_the_trait() {
        let client = ScriptedClient {
            reply: ChatResponse {
                model: "m".into(),
                content: "ok".into(),
                usage: None,
            },
        };
        let deployment = Deployment::new("m", "http://x");
        let out = client.chat(&deployment, &req()).await.unwrap();
        assert_eq!(out.content, "ok");
    }
}

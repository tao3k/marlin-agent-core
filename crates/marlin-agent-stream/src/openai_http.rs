// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Thin, zero-orchestration HTTP/HTTPS client for OpenAI-compatible
//! `/v1/chat/completions` endpoints backed by `reqwest` + `rustls`.

use std::time::Duration;

use marlin_agent_protocol::{ModelGatewayMessageRole, ModelGatewayRequest};

const MAX_OPENAI_HTTP_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// Configuration for an OpenAI-compatible HTTP/HTTPS chat completions endpoint.
#[derive(Clone, Debug)]
pub struct OpenAiHttpConfig {
    pub base_url: String,
    pub model_override: Option<String>,
    pub api_key: Option<String>,
    pub timeout: Duration,
}

/// Thin HTTP/HTTPS client executing single `LlmComplete` atomic effects against
/// any OpenAI-compatible `POST /v1/chat/completions` server (`http://` or `https://`).
#[derive(Clone, Debug)]
pub struct OpenAiCompatibleHttpClient {
    client: reqwest::Client,
    completions_url: String,
    model_override: Option<String>,
    api_key: Option<String>,
}

impl OpenAiCompatibleHttpClient {
    /// Constructs a validated client from `config`, supporting both `http://` and `https://`.
    pub fn new(config: OpenAiHttpConfig) -> Result<Self, String> {
        let completions_url = normalize_completions_url(&config.base_url)?;
        let timeout = if config.timeout.is_zero() {
            Duration::from_secs(15)
        } else {
            config.timeout
        };
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|err| format!("build reqwest tls client: {err}"))?;
        Ok(Self {
            client,
            completions_url,
            model_override: config.model_override,
            api_key: config.api_key,
        })
    }

    /// Returns the resolved chat completions endpoint URL.
    pub fn completions_url(&self) -> &str {
        &self.completions_url
    }

    /// Executes one non-streaming OpenAI chat completion request and returns
    /// `choices[0].message.content`.
    pub async fn complete_text(&self, request: &ModelGatewayRequest) -> Result<String, String> {
        let model = self
            .model_override
            .as_deref()
            .unwrap_or_else(|| request.endpoint().model.as_str());
        let messages: Vec<serde_json::Value> = request
            .messages()
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    ModelGatewayMessageRole::System => "system",
                    ModelGatewayMessageRole::Developer => "developer",
                    ModelGatewayMessageRole::User => "user",
                    ModelGatewayMessageRole::Assistant => "assistant",
                    ModelGatewayMessageRole::Tool => "tool",
                };
                serde_json::json!({
                    "role": role,
                    "content": msg.content,
                })
            })
            .collect();
        let payload = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        let mut req_builder = self.client.post(&self.completions_url).json(&payload);
        if let Some(key) = &self.api_key {
            req_builder = req_builder.bearer_auth(key);
        }
        let resp = req_builder
            .send()
            .await
            .map_err(|err| format!("send llm http request to {}: {err}", self.completions_url))?;
        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|err| format!("read llm http response body: {err}"))?;
        if bytes.len() > MAX_OPENAI_HTTP_RESPONSE_BYTES {
            return Err("llm http response exceeded 4 MiB safety limit".to_owned());
        }
        if !status.is_success() {
            let preview = String::from_utf8_lossy(&bytes);
            return Err(format!("llm http status {}: {preview}", status.as_u16()));
        }
        extract_completion_content(&bytes)
    }
}

fn normalize_completions_url(raw_url: &str) -> Result<String, String> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Err("MARLIN_SCHEME_LLM_BASE_URL must not be empty".to_owned());
    }
    let after_scheme = if let Some(rest) = trimmed.strip_prefix("https://") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        rest
    } else {
        return Err(format!(
            "MARLIN_SCHEME_LLM_BASE_URL must use http:// or https:// scheme, got {trimmed}"
        ));
    };
    let authority = after_scheme.split('/').next().unwrap_or("");
    if authority.is_empty() {
        return Err("MARLIN_SCHEME_LLM_BASE_URL missing host authority".to_owned());
    }
    let clean = trimmed.trim_end_matches('/');
    if clean.ends_with("/chat/completions") {
        Ok(clean.to_owned())
    } else if clean.ends_with("/v1") {
        Ok(format!("{clean}/chat/completions"))
    } else {
        Ok(format!("{clean}/v1/chat/completions"))
    }
}

fn extract_completion_content(body: &[u8]) -> Result<String, String> {
    let json: serde_json::Value =
        serde_json::from_slice(body).map_err(|err| format!("parse llm json response: {err}"))?;
    json.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c0| c0.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| "llm json response missing choices[0].message.content".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use marlin_agent_protocol::{ModelEndpoint, user_gateway_message};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    #[test]
    fn normalize_url_supports_https_and_http_and_rejects_other_schemes() {
        assert_eq!(
            normalize_completions_url("https://api.openai.com").unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            normalize_completions_url("https://api.openai.com/v1/").unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            normalize_completions_url("http://127.0.0.1:8080/v1/chat/completions").unwrap(),
            "http://127.0.0.1:8080/v1/chat/completions"
        );
        assert!(normalize_completions_url("ftp://example.com").is_err());
        assert!(normalize_completions_url("https://").is_err());
    }

    #[tokio::test]
    async fn client_completes_real_loopback_http_openai_request() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let srv = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap();
            let req_text = String::from_utf8_lossy(&buf[..n]);
            assert!(req_text.contains("POST /v1/chat/completions"));
            assert!(
                req_text
                    .to_ascii_lowercase()
                    .contains("authorization: bearer sk-s29")
            );
            let body = r#"{"id":"cmpl-1","choices":[{"index":0,"message":{"role":"assistant","content":"tls-client-ok"}}]}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            sock.write_all(resp.as_bytes()).await.unwrap();
        });

        let client = OpenAiCompatibleHttpClient::new(OpenAiHttpConfig {
            base_url: format!("http://{addr}"),
            model_override: Some("gpt-s29".to_owned()),
            api_key: Some("sk-s29".to_owned()),
            timeout: Duration::from_secs(5),
        })
        .unwrap();
        let req = ModelGatewayRequest::new(
            ModelEndpoint::new("openai", "fallback-model"),
            vec![user_gateway_message("ping")],
        );
        let reply = client.complete_text(&req).await.unwrap();
        assert_eq!(reply, "tls-client-ok");
        srv.await.unwrap();
    }
}

// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Thin adapter routing OpenAI-compatible `/v1/chat/completions` requests
//! through the canonical `weaver-llm` HTTP transport (`Policy::none()` redirect
//! confinement, HTTPS-for-remote / loopback-only HTTP enforcement, userinfo
//! rejection, `no_proxy()`, and 4 MiB bounded response streaming).

use std::{fmt, time::Duration};

use marlin_agent_protocol::{ModelGatewayMessageRole, ModelGatewayRequest};
use weaver_llm::{
    Client as WeaverClient, Endpoint as WeaverEndpoint, Error as WeaverError,
    Message as WeaverMessage, Options as WeaverOptions, Protocol as WeaverProtocol,
    Request as WeaverRequest, Role as WeaverRole,
};

const MAX_OPENAI_HTTP_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const LOOPBACK_FALLBACK_CREDENTIAL: &str = "marlin-loopback-no-key";

/// Configuration for an OpenAI-compatible HTTP/HTTPS chat completions endpoint.
#[derive(Clone, Debug)]
pub struct OpenAiHttpConfig {
    pub base_url: String,
    pub model_override: Option<String>,
    pub api_key: Option<String>,
    pub timeout: Duration,
}

/// OpenAI-compatible chat completions adapter backed by `weaver_llm::Client`.
#[derive(Clone)]
pub struct OpenAiCompatibleHttpClient {
    client: WeaverClient,
    endpoint: WeaverEndpoint,
    completions_url: String,
    model_override: Option<String>,
    api_key: Option<String>,
}

impl fmt::Debug for OpenAiCompatibleHttpClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiCompatibleHttpClient")
            .field("completions_url", &self.completions_url)
            .field("model_override", &self.model_override)
            .field("has_api_key", &self.api_key.is_some())
            .finish()
    }
}

impl OpenAiCompatibleHttpClient {
    /// Constructs a validated `weaver-llm`-backed client from `config`.
    ///
    /// Enforces `weaver-llm` endpoint invariants: `https://` required for remote
    /// hosts, `http://` allowed only on explicit loopback (`localhost`, `127.0.0.1`,
    /// `[::1]`), zero URL userinfo/fragments/control characters, no automatic
    /// redirects (`Policy::none()`), and a 4 MiB response body ceiling.
    pub fn new(config: OpenAiHttpConfig) -> Result<Self, String> {
        let completions_url = normalize_completions_url(&config.base_url)?;
        let endpoint = WeaverEndpoint::new(WeaverProtocol::OpenAi, &completions_url).map_err(
            |err| {
                format!(
                    "invalid weaver-llm endpoint {completions_url} (remote hosts require https:// without userinfo): {err:?}"
                )
            },
        )?;
        let timeout = if config.timeout.is_zero() {
            Duration::from_secs(15)
        } else {
            config.timeout
        };
        let client = WeaverClient::new(timeout, MAX_OPENAI_HTTP_RESPONSE_BYTES)
            .map_err(|err| format!("build weaver-llm tls client: {err:?}"))?;
        Ok(Self {
            client,
            endpoint,
            completions_url,
            model_override: config.model_override,
            api_key: config.api_key,
        })
    }

    /// Returns the resolved chat completions endpoint URL.
    pub fn completions_url(&self) -> &str {
        &self.completions_url
    }

    /// Executes one non-streaming OpenAI chat completion request via `weaver-llm`
    /// and returns `choices[0].message.content`.
    pub async fn complete_text(&self, request: &ModelGatewayRequest) -> Result<String, String> {
        let model = self
            .model_override
            .as_deref()
            .unwrap_or_else(|| request.endpoint().model.as_str());
        let messages: Vec<WeaverMessage> = request
            .messages()
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    ModelGatewayMessageRole::System => WeaverRole::System,
                    ModelGatewayMessageRole::Developer => WeaverRole::Developer,
                    ModelGatewayMessageRole::User => WeaverRole::User,
                    ModelGatewayMessageRole::Assistant => WeaverRole::Assistant,
                    ModelGatewayMessageRole::Tool => WeaverRole::Tool,
                };
                WeaverMessage {
                    role,
                    content: msg.content.clone(),
                    images: Vec::new(),
                    name: msg.name.clone(),
                    tool_call_id: msg.tool_call_id.as_ref().map(|id| id.as_str().to_owned()),
                    tool_calls: Vec::new(),
                    responses_output: None,
                    anthropic_output: None,
                }
            })
            .collect();
        let options = WeaverOptions::default();
        let weaver_req = WeaverRequest {
            model,
            messages: &messages,
            options: &options,
        };
        let credential = self
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .unwrap_or(LOOPBACK_FALLBACK_CREDENTIAL);

        let completion = self
            .client
            .complete(&self.endpoint, credential, &weaver_req)
            .await
            .map_err(|err| match err {
                WeaverError::Limit => "llm http response exceeded 4 MiB safety limit".to_owned(),
                other => format!(
                    "weaver-llm http request to {} failed: {other:?}",
                    self.completions_url
                ),
            })?;

        completion
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| "llm json response missing choices[0].message.content".to_owned())
    }
}

fn normalize_completions_url(raw_url: &str) -> Result<String, String> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Err("MARLIN_SCHEME_LLM_BASE_URL must not be empty".to_owned());
    }
    if trimmed
        .chars()
        .any(|ch| ch.is_ascii_control() || ch.is_whitespace())
    {
        return Err(
            "MARLIN_SCHEME_LLM_BASE_URL must not contain whitespace or control characters"
                .to_owned(),
        );
    }
    WeaverEndpoint::new(WeaverProtocol::OpenAi, trimmed).map_err(|err| {
        format!("MARLIN_SCHEME_LLM_BASE_URL rejected by weaver-llm endpoint policy: {err:?}")
    })?;
    let clean = trimmed.trim_end_matches('/');
    let resolved = if clean.ends_with("/chat/completions") {
        clean.to_owned()
    } else if clean.ends_with("/v1") {
        format!("{clean}/chat/completions")
    } else {
        format!("{clean}/v1/chat/completions")
    };
    WeaverEndpoint::new(WeaverProtocol::OpenAi, &resolved).map_err(|err| {
        format!("MARLIN_SCHEME_LLM_BASE_URL rejected by weaver-llm endpoint policy: {err:?}")
    })?;
    Ok(resolved)
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
            let body = r#"{"id":"cmpl-1","model":"gpt-s29","choices":[{"index":0,"message":{"role":"assistant","content":"tls-client-ok"},"finish_reason":"stop"}]}"#;
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

    #[tokio::test]
    async fn sm23_adv_l4_blocks_307_redirect_ssrf_and_prompt_exfiltration() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        let victim_hit = Arc::new(AtomicBool::new(false));
        let victim_hit_clone = Arc::clone(&victim_hit);
        let victim_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let victim_addr = victim_listener.local_addr().unwrap();
        let victim_task = tokio::spawn(async move {
            if let Ok(Ok((mut sock, _))) =
                tokio::time::timeout(Duration::from_millis(800), victim_listener.accept()).await
            {
                victim_hit_clone.store(true, Ordering::SeqCst);
                let mut buf = vec![0u8; 4096];
                let _ = sock.read(&mut buf).await;
                let body = r#"{"id":"cmpl-evil","model":"gpt-adv-l4","choices":[{"index":0,"message":{"role":"assistant","content":"INTERNAL_ADMIN_SECRET_42"},"finish_reason":"stop"}]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
            }
        });

        let redirect_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let redirect_addr = redirect_listener.local_addr().unwrap();
        let redirect_task = tokio::spawn(async move {
            let (mut sock, _) = redirect_listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = sock.read(&mut buf).await;
            let resp = format!(
                "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{victim_addr}/v1/chat/completions\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        });

        let client = OpenAiCompatibleHttpClient::new(OpenAiHttpConfig {
            base_url: format!("http://{redirect_addr}"),
            model_override: Some("gpt-adv-l4".to_owned()),
            api_key: Some("sk-adv-l4-secret".to_owned()),
            timeout: Duration::from_secs(3),
        })
        .unwrap();
        let req = ModelGatewayRequest::new(
            ModelEndpoint::new("openai", "gpt-adv-l4"),
            vec![user_gateway_message("CONFIDENTIAL_PROMPT")],
        );
        let result = client.complete_text(&req).await;
        let _ = redirect_task.await;
        let _ = victim_task.await;

        assert!(
            !victim_hit.load(Ordering::SeqCst),
            "ADV-L4 SSRF ESCAPE PROVEN: reqwest followed 307 redirect to internal victim service and exfiltrated prompt!"
        );
        assert!(
            result.is_err(),
            "ADV-L4 REDIRECT PROVEN: complete_text succeeded across 307 redirect with {:?}",
            result
        );
    }

    #[test]
    fn sm23_adv_l4_blocks_cleartext_http_bearer_to_remote_host() {
        let res = OpenAiCompatibleHttpClient::new(OpenAiHttpConfig {
            base_url: "http://api.remote-attacker.example.com".to_owned(),
            model_override: None,
            api_key: Some("sk-prod-bearer-secret".to_owned()),
            timeout: Duration::from_secs(5),
        });
        assert!(
            res.is_err(),
            "ADV-L4 CREDENTIAL LEAK PROVEN: accepted cleartext http:// for non-loopback host with Bearer api_key"
        );
    }

    #[test]
    fn sm23_adv_l4_blocks_url_userinfo_and_crlf_authority_smuggling() {
        for hostile in [
            "https://api.openai.com@evil.example/v1",
            "https://user:pass@api.openai.com/v1",
            "https://api.openai.com/v1\r\nX-Injected: 1",
            "https://api.openai.com /v1",
        ] {
            let res = normalize_completions_url(hostile);
            assert!(
                res.is_err(),
                "ADV-L4 URL SMUGGLING PROVEN: normalize_completions_url accepted hostile URL {hostile:?} -> {res:?}"
            );
        }
    }
}

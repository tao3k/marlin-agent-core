// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Production read-only `ToolRuntime` handler, loopback gRPC client, and Magit
//! workbench review projection for `aitia serve-marlin-grpc` (RFC 0024 / RFC 0026).
//!
//! Connects Marlin Agent Core over loopback TCP using 5-byte length-prefixed
//! `application/grpc+json` frames + constant-time Bearer authentication, fetches
//! the single-source-of-truth `ModelToolDefinition` (`aitia_review_step`) from
//! Scheme via `GetToolSchema`, enforces fail-closed `actionAuthorized == false`
//! on every response, and projects typed read-only review snapshots for Marlin's
//! human disposition UI (`ui-magit` / `marlin-agent-app-server`).

use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use marlin_agent_protocol::{ModelToolCall, ModelToolDefinition, ModelToolName};
use marlin_agent_runtime::{ModelToolOutcome, RuntimeContext, RuntimeFuture, ToolRuntime};
use serde_json::{Value, json};

const SERVICE_PREFIX: &str = "/aitia.marlin.v1.AitiaSidecarGateway/";
const MAX_GRPC_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
pub const AITIA_REVIEW_TOOL_NAME: &str = "aitia_review_step";

/// Raw gRPC-over-HTTP/1.1 response returned by `aitia serve-marlin-grpc`.
#[derive(Clone, Debug)]
pub struct AitiaGrpcRawResponse {
    pub grpc_status: u8,
    pub grpc_message: String,
    pub body: Value,
}

/// Thin loopback gRPC client for `aitia serve-marlin-grpc` (`application/grpc+json`).
///
/// Enforces `actionAuthorized == false` on every successful RPC response so the
/// sidecar oracle can never grant execution authority inside Marlin.
#[derive(Clone, Debug)]
pub struct AitiaSidecarGrpcClient {
    addr: SocketAddr,
    bearer_token: Option<String>,
    timeout: Duration,
}

impl AitiaSidecarGrpcClient {
    #[must_use]
    pub fn new(addr: SocketAddr, bearer_token: Option<String>) -> Self {
        Self {
            addr,
            bearer_token,
            timeout: Duration::from_secs(60),
        }
    }

    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Invoke any RPC method on `aitia.marlin.v1.AitiaSidecarGateway` and return
    /// the raw gRPC status + decoded JSON envelope (useful for negative controls).
    pub fn call_rpc_raw(
        &self,
        method: &str,
        payload: &Value,
        override_bearer: Option<&str>,
    ) -> Result<AitiaGrpcRawResponse, String> {
        let payload_bytes = serde_json::to_vec(payload)
            .map_err(|e| format!("failed to serialize gRPC JSON payload: {e}"))?;
        let frame = encode_grpc_frame(&payload_bytes)?;

        let mut stream = TcpStream::connect_timeout(&self.addr, self.timeout)
            .map_err(|e| format!("failed to connect to aitia gRPC at {}: {e}", self.addr))?;
        let _ = stream.set_read_timeout(Some(self.timeout));
        let _ = stream.set_write_timeout(Some(self.timeout));

        let mut req_head = format!(
            "POST {SERVICE_PREFIX}{method} HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Type: application/grpc+json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n",
            self.addr,
            frame.len()
        );
        let token = override_bearer.or(self.bearer_token.as_deref());
        if let Some(tok) = token {
            req_head.push_str(&format!("Authorization: Bearer {tok}\r\n"));
        }
        req_head.push_str("\r\n");

        stream
            .write_all(req_head.as_bytes())
            .and_then(|()| stream.write_all(&frame))
            .and_then(|()| stream.flush())
            .map_err(|e| format!("failed to send gRPC request `{method}`: {e}"))?;

        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        reader
            .read_line(&mut status_line)
            .map_err(|e| format!("failed to read gRPC HTTP status line: {e}"))?;
        if !status_line.starts_with("HTTP/1.1 200") {
            return Err(format!(
                "unexpected HTTP status from aitia gRPC `{method}`: {}",
                status_line.trim()
            ));
        }

        let mut grpc_status: u8 = 2;
        let mut grpc_message = String::new();
        let mut content_len: usize = 0;
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader
                .read_line(&mut line)
                .map_err(|e| format!("failed to read gRPC response header: {e}"))?;
            if n == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim().to_ascii_lowercase();
                let val = v.trim();
                match key.as_str() {
                    "grpc-status" => {
                        grpc_status = val.parse::<u8>().unwrap_or(2);
                    }
                    "grpc-message" => {
                        grpc_message = val.to_owned();
                    }
                    "content-length" => {
                        content_len = val.parse::<usize>().unwrap_or(usize::MAX);
                    }
                    _ => {}
                }
            }
        }
        if content_len > MAX_GRPC_RESPONSE_BYTES {
            return Err(format!(
                "aitia gRPC response for `{method}` exceeds maximum size ({content_len} bytes)"
            ));
        }
        let mut raw_body = vec![0u8; content_len];
        reader
            .read_exact(&mut raw_body)
            .map_err(|e| format!("failed to read gRPC frame body for `{method}`: {e}"))?;
        let json_bytes = decode_grpc_frame(&raw_body)?;
        let body: Value = serde_json::from_slice(json_bytes)
            .map_err(|e| format!("invalid JSON inside gRPC frame for `{method}`: {e}"))?;

        Ok(AitiaGrpcRawResponse {
            grpc_status,
            grpc_message,
            body,
        })
    }

    /// Invoke a gRPC method and fail closed unless `grpc-status == 0` AND
    /// `actionAuthorized == false`.
    pub fn call_rpc(&self, method: &str, payload: &Value) -> Result<Value, String> {
        let resp = self.call_rpc_raw(method, payload, None)?;
        if resp.grpc_status != 0 {
            return Err(format!(
                "aitia gRPC `{method}` failed (status={}): {}",
                resp.grpc_status, resp.grpc_message
            ));
        }
        if resp.body.get("actionAuthorized").and_then(Value::as_bool) != Some(false) {
            return Err(format!(
                "fail-closed invariant breach: aitia gRPC `{method}` did not certify actionAuthorized=false"
            ));
        }
        Ok(resp.body)
    }

    /// Check sidecar health (`Health`).
    pub fn health(&self) -> Result<Value, String> {
        self.call_rpc("Health", &json!({}))
    }

    /// Fetch the single-source-of-truth `ModelToolDefinition` (`GetToolSchema`)
    /// projected directly from Scheme (`gitops-review-tool-schema`).
    pub fn get_tool_definition(&self) -> Result<ModelToolDefinition, String> {
        let raw = self.call_rpc("GetToolSchema", &json!({}))?;
        if raw.get("readOnly").and_then(Value::as_bool) != Some(true) {
            return Err("aitia GetToolSchema must certify readOnly=true".to_owned());
        }
        let name_str = raw
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "aitia GetToolSchema missing `name`".to_owned())?;
        let name = ModelToolName::new(name_str)
            .map_err(|e| format!("invalid aitia tool name `{name_str}`: {e}"))?;
        let description = raw
            .get("description")
            .and_then(Value::as_str)
            .ok_or_else(|| "aitia GetToolSchema missing `description`".to_owned())?
            .to_owned();
        let parameters = raw
            .get("parameters")
            .cloned()
            .ok_or_else(|| "aitia GetToolSchema missing `parameters`".to_owned())?;
        Ok(ModelToolDefinition {
            name,
            description,
            parameters,
        })
    }

    /// Execute one progressive FunFlow/ASCENT review step (`StepReview`).
    pub fn step_review(&self, request: &Value) -> Result<Value, String> {
        self.call_rpc("StepReview", request)
    }

    /// Verify DE dual-mode formal evidence (`VerifyFormalEvidence`).
    pub fn verify_formal_evidence(
        &self,
        formal_evidence_json: &str,
        expected_context_digest: &str,
    ) -> Result<Value, String> {
        self.call_rpc(
            "VerifyFormalEvidence",
            &json!({
                "formalEvidenceJson": formal_evidence_json,
                "expectedContextDigest": expected_context_digest,
            }),
        )
    }
}

/// Process-isolated `aitia serve-marlin-grpc` daemon handle that cleanly stops
/// the sidecar process on drop.
pub struct ManagedAitiaSidecarServer {
    child: Child,
    client: AitiaSidecarGrpcClient,
}

impl ManagedAitiaSidecarServer {
    /// Locate the installed `aitia` binary (`AITIA_BIN` or `~/.local/bin/aitia`).
    #[must_use]
    pub fn resolve_aitia_binary() -> Option<PathBuf> {
        if let Some(explicit) = env::var_os("AITIA_BIN").filter(|v| !v.is_empty()) {
            let p = PathBuf::from(explicit);
            if p.is_file() {
                return Some(p);
            }
        }
        if let Some(home) = env::var_os("HOME") {
            let local = PathBuf::from(home).join(".local/bin/aitia");
            if local.is_file() {
                return Some(local);
            }
        }
        None
    }

    /// Spawn `aitia serve-marlin-grpc --bind 127.0.0.1:0 --bearer-token <token>`
    /// and wait for its ready JSON handshake.
    pub fn spawn(aitia_bin: &Path, bearer_token: &str) -> Result<Self, String> {
        if bearer_token.trim().is_empty() {
            return Err("bearer_token must be non-blank".to_owned());
        }
        let mut child = Command::new(aitia_bin)
            .args([
                "serve-marlin-grpc",
                "--bind",
                "127.0.0.1:0",
                "--bearer-token",
                bearer_token,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn `{}`: {e}", aitia_bin.display()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "aitia sidecar stdout pipe missing".to_owned())?;
        let mut reader = BufReader::new(stdout);
        let mut ready_line = String::new();
        reader
            .read_line(&mut ready_line)
            .map_err(|e| format!("failed to read aitia ready handshake: {e}"))?;
        if ready_line.trim().is_empty() {
            let _ = child.kill();
            let out = child.wait_with_output().ok();
            let stderr = out
                .as_ref()
                .map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
                .unwrap_or_default();
            return Err(format!(
                "aitia serve-marlin-grpc exited without ready banner: {}",
                stderr.trim()
            ));
        }
        let ready: Value = serde_json::from_str(ready_line.trim())
            .map_err(|e| format!("invalid aitia ready JSON `{}`: {e}", ready_line.trim()))?;
        let addr_str = ready
            .get("listenAddr")
            .and_then(Value::as_str)
            .ok_or_else(|| "aitia ready JSON missing `listenAddr`".to_owned())?;
        let addr: SocketAddr = addr_str
            .parse()
            .map_err(|e| format!("invalid aitia listenAddr `{addr_str}`: {e}"))?;
        if !addr.ip().is_loopback() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "refused non-loopback aitia listenAddr `{addr}` in tooling client"
            ));
        }
        Ok(Self {
            child,
            client: AitiaSidecarGrpcClient::new(addr, Some(bearer_token.to_owned())),
        })
    }

    #[must_use]
    pub fn client(&self) -> &AitiaSidecarGrpcClient {
        &self.client
    }
}

impl Drop for ManagedAitiaSidecarServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Production read-only `ToolRuntime` handler binding Marlin Agent Core's
/// `aitia_review_step` tool calls to a process-isolated `aitia serve-marlin-grpc`
/// sidecar over loopback TCP.
#[derive(Clone, Debug)]
pub struct AitiaReviewToolHandler {
    client: AitiaSidecarGrpcClient,
    default_repo: PathBuf,
    external_vault_dir: Option<PathBuf>,
}

impl AitiaReviewToolHandler {
    #[must_use]
    pub fn new(
        client: AitiaSidecarGrpcClient,
        default_repo: PathBuf,
        external_vault_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            client,
            default_repo,
            external_vault_dir,
        }
    }

    #[must_use]
    pub fn client(&self) -> &AitiaSidecarGrpcClient {
        &self.client
    }
}

impl ToolRuntime for AitiaReviewToolHandler {
    type Invocation = ModelToolCall;
    type Output = ModelToolOutcome;

    fn run_tool(
        &self,
        call: ModelToolCall,
        _context: RuntimeContext,
    ) -> RuntimeFuture<ModelToolOutcome> {
        let client = self.client.clone();
        let default_repo = self.default_repo.clone();
        let external_vault_dir = self.external_vault_dir.clone();
        Box::pin(async move {
            if call.name.as_str() != AITIA_REVIEW_TOOL_NAME {
                return ModelToolOutcome::Failed(format!(
                    "AitiaReviewToolHandler refused unexpected tool `{}`",
                    call.name.as_str()
                ));
            }
            let mut req_val = Value::Object(call.arguments);
            if let Some(obj) = req_val.as_object_mut() {
                let needs_default_repo = obj
                    .get("repo")
                    .and_then(Value::as_str)
                    .is_none_or(|s| s.trim().is_empty() || s.trim() == ".");
                if needs_default_repo {
                    obj.insert(
                        "repo".to_owned(),
                        Value::String(default_repo.to_string_lossy().into_owned()),
                    );
                }
                if let Some(vault) = external_vault_dir {
                    let missing_vault = obj
                        .get("vaultDir")
                        .and_then(Value::as_str)
                        .is_none_or(|s| s.trim().is_empty());
                    if missing_vault {
                        obj.insert(
                            "vaultDir".to_owned(),
                            Value::String(vault.to_string_lossy().into_owned()),
                        );
                    }
                }
            }
            let res = tokio::task::spawn_blocking(move || client.step_review(&req_val)).await;
            match res {
                Ok(Ok(resp)) => match serde_json::to_string(&resp) {
                    Ok(body) => ModelToolOutcome::Completed(body),
                    Err(e) => ModelToolOutcome::Failed(format!("serialize StepReview JSON: {e}")),
                },
                Ok(Err(err)) => ModelToolOutcome::Failed(err),
                Err(join_err) => {
                    ModelToolOutcome::Failed(format!("aitia StepReview worker failed: {join_err}"))
                }
            }
        })
    }
}

/// One typed diff finding projected for Marlin's Magit human disposition panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AitiaMagitFinding {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub rule: String,
    pub hallucination_class: String,
    pub detail: String,
}

/// Summary of one machine-executed Quint or Lean 4 ephemeral overlay projected
/// for Marlin's Magit workbench.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AitiaMagitOverlaySummary {
    pub model_id: String,
    pub formal_kind: String,
    pub machine_executed: bool,
    pub passed: bool,
    pub countermodel_verified: bool,
    pub stored_in_sidecar_vault: bool,
    pub resolver_digest: String,
    pub archived_overlay_path: Option<String>,
    pub archived_receipt_path: Option<String>,
}

/// Typed read-only human-disposition snapshot projected from `StepReview` JSON
/// for Marlin's `ui-magit` / `marlin-agent-app-server` review workbench.
///
/// Strictly enforces `action_authorized == false`: the UI displays verified
/// evidence and competing hypotheses, while explicit human confirmation gates
/// any subsequent sidecar commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AitiaMagitReviewSnapshot {
    pub target_repo: String,
    pub base_ref: String,
    pub context_digest: String,
    pub diff_decision: String,
    pub core_floor_preserved: bool,
    pub shared_prefix_cas_hit: bool,
    pub ascent_choice_status: String,
    pub world_count: usize,
    pub surviving_branch: Option<String>,
    pub open_obligations: Vec<String>,
    pub findings: Vec<AitiaMagitFinding>,
    pub quint_overlay: Option<AitiaMagitOverlaySummary>,
    pub lean_overlay: Option<AitiaMagitOverlaySummary>,
    pub recommended_formal_models: Vec<String>,
    pub human_disposition_required: bool,
    pub action_authorized: bool,
}

impl AitiaMagitReviewSnapshot {
    /// Project a raw `StepReview` JSON envelope into a typed Magit review panel
    /// snapshot, failing closed unless `actionAuthorized == false`.
    pub fn from_review_response(resp: &Value) -> Result<Self, String> {
        if resp.get("actionAuthorized").and_then(Value::as_bool) != Some(false) {
            return Err(
                "fail-closed Magit projection: StepReview response must certify actionAuthorized=false"
                    .to_owned(),
            );
        }
        let sp = &resp["sharedPrefix"];
        let target_repo = sp["targetRepo"]
            .as_str()
            .unwrap_or("external-repo")
            .to_owned();
        let base_ref = sp["baseRef"].as_str().unwrap_or("HEAD").to_owned();
        let context_digest = sp["contextDigest"].as_str().unwrap_or("").to_owned();
        let diff_decision = sp["diffDecision"].as_str().unwrap_or("REFUSE").to_owned();
        let core_floor_preserved = sp["coreFloorPreserved"].as_bool().unwrap_or(false);
        let shared_prefix_cas_hit = sp["sharedPrefixCasHit"].as_bool().unwrap_or(false);

        let sol = &resp["ascentChoiceSolution"];
        let ascent_choice_status = sol["choiceStatus"]
            .as_str()
            .unwrap_or("unsatisfiable")
            .to_owned();
        let world_count = usize::try_from(sol["worldCount"].as_u64().unwrap_or(0)).unwrap_or(0);
        let surviving_branch = sol["worlds"]
            .as_array()
            .and_then(|arr| {
                (arr.len() == 1)
                    .then(|| arr[0]["reviewBranch"].as_str())
                    .flatten()
            })
            .map(str::to_owned);

        let open_obligations = resp["openObligations"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        let findings = resp["diffFindings"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|f| AitiaMagitFinding {
                        id: f["id"].as_str().unwrap_or("").to_owned(),
                        file: f["file"].as_str().unwrap_or("").to_owned(),
                        line: usize::try_from(f["line"].as_u64().unwrap_or(0)).unwrap_or(0),
                        rule: f["rule"].as_str().unwrap_or("").to_owned(),
                        hallucination_class: f["hallucinationClass"]
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                        detail: f["detail"].as_str().unwrap_or("").to_owned(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let recommended_formal_models = resp["recommendedFormalModels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["modelId"].as_str())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        let quint_overlay = resp
            .get("formalOverlayExecution")
            .filter(|v| v.is_object())
            .map(|v| parse_overlay_summary(v, "quint"));
        let lean_overlay = resp
            .get("leanOverlayExecution")
            .filter(|v| v.is_object())
            .map(|v| parse_overlay_summary(v, "lean4"));

        Ok(Self {
            target_repo,
            base_ref,
            context_digest,
            diff_decision,
            core_floor_preserved,
            shared_prefix_cas_hit,
            ascent_choice_status,
            world_count,
            surviving_branch,
            open_obligations,
            findings,
            quint_overlay,
            lean_overlay,
            recommended_formal_models,
            human_disposition_required: true,
            action_authorized: false,
        })
    }
}

fn parse_overlay_summary(v: &Value, kind: &str) -> AitiaMagitOverlaySummary {
    AitiaMagitOverlaySummary {
        model_id: v["modelId"].as_str().unwrap_or("").to_owned(),
        formal_kind: kind.to_owned(),
        machine_executed: v["machineExecuted"].as_bool().unwrap_or(false),
        passed: v["passed"].as_bool().unwrap_or(false),
        countermodel_verified: v["countermodelVerified"].as_bool().unwrap_or(false),
        stored_in_sidecar_vault: v["storedInSidecarVault"].as_bool().unwrap_or(false),
        resolver_digest: v["resolverDigest"].as_str().unwrap_or("").to_owned(),
        archived_overlay_path: v["archivedOverlayPath"].as_str().map(str::to_owned),
        archived_receipt_path: v["archivedReceiptPath"].as_str().map(str::to_owned),
    }
}

fn encode_grpc_frame(payload: &[u8]) -> Result<Vec<u8>, String> {
    let len = u32::try_from(payload.len())
        .map_err(|_| "gRPC request payload exceeds u32 frame limit".to_owned())?;
    let mut frame = Vec::with_capacity(5 + payload.len());
    frame.push(0u8);
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

fn decode_grpc_frame(body: &[u8]) -> Result<&[u8], String> {
    if body.len() < 5 {
        return Err("aitia gRPC response shorter than 5-byte frame prefix".to_owned());
    }
    if body[0] != 0 {
        return Err("compressed aitia gRPC frames are not supported".to_owned());
    }
    let declared = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
    if declared != body.len() - 5 {
        return Err(format!(
            "aitia gRPC frame length mismatch: declared={declared}, actual={}",
            body.len() - 5
        ));
    }
    Ok(&body[5..])
}

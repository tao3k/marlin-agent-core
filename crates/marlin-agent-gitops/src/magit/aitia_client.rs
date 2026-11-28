// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Real RFC 0024 Lambda Aitia gRPC client (`aitia.marlin.v1.AitiaSidecarGateway`)
//! speaking length-prefixed `Content-Type: application/grpc+json` frames to a
//! live `aitia serve-marlin-grpc` daemon (either pre-configured via
//! `MARLIN_AITIA_GRPC_ADDR` or supervised locally from `~/.local/bin/aitia`).
//!
//! Fails closed to `AitiaLiveReport::offline(...)` when Lambda Aitia is absent
//! or unreachable — never fabricates risk verdicts or silently invents findings.

use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    process::{Child, Command},
    sync::Mutex,
    time::timeout,
};

const SERVICE_PREFIX: &str = "/aitia.marlin.v1.AitiaSidecarGateway/";
const AITIA_RPC_TIMEOUT: Duration = Duration::from_secs(12);
const AITIA_STEP_REVIEW_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_GRPC_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

/// Typed request for `AitiaSidecarGateway::StepReview` (RFC 0008 / RFC 0026).
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AitiaStepReviewRequest {
    pub repo_dir: PathBuf,
    pub base_ref: String,
    pub vault_dir: Option<PathBuf>,
    pub plan_dsl: Option<String>,
    pub formal_overlay_source: Option<String>,
    pub formal_overlay_name: Option<String>,
    pub lean_overlay_source: Option<String>,
    pub lean_overlay_name: Option<String>,
}

/// Read-only projection of a machine-executed Quint 0.33 or Lean 4.36 MR05
/// ephemeral formal overlay (plus external sidecar vault archive paths).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AitiaFormalOverlayProjection {
    pub tool: String, // "quint" | "lean4"
    pub passed: bool,
    pub countermodel_verified: bool,
    pub proof_clean: bool,
    pub stored_in_sidecar_vault: bool,
    pub extends_model: Option<String>,
    pub archived_overlay_path: Option<String>,
    pub archived_receipt_path: Option<String>,
    pub item_count: usize,
}

/// Recommended upstream Deterministic Engineering formal model from the catalog.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AitiaFormalModelRecommendation {
    pub model_id: String,
    pub tool: String,
    pub library: String,
    pub rationale: String,
}

/// Read-only Magit human disposition projection of a `StepReview` response.
/// Enforces fail-closed zero-authority (`action_authorized == false`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AitiaProgressiveReviewSnapshot {
    pub choice_status: String,
    pub world_count: usize,
    pub admissible: bool,
    pub action_authorized: bool,
    pub human_disposition_required: bool,
    pub shared_prefix_cas_hit: bool,
    pub plan_format: String,
    pub plan_valid: bool,
    pub plan_acyclic: bool,
    pub context_digest: String,
    pub open_obligations: Vec<String>,
    pub active_rule_packs: Vec<String>,
    pub findings: Vec<AitiaG2Finding>,
    pub quint_overlay: Option<AitiaFormalOverlayProjection>,
    pub lean_overlay: Option<AitiaFormalOverlayProjection>,
    pub recommended_formal_models: Vec<AitiaFormalModelRecommendation>,
}

impl AitiaProgressiveReviewSnapshot {
    /// Parses a `StepReview` JSON response into a read-only Magit disposition snapshot.
    /// Fails closed if `actionAuthorized` is not explicitly `false`.
    pub fn from_step_review_json(val: &Value) -> Result<Self, String> {
        let action_authorized = val.get("actionAuthorized").and_then(Value::as_bool);
        if action_authorized != Some(false) {
            return Err(format!(
                "zero-authority invariant violated: expected actionAuthorized=false, got {action_authorized:?}"
            ));
        }

        let sp = val.get("sharedPrefix");
        let sol = val.get("ascentChoiceSolution");
        let pv = val.get("planValidation");

        let choice_status = sol
            .and_then(|s| s.get("choiceStatus"))
            .or_else(|| val.get("choiceStatus"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let world_count = sol
            .and_then(|s| s.get("worldCount"))
            .or_else(|| val.get("worldCount"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        let admissible = val
            .get("admissible")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let shared_prefix_cas_hit = sp
            .and_then(|s| s.get("sharedPrefixCasHit"))
            .or_else(|| {
                val.get("funflowState")
                    .and_then(|s| s.get("sharedPrefixCasHit"))
            })
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let plan_format = pv
            .and_then(|p| p.get("planFormat"))
            .or_else(|| val.get("planFormat"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let plan_valid = pv
            .and_then(|p| p.get("valid"))
            .or_else(|| val.get("planValid"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let plan_acyclic = pv
            .and_then(|p| p.get("acyclic"))
            .or_else(|| val.get("planAcyclic"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let context_digest = sp
            .and_then(|s| s.get("contextDigest"))
            .or_else(|| val.get("contextDigest"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let open_obligations: Vec<String> = val
            .get("openObligations")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let active_rule_packs: Vec<String> = sp
            .and_then(|s| s.get("activeRulePacks"))
            .or_else(|| {
                val.get("funflowState")
                    .and_then(|s| s.get("activeRulePacks"))
            })
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        let mut findings = Vec::new();
        let raw_findings = val
            .get("diffFindings")
            .and_then(Value::as_array)
            .or_else(|| {
                val.get("diffDefense")
                    .and_then(|d| d.get("g2Findings"))
                    .and_then(Value::as_array)
            });
        if let Some(arr) = raw_findings {
            for item in arr {
                let file = item
                    .get("file")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim_start_matches("./")
                    .to_owned();
                if file.is_empty() {
                    continue;
                }
                findings.push(AitiaG2Finding {
                    surface: item
                        .get("surface")
                        .and_then(Value::as_str)
                        .unwrap_or("diff")
                        .to_owned(),
                    subdomain: item
                        .get("subdomain")
                        .and_then(Value::as_str)
                        .unwrap_or("core")
                        .to_owned(),
                    hallucination_class: item
                        .get("hallucinationClass")
                        .and_then(Value::as_str)
                        .unwrap_or("finding")
                        .to_owned(),
                    rule: item
                        .get("rule")
                        .and_then(Value::as_str)
                        .unwrap_or("g2")
                        .to_owned(),
                    file,
                    line: item.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
                    excerpt: item
                        .get("excerpt")
                        .or_else(|| item.get("detail"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .chars()
                        .take(160)
                        .collect(),
                });
            }
        }

        let quint_overlay = val
            .get("formalOverlayExecution")
            .filter(|v| v.is_object())
            .map(|q| AitiaFormalOverlayProjection {
                tool: "quint".to_owned(),
                passed: q.get("passed").and_then(Value::as_bool).unwrap_or(false),
                countermodel_verified: q
                    .get("countermodelVerified")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                proof_clean: q
                    .get("typecheckPassed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                stored_in_sidecar_vault: q
                    .get("storedInSidecarVault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                extends_model: q
                    .get("extendsModel")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                archived_overlay_path: q
                    .get("archivedOverlayPath")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                archived_receipt_path: q
                    .get("archivedReceiptPath")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                item_count: q
                    .get("passingTests")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0),
            });

        let lean_overlay = val
            .get("leanOverlayExecution")
            .filter(|v| v.is_object())
            .map(|l| AitiaFormalOverlayProjection {
                tool: "lean4".to_owned(),
                passed: l.get("passed").and_then(Value::as_bool).unwrap_or(false),
                countermodel_verified: l
                    .get("countermodelVerified")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                proof_clean: l
                    .get("proofClean")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                stored_in_sidecar_vault: l
                    .get("storedInSidecarVault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                extends_model: l
                    .get("extendsModel")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                archived_overlay_path: l
                    .get("archivedOverlayPath")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                archived_receipt_path: l
                    .get("archivedReceiptPath")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                item_count: l
                    .get("theorems")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0),
            });

        let recommended_formal_models = val
            .get("recommendedFormalModels")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        let model_id = m.get("modelId")?.as_str()?.to_owned();
                        Some(AitiaFormalModelRecommendation {
                            model_id,
                            tool: m
                                .get("tool")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown")
                                .to_owned(),
                            library: m
                                .get("library")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_owned(),
                            rationale: m
                                .get("rationale")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_owned(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            choice_status,
            world_count,
            admissible,
            action_authorized: false,
            human_disposition_required: !admissible || !open_obligations.is_empty(),
            shared_prefix_cas_hit,
            plan_format,
            plan_valid,
            plan_acyclic,
            context_digest,
            open_obligations,
            active_rule_packs,
            findings,
            quint_overlay,
            lean_overlay,
            recommended_formal_models,
        })
    }
}

/// One real finding returned by Lambda Aitia's Scheme POO + Ascent G2 Defense Audit.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AitiaG2Finding {
    pub surface: String,
    pub subdomain: String,
    pub hallucination_class: String,
    pub rule: String,
    pub file: String,
    pub line: u32,
    pub excerpt: String,
}

/// Authoritative structured report returned by `AitiaSidecarGateway::AuditDiffDefense`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AitiaLiveReport {
    pub online: bool,
    pub transport: String, // "grpc" | "offline"
    pub decision: String,  // "ADMIT" | "BLOCK" | "REVIEW" | "OFFLINE"
    pub engine_owner: String,
    pub report_digest: Option<String>,
    pub causal_acyclic: Option<bool>,
    pub total_findings: u32,
    pub counts_by_class: BTreeMap<String, u32>,
    pub findings: Vec<AitiaG2Finding>,
    pub note: String,
}

impl AitiaLiveReport {
    pub fn offline(reason: impl Into<String>) -> Self {
        Self {
            online: false,
            transport: "offline".to_owned(),
            decision: "OFFLINE".to_owned(),
            engine_owner: "none".to_owned(),
            report_digest: None,
            causal_acyclic: None,
            total_findings: 0,
            counts_by_class: BTreeMap::new(),
            findings: Vec::new(),
            note: reason.into(),
        }
    }
}

/// Persistent client for `aitia.marlin.v1.AitiaSidecarGateway`.
/// Automatically attaches to `MARLIN_AITIA_GRPC_ADDR` when set, or lazily spawns
/// a loopback `aitia serve-marlin-grpc --bind 127.0.0.1:0` sidecar daemon if the
/// `aitia` binary is installed (`MARLIN_AITIA_BIN` / `~/.local/bin/aitia` / `PATH`).
#[derive(Clone, Default)]
pub struct AitiaMarlinGrpcClient {
    inner: Arc<Mutex<AitiaClientState>>,
}

#[derive(Default)]
struct AitiaClientState {
    listen_addr: Option<String>,
    bearer_token: Option<String>,
    child: Option<Child>,
    spawn_attempted: bool,
}

impl Drop for AitiaClientState {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.start_kill();
        }
    }
}

impl AitiaMarlinGrpcClient {
    /// Creates a client pointing at an explicit `aitia serve-marlin-grpc` address.
    pub fn with_endpoint(addr: impl Into<String>, bearer_token: Option<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(AitiaClientState {
                listen_addr: Some(addr.into()),
                bearer_token,
                child: None,
                spawn_attempted: true,
            })),
        }
    }

    /// Creates a client using `MARLIN_AITIA_GRPC_ADDR` or auto-managed local sidecar.
    pub fn from_env() -> Self {
        let addr = env::var("MARLIN_AITIA_GRPC_ADDR")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        let token = env::var("AITIA_GRPC_BEARER_TOKEN")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        Self {
            inner: Arc::new(Mutex::new(AitiaClientState {
                listen_addr: addr.clone(),
                bearer_token: token,
                child: None,
                spawn_attempted: addr.is_some(),
            })),
        }
    }

    /// Runs `AitiaSidecarGateway::AuditDiffDefense` over length-prefixed gRPC frames.
    /// Never fails the Magit snapshot on sidecar unavailability — instead returns an
    /// explicit `AitiaLiveReport::offline(...)` so the UI never fakes a risk score.
    pub async fn audit_diff_defense(&self, repo_dir: &Path, base_ref: &str) -> AitiaLiveReport {
        if env::var("MARLIN_AITIA_DISABLE")
            .ok()
            .is_some_and(|v| v.trim() == "1")
        {
            return AitiaLiveReport::offline("disabled via MARLIN_AITIA_DISABLE=1");
        }
        let (addr, token) = match self.ensure_endpoint().await {
            Ok(pair) => pair,
            Err(reason) => return AitiaLiveReport::offline(reason),
        };
        let payload = json!({
            "repoDir": repo_dir.to_string_lossy(),
            "baseRef": base_ref,
            "stagedOnly": false,
        });
        match invoke_grpc_json(&addr, token.as_deref(), "AuditDiffDefense", &payload).await {
            Ok(val) => parse_audit_defense_report(&val),
            Err(err_msg) => {
                // Reset cached local sidecar so a dead child can respawn cleanly next refresh.
                let mut st = self.inner.lock().await;
                if st.child.is_some() {
                    if let Some(ref mut c) = st.child {
                        let _ = c.start_kill();
                    }
                    st.child = None;
                    st.listen_addr = None;
                    st.spawn_attempted = false;
                }
                AitiaLiveReport::offline(format!("Aitia gRPC call failed: {err_msg}"))
            }
        }
    }

    /// Calls `AitiaSidecarGateway::Health` to verify a live gRPC daemon.
    pub async fn health(&self) -> Result<Value, String> {
        let (addr, token) = self.ensure_endpoint().await?;
        invoke_grpc_json(&addr, token.as_deref(), "Health", &json!({})).await
    }

    /// Calls `AitiaSidecarGateway::GetToolSchema` to fetch the authoritative
    /// self-discovering JSON Schema for `aitia_review_step`.
    pub async fn get_tool_schema(&self) -> Result<Value, String> {
        let (addr, token) = self.ensure_endpoint().await?;
        invoke_grpc_json(&addr, token.as_deref(), "GetToolSchema", &json!({})).await
    }

    /// Calls `AitiaSidecarGateway::StepReview` (RFC 0008 / RFC 0026) and returns
    /// a fail-closed, zero-authority `AitiaProgressiveReviewSnapshot`.
    pub async fn step_review(
        &self,
        req: &AitiaStepReviewRequest,
    ) -> Result<AitiaProgressiveReviewSnapshot, String> {
        if env::var("MARLIN_AITIA_DISABLE")
            .ok()
            .is_some_and(|v| v.trim() == "1")
        {
            return Err("disabled via MARLIN_AITIA_DISABLE=1".to_owned());
        }
        let (addr, token) = self.ensure_endpoint().await?;
        let base_ref = if req.base_ref.trim().is_empty() {
            "HEAD"
        } else {
            req.base_ref.as_str()
        };
        let mut payload = json!({
            "repo": req.repo_dir.to_string_lossy(),
            "baseRef": base_ref,
        });
        if let Some(ref vault) = req.vault_dir {
            payload["vaultDir"] = json!(vault.to_string_lossy());
        }
        if let Some(ref plan) = req.plan_dsl {
            payload["planSexp"] = json!(plan);
        }
        if let Some(ref src) = req.formal_overlay_source {
            payload["formalOverlaySource"] = json!(src);
        }
        if let Some(ref name) = req.formal_overlay_name {
            payload["formalOverlayName"] = json!(name);
        }
        if let Some(ref src) = req.lean_overlay_source {
            payload["leanOverlaySource"] = json!(src);
        }
        if let Some(ref name) = req.lean_overlay_name {
            payload["leanOverlayName"] = json!(name);
        }
        let raw = invoke_grpc_json_with_timeout(
            &addr,
            token.as_deref(),
            "StepReview",
            &payload,
            AITIA_STEP_REVIEW_TIMEOUT,
        )
        .await?;
        AitiaProgressiveReviewSnapshot::from_step_review_json(&raw)
    }

    async fn ensure_endpoint(&self) -> Result<(String, Option<String>), String> {
        let mut st = self.inner.lock().await;
        if let Some(ref addr) = st.listen_addr {
            return Ok((addr.clone(), st.bearer_token.clone()));
        }
        if st.spawn_attempted {
            return Err("aitia binary unavailable or failed to start".to_owned());
        }
        st.spawn_attempted = true;

        let bin = find_aitia_binary().ok_or_else(|| {
            "aitia binary not found (install ~/.local/bin/aitia or set MARLIN_AITIA_GRPC_ADDR)"
                .to_owned()
        })?;
        let mut cmd = Command::new(&bin);
        cmd.args(["serve-marlin-grpc", "--bind", "127.0.0.1:0"])
            .env_remove("GERBIL_LOADPATH")
            .env_remove("GERBIL_PATH")
            .env_remove("AITIA_PROGRAM_MANIFEST")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn {}: {e}", bin.display()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "aitia serve-marlin-grpc missing stdout pipe".to_owned())?;
        let mut reader = BufReader::new(stdout);
        let mut ready_line = String::new();
        let read_res = timeout(Duration::from_secs(10), reader.read_line(&mut ready_line)).await;
        match read_res {
            Ok(Ok(n)) if n > 0 => {
                let v: Value = serde_json::from_str(ready_line.trim()).map_err(|e| {
                    format!("invalid aitia ready JSON {:?}: {e}", ready_line.trim())
                })?;
                let listen = v
                    .get("listenAddr")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "aitia ready JSON missing listenAddr".to_owned())?
                    .to_owned();
                st.listen_addr = Some(listen.clone());
                st.child = Some(child);
                Ok((listen, st.bearer_token.clone()))
            }
            _ => {
                let _ = child.start_kill();
                Err("timed out waiting for aitia serve-marlin-grpc ready line".to_owned())
            }
        }
    }
}

fn find_aitia_binary() -> Option<PathBuf> {
    if let Ok(explicit) = env::var("MARLIN_AITIA_BIN") {
        let p = PathBuf::from(explicit.trim());
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(home) = env::var("HOME") {
        let candidate = PathBuf::from(home).join(".local/bin/aitia");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join("aitia");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

async fn invoke_grpc_json(
    addr: &str,
    bearer_token: Option<&str>,
    method: &str,
    payload: &Value,
) -> Result<Value, String> {
    invoke_grpc_json_with_timeout(addr, bearer_token, method, payload, AITIA_RPC_TIMEOUT).await
}

async fn invoke_grpc_json_with_timeout(
    addr: &str,
    bearer_token: Option<&str>,
    method: &str,
    payload: &Value,
    rpc_timeout: Duration,
) -> Result<Value, String> {
    let payload_bytes = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let payload_len =
        u32::try_from(payload_bytes.len()).map_err(|_| "gRPC payload exceeds u32".to_owned())?;
    let mut frame = Vec::with_capacity(5 + payload_bytes.len());
    frame.push(0u8);
    frame.extend_from_slice(&payload_len.to_be_bytes());
    frame.extend_from_slice(&payload_bytes);

    let mut req_head = format!(
        "POST {SERVICE_PREFIX}{method} HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Content-Type: application/grpc+json\r\n\
         Content-Length: {}\r\n",
        frame.len()
    );
    if let Some(tok) = bearer_token {
        req_head.push_str(&format!("Authorization: Bearer {tok}\r\n"));
    }
    req_head.push_str("Connection: close\r\n\r\n");

    let fut = async {
        let mut stream = TcpStream::connect(addr)
            .await
            .map_err(|e| format!("connect {addr}: {e}"))?;
        stream
            .write_all(req_head.as_bytes())
            .await
            .map_err(|e| format!("write headers: {e}"))?;
        stream
            .write_all(&frame)
            .await
            .map_err(|e| format!("write frame: {e}"))?;
        stream.flush().await.map_err(|e| format!("flush: {e}"))?;

        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        reader
            .read_line(&mut status_line)
            .await
            .map_err(|e| format!("read status: {e}"))?;
        let mut content_len: usize = 0;
        let mut grpc_status: u8 = 0;
        let mut grpc_message = String::new();
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("read header: {e}"))?;
            if n == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                match k.trim().to_ascii_lowercase().as_str() {
                    "content-length" => {
                        content_len = v.trim().parse::<usize>().unwrap_or(0);
                    }
                    "grpc-status" => {
                        grpc_status = v.trim().parse::<u8>().unwrap_or(2);
                    }
                    "grpc-message" => {
                        grpc_message = v.trim().to_owned();
                    }
                    _ => {}
                }
            }
        }
        if content_len > MAX_GRPC_RESPONSE_BYTES {
            return Err("Aitia gRPC response exceeded bounded size limit".to_owned());
        }
        let mut body = vec![0u8; content_len];
        if content_len > 0 {
            reader
                .read_exact(&mut body)
                .await
                .map_err(|e| format!("read frame body: {e}"))?;
        }
        if body.len() < 5 {
            return Err(format!(
                "short gRPC frame (status={grpc_status}, msg={grpc_message})"
            ));
        }
        let declared = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
        if declared > body.len() - 5 {
            return Err("truncated gRPC frame".to_owned());
        }
        let json_slice = &body[5..5 + declared];
        if grpc_status != 0 {
            return Err(format!("grpc-status={grpc_status} ({grpc_message})"));
        }
        serde_json::from_slice::<Value>(json_slice)
            .map_err(|e| format!("invalid JSON response: {e}"))
    };

    timeout(rpc_timeout, fut)
        .await
        .map_err(|_| "Aitia gRPC request timed out".to_owned())?
}

fn parse_audit_defense_report(val: &Value) -> AitiaLiveReport {
    let decision = val
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN")
        .to_owned();
    let engine_owner = val
        .get("engineOwner")
        .and_then(Value::as_str)
        .unwrap_or("lambda-aitia")
        .to_owned();
    let report_digest = val
        .get("reportDigest")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let causal_acyclic = val
        .get("temporalCausality")
        .and_then(|tc| tc.get("acyclic"))
        .and_then(Value::as_bool);
    let total_findings = val
        .get("totalFindings")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;

    let mut counts_by_class = BTreeMap::new();
    if let Some(obj) = val.get("countsByClass").and_then(Value::as_object) {
        for (k, v) in obj {
            if let Some(n) = v.as_u64() {
                counts_by_class.insert(k.clone(), n as u32);
            }
        }
    }

    let mut findings = Vec::new();
    if let Some(arr) = val.get("g2Findings").and_then(Value::as_array) {
        for item in arr {
            let file = item
                .get("file")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim_start_matches("./")
                .to_owned();
            if file.is_empty() {
                continue;
            }
            findings.push(AitiaG2Finding {
                surface: item
                    .get("surface")
                    .and_then(Value::as_str)
                    .unwrap_or("diff")
                    .to_owned(),
                subdomain: item
                    .get("subdomain")
                    .and_then(Value::as_str)
                    .unwrap_or("core")
                    .to_owned(),
                hallucination_class: item
                    .get("hallucinationClass")
                    .and_then(Value::as_str)
                    .unwrap_or("finding")
                    .to_owned(),
                rule: item
                    .get("rule")
                    .and_then(Value::as_str)
                    .unwrap_or("g2")
                    .to_owned(),
                file,
                line: item.get("line").and_then(Value::as_u64).unwrap_or(0) as u32,
                excerpt: item
                    .get("excerpt")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .chars()
                    .take(160)
                    .collect(),
            });
        }
    }

    AitiaLiveReport {
        online: true,
        transport: "grpc".to_owned(),
        decision: decision.clone(),
        engine_owner: engine_owner.clone(),
        report_digest,
        causal_acyclic,
        total_findings,
        counts_by_class,
        findings,
        note: format!(
            "Aitia gRPC ({engine_owner}): decision={decision}, findings={total_findings}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progressive_review_snapshot_enforces_zero_authority_and_extracts_vault_overlays() {
        let forbidden = json!({
            "actionAuthorized": true,
            "choiceStatus": "unique-deduction",
            "worldCount": 1
        });
        let err = AitiaProgressiveReviewSnapshot::from_step_review_json(&forbidden)
            .expect_err("must reject actionAuthorized=true fail-closed");
        assert!(err.contains("zero-authority"));

        let missing = json!({
            "choiceStatus": "unique-deduction",
            "worldCount": 1
        });
        assert!(AitiaProgressiveReviewSnapshot::from_step_review_json(&missing).is_err());

        let valid = json!({
            "actionAuthorized": false,
            "admissible": false,
            "choiceStatus": "unique-deduction",
            "worldCount": 1,
            "planFormat": "sexp-aitia-review-plan",
            "planValid": true,
            "planAcyclic": true,
            "contextDigest": "sha256:ctx-1",
            "openObligations": ["SWE-083", "SWE-087"],
            "funflowState": {
                "sharedPrefixCasHit": true,
                "activeRulePacks": ["de-g2-core"]
            },
            "formalOverlayExecution": {
                "passed": true,
                "typecheckPassed": true,
                "countermodelVerified": true,
                "storedInSidecarVault": true,
                "extendsModel": "oauth-token-lifecycle",
                "archivedOverlayPath": "/tmp/vault/a.qnt",
                "archivedReceiptPath": "/tmp/vault/a.qnt.receipt.json",
                "passingTests": ["safePathTest", "mr05CountermodelTest"]
            },
            "leanOverlayExecution": {
                "passed": true,
                "proofClean": true,
                "countermodelVerified": true,
                "storedInSidecarVault": true,
                "extendsModel": "ControlPlaneAuthAndGovernanceConfinement",
                "archivedOverlayPath": "/tmp/vault/b.lean",
                "archivedReceiptPath": "/tmp/vault/b.lean.receipt.json",
                "theorems": ["safePath", "mr05Countermodel"]
            },
            "recommendedFormalModels": [
                {
                    "modelId": "oauth-token-lifecycle",
                    "tool": "quint",
                    "library": "@deterministic-engineering/quint-models",
                    "rationale": "Auth token lifecycle invariants"
                }
            ]
        });

        let snap = AitiaProgressiveReviewSnapshot::from_step_review_json(&valid)
            .expect("valid zero-authority StepReview response must parse");
        assert!(!snap.action_authorized);
        assert!(snap.human_disposition_required);
        assert!(snap.shared_prefix_cas_hit);
        assert_eq!(snap.world_count, 1);
        assert_eq!(snap.choice_status, "unique-deduction");
        let q = snap.quint_overlay.expect("quint overlay");
        assert!(q.passed && q.countermodel_verified && q.stored_in_sidecar_vault);
        assert_eq!(q.archived_overlay_path.as_deref(), Some("/tmp/vault/a.qnt"));
        assert_eq!(q.item_count, 2);
        let l = snap.lean_overlay.expect("lean overlay");
        assert!(l.passed && l.proof_clean && l.countermodel_verified);
        assert_eq!(
            l.archived_overlay_path.as_deref(),
            Some("/tmp/vault/b.lean")
        );
        assert_eq!(snap.recommended_formal_models.len(), 1);
    }
}

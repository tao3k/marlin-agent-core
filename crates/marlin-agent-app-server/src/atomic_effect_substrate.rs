// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Production thin, domain-agnostic 4-primitive leaf substrate implementing
//! `marlin_agent_runtime::AtomicEffectExecutor` over:
//!   1. `marlin_agent_storage::AgentStorage` (`TursoAgentStorage` SQLite WAL or in-memory)
//!      for `CasGet` / `CasPut`
//!   2. `marlin_agent_protocol::ModelGateway` for `LlmComplete`
//!   3. Pluggable `AtomicSandboxRunner` (default fail-closed `DenySandboxRunner` or
//!      explicit `BoundedTokioSandboxRunner`) for `SandboxRun`
//!
//! Zero high-order orchestration lives here; Scheme owns all scheduling, caching policy,
//! prompt construction, detour rules, and hypothesis inference.

use std::collections::BTreeSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use marlin_agent_kernel::{
    DerivationId, ExecutionCircuitBreaker, GovernanceAdmissionGate, GovernedSubjectName,
    ToolBudgetUnits, ToolIfcContext,
};
use marlin_agent_protocol::ModelGateway;
use marlin_agent_runtime::{
    AtomicEffect, AtomicEffectExecutor, AtomicEffectReceipt, CasHitReceiptPayload,
    EffectErrorReceiptPayload, LlmCompletedReceiptPayload, SandboxExitedReceiptPayload,
    SandboxRunEffectPayload, sha256_prefixed_hex,
};
use marlin_agent_storage::{
    AgentId, AgentStorage, ArtifactHash, ArtifactPointerKey, ArtifactPointerUpdate, ArtifactRecord,
    EventId, ProjectId, SessionId,
};

/// Leaf execution hook for `AtomicEffect::SandboxRun`.
pub trait AtomicSandboxRunner: Send + Sync {
    fn run_sandbox<'a>(
        &'a self,
        payload: SandboxRunEffectPayload,
    ) -> Pin<Box<dyn Future<Output = AtomicEffectReceipt> + Send + 'a>>;
}

/// Default fail-closed sandbox runner rejecting unmounted sandbox execution.
#[derive(Clone, Copy, Debug, Default)]
pub struct DenySandboxRunner;

impl AtomicSandboxRunner for DenySandboxRunner {
    fn run_sandbox<'a>(
        &'a self,
        payload: SandboxRunEffectPayload,
    ) -> Pin<Box<dyn Future<Output = AtomicEffectReceipt> + Send + 'a>> {
        Box::pin(async move {
            AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                effect_id: payload.effect_id,
                message: "sandbox runner denied: no authorized sandbox policy mounted".to_owned(),
            })
        })
    }
}

/// Bounded Tokio command runner for explicitly admitted `AtomicEffect::SandboxRun` payloads.
///
/// Routes every child process through `marlin_agent_runtime::AsyncManagedChildProcess`
/// so sandbox commands inherit Unix process-group containment, graceful `SIGTERM` ->
/// `SIGKILL` shutdown, persistent ledger tracking, and OpenTelemetry `runtime.process` spans.
#[derive(Clone, Debug)]
pub struct BoundedTokioSandboxRunner {
    timeout: Duration,
    max_output_bytes: usize,
    runtime_context: Option<marlin_agent_runtime::RuntimeContext>,
}

impl BoundedTokioSandboxRunner {
    /// Creates a bounded command runner enforcing `timeout` and `max_output_bytes`.
    pub fn new(timeout: Duration, max_output_bytes: usize) -> Self {
        Self {
            timeout,
            max_output_bytes: max_output_bytes.max(1),
            runtime_context: None,
        }
    }

    /// Bind a host `RuntimeContext` so every sandboxed child shares the host's
    /// process registry, persistent ledger, and admission quota.
    pub fn with_runtime_context(mut self, context: marlin_agent_runtime::RuntimeContext) -> Self {
        self.runtime_context = Some(context);
        self
    }
}

const SAFE_SANDBOX_ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "HOME",
    "TMPDIR",
    "TEMP",
    "TMP",
    "LANG",
    "LC_ALL",
    "TERM",
    "USER",
    "LOGNAME",
    "TZ",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "NIX_SSL_CERT_FILE",
];

fn apply_scrubbed_env(cmd: &mut tokio::process::Command) {
    cmd.env_clear();
    for key in SAFE_SANDBOX_ENV_ALLOWLIST {
        if let Some(val) = std::env::var_os(key) {
            cmd.env(key, val);
        }
    }
}

impl AtomicSandboxRunner for BoundedTokioSandboxRunner {
    fn run_sandbox<'a>(
        &'a self,
        payload: SandboxRunEffectPayload,
    ) -> Pin<Box<dyn Future<Output = AtomicEffectReceipt> + Send + 'a>> {
        let timeout = self.timeout;
        let max_bytes = self.max_output_bytes;
        let bound_context = self.runtime_context.clone();
        Box::pin(async move {
            let (_ephemeral_runtime, context) = match bound_context {
                Some(ctx) => (None, ctx.child_context()),
                None => {
                    let (rt, _events) = marlin_agent_runtime::TokioAgentRuntime::new(16);
                    let ctx = rt.context();
                    (Some(rt), ctx)
                }
            };
            let mut cmd = tokio::process::Command::new(&payload.program);
            apply_scrubbed_env(&mut cmd);
            cmd.args(&payload.args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            if let Some(cwd) = &payload.cwd {
                cmd.current_dir(cwd);
            }
            let handle = format!("sandbox:{}", payload.effect_id);
            let mut argv = Vec::with_capacity(payload.args.len() + 1);
            argv.push(payload.program.clone());
            argv.extend(payload.args.iter().cloned());
            let mut command_obs =
                marlin_agent_runtime::process::RuntimeCommandObservation::new("sandbox.run")
                    .with_argv(argv);
            if let Some(cwd) = &payload.cwd {
                command_obs = command_obs.with_cwd(cwd.clone());
            }
            let spec = marlin_agent_runtime::ManagedChildProcessSpec::new(
                marlin_agent_runtime::process::RuntimeProcessKind::Tool,
                handle.clone(),
            )
            .with_handle(handle)
            .with_command(command_obs);

            let child = match marlin_agent_runtime::AsyncManagedChildProcess::spawn_with_spec(
                &context, cmd, spec,
            )
            .await
            {
                Ok(child) => child,
                Err(err) => {
                    return AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                        effect_id: payload.effect_id,
                        message: format!("sandbox spawn failed: {err}"),
                    });
                }
            };

            match tokio::time::timeout(timeout, child.wait_with_bounded_output(max_bytes)).await {
                Ok(Ok(output)) => AtomicEffectReceipt::SandboxExited(SandboxExitedReceiptPayload {
                    effect_id: payload.effect_id,
                    exit_code: output.status.code().unwrap_or(-1),
                    stdout: output.stdout,
                    stderr: output.stderr,
                }),
                Ok(Err(err)) => AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                    effect_id: payload.effect_id,
                    message: format!("sandbox wait failed: {err}"),
                }),
                Err(_) => AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                    effect_id: payload.effect_id,
                    message: format!("sandbox command timed out after {}ms", timeout.as_millis()),
                }),
            }
        })
    }
}

/// Fail-closed policy wrapper around `BoundedTokioSandboxRunner` enforcing:
/// 1. Explicit exact-match `allowed_programs` allowlist, and
/// 2. Optional `confined_root` directory confinement (canonical path check blocking `..` escape).
#[derive(Clone, Debug)]
pub struct AllowlistSandboxRunner {
    inner: BoundedTokioSandboxRunner,
    allowed_programs: BTreeSet<String>,
    confined_root: Option<PathBuf>,
}

impl AllowlistSandboxRunner {
    /// Creates an allowlisted sandbox runner (denies all programs not in `allowed_programs`).
    pub fn new<I, S>(timeout: Duration, max_output_bytes: usize, allowed_programs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            inner: BoundedTokioSandboxRunner::new(timeout, max_output_bytes),
            allowed_programs: allowed_programs.into_iter().map(Into::into).collect(),
            confined_root: None,
        }
    }

    /// Confines all executed commands to `root` (or subdirectories thereof).
    pub fn with_confined_root(mut self, root: impl AsRef<Path>) -> Self {
        self.confined_root = Some(root.as_ref().to_path_buf());
        self
    }

    /// Bind a host `RuntimeContext` for persistent process ledger & quota accounting.
    pub fn with_runtime_context(mut self, context: marlin_agent_runtime::RuntimeContext) -> Self {
        self.inner = self.inner.with_runtime_context(context);
        self
    }

    fn validate_and_resolve_cwd(&self, requested: Option<&str>) -> Result<Option<String>, String> {
        let Some(root) = &self.confined_root else {
            return Ok(requested.map(str::to_owned));
        };
        let canonical_root = std::fs::canonicalize(root)
            .map_err(|err| format!("confined sandbox root unavailable: {err}"))?;
        let candidate = match requested {
            Some(raw) if !raw.trim().is_empty() => {
                let p = Path::new(raw);
                if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    canonical_root.join(p)
                }
            }
            _ => canonical_root.clone(),
        };
        let canonical_cwd = std::fs::canonicalize(&candidate)
            .map_err(|err| format!("sandbox cwd invalid: {err}"))?;
        if !canonical_cwd.starts_with(&canonical_root) {
            return Err("sandbox cwd escapes confined root".to_owned());
        }
        Ok(Some(canonical_cwd.to_string_lossy().into_owned()))
    }

    /// Executes one allowlisted, root-confined command without requiring callers
    /// to import `marlin_agent_runtime` types.
    pub async fn run_command(
        &self,
        program: String,
        args: Vec<String>,
        cwd: Option<String>,
    ) -> Result<(i32, Vec<u8>, Vec<u8>), String> {
        let receipt = self
            .run_sandbox(SandboxRunEffectPayload {
                effect_id: "allowlist-direct".to_owned(),
                program,
                args,
                cwd,
            })
            .await;
        match receipt {
            AtomicEffectReceipt::SandboxExited(p) => Ok((p.exit_code, p.stdout, p.stderr)),
            AtomicEffectReceipt::EffectError(p) => Err(p.message),
            other => Err(format!("unexpected sandbox receipt: {other:?}")),
        }
    }
}

impl AtomicSandboxRunner for AllowlistSandboxRunner {
    fn run_sandbox<'a>(
        &'a self,
        mut payload: SandboxRunEffectPayload,
    ) -> Pin<Box<dyn Future<Output = AtomicEffectReceipt> + Send + 'a>> {
        if !self.allowed_programs.contains(&payload.program) {
            let effect_id = payload.effect_id;
            let program = payload.program;
            return Box::pin(async move {
                AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                    effect_id,
                    message: format!("sandbox program denied by allowlist: {program}"),
                })
            });
        }
        match self.validate_and_resolve_cwd(payload.cwd.as_deref()) {
            Ok(resolved_cwd) => {
                payload.cwd = resolved_cwd;
                self.inner.run_sandbox(payload)
            }
            Err(message) => {
                let effect_id = payload.effect_id;
                Box::pin(async move {
                    AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                        effect_id,
                        message,
                    })
                })
            }
        }
    }
}

static ATOMIC_EXEC_BOOT_SEQ: AtomicU64 = AtomicU64::new(1);

fn cas_pointer_key_for(key: &str) -> Result<ArtifactPointerKey, String> {
    let digest = sha256_prefixed_hex(key.as_bytes());
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest.as_str());
    ArtifactPointerKey::new(format!("ptr:cas:{hex}")).map_err(|e| format!("{e}"))
}

fn cas_now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn cas_new_boot_tag() -> String {
    let pid = std::process::id();
    let boot_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let boot_seq = ATOMIC_EXEC_BOOT_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{pid:x}-{boot_ns:x}-{boot_seq:x}")
}

/// Thin runtime-agnostic CAS adapter over `AgentStorage` sharing the exact `ptr:cas:<sha256>`
/// key scheme with `CompositeAtomicEffectExecutor`.
///
/// Allows `marlin-scheme-host` to execute Scheme-emitted `CasGet` / `CasPut` effects directly
/// against disk `TursoAgentStorage` without importing `marlin-agent-runtime`.
pub struct StorageCasStore<S> {
    storage: S,
    project_id: ProjectId,
    session_id: SessionId,
    agent_id: AgentId,
    boot_tag: String,
    seq: AtomicU64,
}

impl<S: AgentStorage + Send + Sync + 'static> StorageCasStore<S> {
    pub fn new(
        storage: S,
        project_id: ProjectId,
        session_id: SessionId,
        agent_id: AgentId,
    ) -> Self {
        Self {
            storage,
            project_id,
            session_id,
            agent_id,
            boot_tag: cas_new_boot_tag(),
            seq: AtomicU64::new(1),
        }
    }

    pub async fn get_blob(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let ptr_key = cas_pointer_key_for(key)?;
        match self
            .storage
            .get_artifact_pointer(&self.project_id, &ptr_key)
            .await
            .map_err(|err| format!("cas get_artifact_pointer failed: {err}"))?
        {
            Some(pointer) => match self
                .storage
                .get_artifact(&self.project_id, &pointer.target_artifact_hash)
                .await
                .map_err(|err| format!("cas get_artifact failed: {err}"))?
            {
                Some(artifact) => Ok(Some(artifact.body)),
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    pub async fn put_blob(&self, key: &str, bytes: Vec<u8>) -> Result<(), String> {
        let digest = sha256_prefixed_hex(&bytes);
        let hash =
            ArtifactHash::new(digest).map_err(|err| format!("invalid artifact hash: {err}"))?;
        let ptr_key = cas_pointer_key_for(key)?;
        let n = self.seq.fetch_add(1, Ordering::Relaxed);
        let event_id = EventId::new(format!("ev-cas-{}-{n:x}", self.boot_tag))
            .map_err(|err| format!("invalid event id: {err}"))?;
        let now_ms = cas_now_unix_ms();
        self.storage
            .put_artifact(ArtifactRecord {
                project_id: self.project_id.clone(),
                artifact_hash: hash.clone(),
                artifact_kind: "atomic-effect-cas-blob".to_owned(),
                producer_session_id: self.session_id.clone(),
                producer_agent_id: self.agent_id.clone(),
                producer_event_id: event_id.clone(),
                media_type: "application/octet-stream".to_owned(),
                size_bytes: bytes.len() as u64,
                body: bytes,
                created_at_unix_ms: now_ms,
            })
            .await
            .map_err(|err| format!("cas put_artifact failed: {err}"))?;
        let expected = self
            .storage
            .get_artifact_pointer(&self.project_id, &ptr_key)
            .await
            .map_err(|err| format!("cas pointer lookup failed: {err}"))?
            .map(|ptr| ptr.target_artifact_hash);
        self.storage
            .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
                project_id: self.project_id.clone(),
                pointer_key: ptr_key,
                expected_artifact_hash: expected,
                new_artifact_hash: hash,
                updated_by_session_id: self.session_id.clone(),
                updated_by_agent_id: self.agent_id.clone(),
                updated_by_event_id: event_id,
                updated_at_unix_ms: now_ms,
            })
            .await
            .map_err(|err| format!("cas pointer update failed: {err}"))?;
        Ok(())
    }
}

const DEFAULT_ATOMIC_EFFECT_TOOL_BUDGET: u32 = 4096;

/// Thin 4-primitive leaf executor composing `AgentStorage` CAS + `ModelGateway` +
/// `AtomicSandboxRunner` behind an authoritative fail-closed `GovernanceAdmissionGate`
/// (`ExecutionCircuitBreaker` + Cedar + ASCENT + Bi-Temporal TMS, SM25).
pub struct CompositeAtomicEffectExecutor<S, G, R = DenySandboxRunner> {
    storage: S,
    gateway: G,
    sandbox: R,
    governance_gate: Arc<GovernanceAdmissionGate>,
    governance_actor: GovernedSubjectName,
    derivation_id: Option<DerivationId>,
    ifc_context: ToolIfcContext,
    project_id: ProjectId,
    session_id: SessionId,
    agent_id: AgentId,
    boot_tag: String,
    seq: AtomicU64,
}

impl<S, G> CompositeAtomicEffectExecutor<S, G, DenySandboxRunner> {
    /// Constructs a leaf executor with fail-closed `DenySandboxRunner` and a live
    /// `GovernanceAdmissionGate` by default.
    pub fn new(
        storage: S,
        gateway: G,
        project_id: ProjectId,
        session_id: SessionId,
        agent_id: AgentId,
    ) -> Self {
        let pid = std::process::id();
        let boot_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let boot_seq = ATOMIC_EXEC_BOOT_SEQ.fetch_add(1, Ordering::Relaxed);
        let breaker = Arc::new(ExecutionCircuitBreaker::new(ToolBudgetUnits::new(
            DEFAULT_ATOMIC_EFFECT_TOOL_BUDGET,
        )));
        let governance_gate = Arc::new(GovernanceAdmissionGate::new(breaker));
        let governance_actor = GovernedSubjectName::new(agent_id.as_str())
            .unwrap_or_else(|_| GovernedSubjectName::new("agent-atomic-effect").unwrap());
        Self {
            storage,
            gateway,
            sandbox: DenySandboxRunner,
            governance_gate,
            governance_actor,
            derivation_id: None,
            ifc_context: ToolIfcContext::clean_trusted(),
            project_id,
            session_id,
            agent_id,
            boot_tag: format!("{pid:x}-{boot_ns:x}-{boot_seq:x}"),
            seq: AtomicU64::new(1),
        }
    }
}

impl<S, G, R> CompositeAtomicEffectExecutor<S, G, R> {
    /// Mounts an explicit `AtomicSandboxRunner` onto this leaf executor while
    /// preserving the bound `GovernanceAdmissionGate`.
    pub fn with_sandbox<R2>(self, sandbox: R2) -> CompositeAtomicEffectExecutor<S, G, R2> {
        CompositeAtomicEffectExecutor {
            storage: self.storage,
            gateway: self.gateway,
            sandbox,
            governance_gate: self.governance_gate,
            governance_actor: self.governance_actor,
            derivation_id: self.derivation_id,
            ifc_context: self.ifc_context,
            project_id: self.project_id,
            session_id: self.session_id,
            agent_id: self.agent_id,
            boot_tag: self.boot_tag,
            seq: self.seq,
        }
    }

    /// Mounts a shared authoritative `GovernanceAdmissionGate` onto this production
    /// GraphEngine executor so operator E-Stop, Cedar `forbid` rules, ASCENT fencing,
    /// and Bi-Temporal TMS retractions block physical effects fail-closed (SM25).
    pub fn with_governance_gate(
        mut self,
        gate: Arc<GovernanceAdmissionGate>,
        actor: GovernedSubjectName,
        derivation_id: Option<DerivationId>,
    ) -> Self {
        self.governance_gate = gate;
        self.governance_actor = actor;
        self.derivation_id = derivation_id;
        self
    }

    /// Configures the CaMeL/FIDES `ToolIfcContext` evaluated before each mutating effect.
    pub fn with_ifc_context(mut self, ifc_context: ToolIfcContext) -> Self {
        self.ifc_context = ifc_context;
        self
    }

    /// Borrows the authoritative `GovernanceAdmissionGate` mediating this executor.
    pub fn governance_gate(&self) -> &Arc<GovernanceAdmissionGate> {
        &self.governance_gate
    }

    fn effect_id_of(effect: &AtomicEffect) -> String {
        match effect {
            AtomicEffect::CasGet(p) => p.effect_id.clone(),
            AtomicEffect::CasPut(p) => p.effect_id.clone(),
            AtomicEffect::LlmComplete(p) => p.effect_id.clone(),
            AtomicEffect::SandboxRun(p) => p.effect_id.clone(),
        }
    }

    fn authorize_atomic_effect(&self, effect: &AtomicEffect) -> Result<(), String> {
        let (tool_label, resource_label): (&str, &str) = match effect {
            AtomicEffect::CasGet(_) => return Ok(()),
            AtomicEffect::CasPut(p) => ("atomic_effect:cas_put", p.key.as_str()),
            AtomicEffect::LlmComplete(_) => ("atomic_effect:llm_complete", "model_gateway"),
            AtomicEffect::SandboxRun(p) => ("atomic_effect:sandbox_run", p.program.as_str()),
        };
        let tool = GovernedSubjectName::new(tool_label)
            .unwrap_or_else(|_| GovernedSubjectName::new("atomic_effect").unwrap());
        let resource = GovernedSubjectName::new(resource_label)
            .unwrap_or_else(|_| GovernedSubjectName::new("workspace").unwrap());
        self.governance_gate
            .authorize_side_effect_on_resource(
                self.derivation_id.as_ref(),
                &self.governance_actor,
                &tool,
                &resource,
                &self.ifc_context,
            )
            .map(|_| ())
            .map_err(|err| format!("governance_admission_denied: {err}"))
    }

    fn pointer_key_for(key: &str) -> Result<ArtifactPointerKey, String> {
        let digest = sha256_prefixed_hex(key.as_bytes());
        let hex = digest.strip_prefix("sha256:").unwrap_or(digest.as_str());
        ArtifactPointerKey::new(format!("ptr:cas:{hex}")).map_err(|e| format!("{e}"))
    }

    fn now_unix_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

impl<S, G, R> AtomicEffectExecutor for CompositeAtomicEffectExecutor<S, G, R>
where
    S: AgentStorage + Send + Sync + 'static,
    G: ModelGateway,
    R: AtomicSandboxRunner,
{
    fn execute_atomic<'a>(
        &'a self,
        effect: AtomicEffect,
    ) -> Pin<Box<dyn Future<Output = AtomicEffectReceipt> + Send + 'a>> {
        Box::pin(async move {
            if let Err(message) = self.authorize_atomic_effect(&effect) {
                return AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                    effect_id: Self::effect_id_of(&effect),
                    message,
                });
            }
            match effect {
                AtomicEffect::CasGet(p) => {
                    let ptr_key = match Self::pointer_key_for(&p.key) {
                        Ok(k) => k,
                        Err(message) => {
                            return AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                                effect_id: p.effect_id,
                                message,
                            });
                        }
                    };
                    match self
                        .storage
                        .get_artifact_pointer(&self.project_id, &ptr_key)
                        .await
                    {
                        Ok(Some(pointer)) => match self
                            .storage
                            .get_artifact(&self.project_id, &pointer.target_artifact_hash)
                            .await
                        {
                            Ok(Some(artifact)) => {
                                AtomicEffectReceipt::CasHit(CasHitReceiptPayload {
                                    effect_id: p.effect_id,
                                    bytes: artifact.body,
                                })
                            }
                            Ok(None) => AtomicEffectReceipt::CasMiss(p.effect_id),
                            Err(err) => {
                                AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                                    effect_id: p.effect_id,
                                    message: format!("cas get_artifact failed: {err}"),
                                })
                            }
                        },
                        Ok(None) => AtomicEffectReceipt::CasMiss(p.effect_id),
                        Err(err) => AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                            effect_id: p.effect_id,
                            message: format!("cas get_artifact_pointer failed: {err}"),
                        }),
                    }
                }

                AtomicEffect::CasPut(p) => {
                    let digest = sha256_prefixed_hex(&p.bytes);
                    let hash = match ArtifactHash::new(digest) {
                        Ok(h) => h,
                        Err(err) => {
                            return AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                                effect_id: p.effect_id,
                                message: format!("invalid artifact hash: {err}"),
                            });
                        }
                    };
                    let ptr_key = match Self::pointer_key_for(&p.key) {
                        Ok(k) => k,
                        Err(message) => {
                            return AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                                effect_id: p.effect_id,
                                message,
                            });
                        }
                    };
                    let n = self.seq.fetch_add(1, Ordering::Relaxed);
                    let event_id = match EventId::new(format!("ev-cas-{}-{n:x}", self.boot_tag)) {
                        Ok(id) => id,
                        Err(err) => {
                            return AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                                effect_id: p.effect_id,
                                message: format!("invalid event id: {err}"),
                            });
                        }
                    };
                    let now_ms = Self::now_unix_ms();
                    if let Err(err) = self
                        .storage
                        .put_artifact(ArtifactRecord {
                            project_id: self.project_id.clone(),
                            artifact_hash: hash.clone(),
                            artifact_kind: "atomic-effect-cas-blob".to_owned(),
                            producer_session_id: self.session_id.clone(),
                            producer_agent_id: self.agent_id.clone(),
                            producer_event_id: event_id.clone(),
                            media_type: "application/octet-stream".to_owned(),
                            size_bytes: p.bytes.len() as u64,
                            body: p.bytes,
                            created_at_unix_ms: now_ms,
                        })
                        .await
                    {
                        return AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                            effect_id: p.effect_id,
                            message: format!("cas put_artifact failed: {err}"),
                        });
                    }
                    let expected = match self
                        .storage
                        .get_artifact_pointer(&self.project_id, &ptr_key)
                        .await
                    {
                        Ok(opt) => opt.map(|ptr| ptr.target_artifact_hash),
                        Err(err) => {
                            return AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                                effect_id: p.effect_id,
                                message: format!("cas pointer lookup failed: {err}"),
                            });
                        }
                    };
                    if let Err(err) = self
                        .storage
                        .compare_and_swap_artifact_pointer(ArtifactPointerUpdate {
                            project_id: self.project_id.clone(),
                            pointer_key: ptr_key,
                            expected_artifact_hash: expected,
                            new_artifact_hash: hash,
                            updated_by_session_id: self.session_id.clone(),
                            updated_by_agent_id: self.agent_id.clone(),
                            updated_by_event_id: event_id,
                            updated_at_unix_ms: now_ms,
                        })
                        .await
                    {
                        return AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                            effect_id: p.effect_id,
                            message: format!("cas pointer update failed: {err}"),
                        });
                    }
                    AtomicEffectReceipt::CasStored(p.effect_id)
                }

                AtomicEffect::LlmComplete(p) => match self.gateway.complete(p.request).await {
                    Ok(resp) => {
                        let content = resp
                            .choices
                            .into_iter()
                            .next()
                            .map(|choice| choice.message.content)
                            .unwrap_or_default();
                        AtomicEffectReceipt::LlmCompleted(LlmCompletedReceiptPayload {
                            effect_id: p.effect_id,
                            content,
                        })
                    }
                    Err(err) => AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                        effect_id: p.effect_id,
                        message: format!("model gateway error: {err}"),
                    }),
                },

                AtomicEffect::SandboxRun(p) => self.sandbox.run_sandbox(p).await,
            }
        })
    }
}

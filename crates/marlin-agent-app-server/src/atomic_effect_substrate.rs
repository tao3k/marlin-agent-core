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

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
#[derive(Clone, Copy, Debug)]
pub struct BoundedTokioSandboxRunner {
    timeout: Duration,
    max_output_bytes: usize,
}

impl BoundedTokioSandboxRunner {
    /// Creates a bounded command runner enforcing `timeout` and `max_output_bytes`.
    pub fn new(timeout: Duration, max_output_bytes: usize) -> Self {
        Self {
            timeout,
            max_output_bytes: max_output_bytes.max(1),
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
        Box::pin(async move {
            let mut cmd = tokio::process::Command::new(&payload.program);
            cmd.args(&payload.args).kill_on_drop(true);
            if let Some(cwd) = &payload.cwd {
                cmd.current_dir(cwd);
            }
            match tokio::time::timeout(timeout, cmd.output()).await {
                Ok(Ok(output)) => {
                    let mut stdout = output.stdout;
                    let mut stderr = output.stderr;
                    stdout.truncate(max_bytes);
                    stderr.truncate(max_bytes);
                    AtomicEffectReceipt::SandboxExited(SandboxExitedReceiptPayload {
                        effect_id: payload.effect_id,
                        exit_code: output.status.code().unwrap_or(-1),
                        stdout,
                        stderr,
                    })
                }
                Ok(Err(err)) => AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                    effect_id: payload.effect_id,
                    message: format!("sandbox spawn failed: {err}"),
                }),
                Err(_) => AtomicEffectReceipt::EffectError(EffectErrorReceiptPayload {
                    effect_id: payload.effect_id,
                    message: format!("sandbox command timed out after {}ms", timeout.as_millis()),
                }),
            }
        })
    }
}

/// Thin 4-primitive leaf executor composing `AgentStorage` CAS + `ModelGateway` + `AtomicSandboxRunner`.
pub struct CompositeAtomicEffectExecutor<S, G, R = DenySandboxRunner> {
    storage: S,
    gateway: G,
    sandbox: R,
    project_id: ProjectId,
    session_id: SessionId,
    agent_id: AgentId,
    seq: AtomicU64,
}

impl<S, G> CompositeAtomicEffectExecutor<S, G, DenySandboxRunner> {
    /// Constructs a leaf executor with fail-closed `DenySandboxRunner` by default.
    pub fn new(
        storage: S,
        gateway: G,
        project_id: ProjectId,
        session_id: SessionId,
        agent_id: AgentId,
    ) -> Self {
        Self {
            storage,
            gateway,
            sandbox: DenySandboxRunner,
            project_id,
            session_id,
            agent_id,
            seq: AtomicU64::new(1),
        }
    }
}

impl<S, G, R> CompositeAtomicEffectExecutor<S, G, R> {
    /// Mounts an explicit `AtomicSandboxRunner` onto this leaf executor.
    pub fn with_sandbox<R2>(self, sandbox: R2) -> CompositeAtomicEffectExecutor<S, G, R2> {
        CompositeAtomicEffectExecutor {
            storage: self.storage,
            gateway: self.gateway,
            sandbox,
            project_id: self.project_id,
            session_id: self.session_id,
            agent_id: self.agent_id,
            seq: self.seq,
        }
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
                    let event_id = match EventId::new(format!("ev-atomic-cas-{n}")) {
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use marlin_agent_protocol::{
        ModelEndpoint, ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
        ModelGatewayFuture, ModelGatewayRequest, ModelGatewayResult, assistant_gateway_message,
        user_gateway_message,
    };
    use marlin_agent_runtime::{
        AtomicEffect, AtomicEffectReceipt, CasGetEffectPayload, CasPutEffectPayload,
        LlmCompleteEffectPayload, SandboxRunEffectPayload, run_atomic_effect_batch,
    };
    use marlin_agent_storage::{
        AgentId, ProjectId, SessionId, TursoAgentStorage, TursoAgentStorageConfig,
        TursoBatchTransactionMode, TursoOptimizationProfile,
    };

    use super::{BoundedTokioSandboxRunner, CompositeAtomicEffectExecutor};

    struct EchoModelGateway;

    impl ModelGateway for EchoModelGateway {
        fn complete(
            &self,
            request: ModelGatewayRequest,
        ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
            let prompt = request
                .messages()
                .last()
                .map(|m| m.content.clone())
                .unwrap_or_default();
            Box::pin(async move {
                Ok(ModelGatewayCompletionResponse::new(
                    "echo-resp-1",
                    "gpt-marlin-echo",
                    vec![ModelGatewayCompletionChoice::new(
                        0,
                        assistant_gateway_message(format!("echo:{prompt}")),
                        None,
                    )],
                ))
            })
        }
    }

    #[tokio::test]
    async fn composite_executor_runs_all_four_primitives_on_real_turso_wal_and_survives_restart() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = TursoAgentStorageConfig {
            path: dir.path().join("atomic-substrate.db"),
            optimization_profile: TursoOptimizationProfile::AsyncIoWithMvcc,
            batch_transaction_mode: TursoBatchTransactionMode::Concurrent,
        };
        let project = ProjectId::new("proj-s11").unwrap();
        let session = SessionId::new("sess-s11").unwrap();
        let agent = AgentId::new("agent-s11").unwrap();

        // Boot #1 with default fail-closed DenySandboxRunner
        let storage1 = TursoAgentStorage::open_local(cfg.clone())
            .await
            .expect("open turso #1");
        let exec1 = Arc::new(CompositeAtomicEffectExecutor::new(
            storage1,
            EchoModelGateway,
            project.clone(),
            session.clone(),
            agent.clone(),
        ));

        let batch1 = vec![
            AtomicEffect::CasGet(CasGetEffectPayload {
                effect_id: "e-miss".to_owned(),
                key: "k:s11:wire".to_owned(),
            }),
            AtomicEffect::LlmComplete(LlmCompleteEffectPayload {
                effect_id: "e-llm".to_owned(),
                request: ModelGatewayRequest::new(
                    ModelEndpoint::new("openai", "gpt-marlin-echo"),
                    vec![user_gateway_message("ping-s11")],
                ),
            }),
            AtomicEffect::SandboxRun(SandboxRunEffectPayload {
                effect_id: "e-sb-denied".to_owned(),
                program: "/bin/echo".to_owned(),
                args: vec!["blocked".to_owned()],
                cwd: None,
            }),
        ];
        let rcpts1 = run_atomic_effect_batch(Arc::clone(&exec1), batch1).await;
        assert!(matches!(rcpts1[0], AtomicEffectReceipt::CasMiss(ref id) if id == "e-miss"));
        assert!(matches!(
            rcpts1[1],
            AtomicEffectReceipt::LlmCompleted(ref p) if p.effect_id == "e-llm" && p.content == "echo:ping-s11"
        ));
        assert!(matches!(
            rcpts1[2],
            AtomicEffectReceipt::EffectError(ref p) if p.effect_id == "e-sb-denied"
        ));

        // Store bytes via CasPut on real Turso WAL, then drop Boot #1 completely!
        let put_rcpts = run_atomic_effect_batch(
            Arc::clone(&exec1),
            vec![AtomicEffect::CasPut(CasPutEffectPayload {
                effect_id: "e-put".to_owned(),
                key: "k:s11:wire".to_owned(),
                bytes: b"(fleet-wire-v1 (round . 11))".to_vec(),
            })],
        )
        .await;
        assert!(
            matches!(put_rcpts.as_slice(), [AtomicEffectReceipt::CasStored(id)] if id == "e-put")
        );
        drop(exec1);

        // Boot #2: Reopen Turso WAL + mount BoundedTokioSandboxRunner
        let storage2 = TursoAgentStorage::open_local(cfg)
            .await
            .expect("open turso #2");
        let exec2 = Arc::new(
            CompositeAtomicEffectExecutor::new(storage2, EchoModelGateway, project, session, agent)
                .with_sandbox(BoundedTokioSandboxRunner::new(Duration::from_secs(5), 4096)),
        );
        let batch2 = vec![
            AtomicEffect::CasGet(CasGetEffectPayload {
                effect_id: "e-hit".to_owned(),
                key: "k:s11:wire".to_owned(),
            }),
            AtomicEffect::SandboxRun(SandboxRunEffectPayload {
                effect_id: "e-sb-ok".to_owned(),
                program: "/bin/echo".to_owned(),
                args: vec!["hello-atomic-sandbox".to_owned()],
                cwd: None,
            }),
        ];
        let rcpts2 = run_atomic_effect_batch(exec2, batch2).await;
        assert!(matches!(
            rcpts2[0],
            AtomicEffectReceipt::CasHit(ref p)
                if p.effect_id == "e-hit" && p.bytes == b"(fleet-wire-v1 (round . 11))"
        ));
        assert!(matches!(
            rcpts2[1],
            AtomicEffectReceipt::SandboxExited(ref p)
                if p.effect_id == "e-sb-ok"
                    && p.exit_code == 0
                    && String::from_utf8_lossy(&p.stdout).trim() == "hello-atomic-sandbox"
        ));
    }
}

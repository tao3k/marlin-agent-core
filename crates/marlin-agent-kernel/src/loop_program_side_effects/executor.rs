// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime executor that replays `LoopProgram` projections into owned side effects.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use marlin_agent_runtime::{RuntimeContext, observability};
use tracing::Instrument;

use crate::{
    DerivationId, ExecutionCircuitBreaker, GovernanceAdmissionGate, GovernedSubjectName,
    LoopProgramExecutionReceipt, LoopProgramRuntimeHandoffExecutionReceipt, ToolIfcContext,
    spawn_loop_program_tool_process,
};

use super::{
    LoopProgramExecutionReplayBundleReceipt, LoopProgramFileSandbox, LoopProgramFileWriteReceipt,
    LoopProgramFileWriteRequest, LoopProgramFileWriteResolver,
    LoopProgramFileWriteSideEffectReceipt, LoopProgramRuntimeReplayBundleReceipt,
    LoopProgramRuntimeSideEffectReceipt, LoopProgramToolProcessResolver,
    LoopProgramToolProcessSideEffectReceipt, StaticLoopProgramFileWriteResolver,
    StaticLoopProgramToolProcessResolver, read_existing_digest, stable_bytes_digest,
};

#[derive(Clone)]
struct GovernanceSideEffectBinding {
    gate: Arc<GovernanceAdmissionGate>,
    actor: GovernedSubjectName,
    derivation_id: Option<DerivationId>,
}

/// Async side-effect executor for handoff projections that require runtime ownership.
#[derive(Clone)]
pub struct LoopProgramRuntimeSideEffectExecutor {
    tool_process_resolver: Arc<dyn LoopProgramToolProcessResolver>,
    file_write_resolver: Arc<dyn LoopProgramFileWriteResolver>,
    file_sandbox: LoopProgramFileSandbox,
    execution_breaker: Option<Arc<ExecutionCircuitBreaker>>,
    governance_gate: Option<GovernanceSideEffectBinding>,
    ifc_context: ToolIfcContext,
    started_at_ms: u64,
    observed_at_ms: u64,
}

impl LoopProgramRuntimeSideEffectExecutor {
    pub fn new<R>(tool_process_resolver: R) -> Self
    where
        R: LoopProgramToolProcessResolver,
    {
        Self {
            tool_process_resolver: Arc::new(tool_process_resolver),
            file_write_resolver: Arc::new(StaticLoopProgramFileWriteResolver::default()),
            file_sandbox: LoopProgramFileSandbox::deny_all(),
            execution_breaker: None,
            governance_gate: None,
            ifc_context: ToolIfcContext::clean_trusted(),
            started_at_ms: 0,
            observed_at_ms: 0,
        }
    }

    pub fn with_execution_breaker(
        mut self,
        execution_breaker: Arc<ExecutionCircuitBreaker>,
    ) -> Self {
        self.execution_breaker = Some(execution_breaker);
        self
    }

    /// Attaches a live Scheme-synchronized `GovernanceAdmissionGate` so every
    /// OS process spawn and disk file write enforces ASCENT transitive fencing, Bi-Temporal
    /// Truth Maintenance causal retraction, synthesized Cedar `forbid` rules, and the
    /// underlying `ExecutionCircuitBreaker`.
    pub fn with_governance_gate(
        mut self,
        gate: Arc<GovernanceAdmissionGate>,
        actor: GovernedSubjectName,
        derivation_id: Option<DerivationId>,
    ) -> Self {
        self.governance_gate = Some(GovernanceSideEffectBinding {
            gate,
            actor,
            derivation_id,
        });
        self
    }

    pub fn with_ifc_context(mut self, ifc_context: ToolIfcContext) -> Self {
        self.ifc_context = ifc_context;
        self
    }

    pub fn with_file_write_resolver<R>(mut self, file_write_resolver: R) -> Self
    where
        R: LoopProgramFileWriteResolver,
    {
        self.file_write_resolver = Arc::new(file_write_resolver);
        self
    }

    pub fn with_file_sandbox(mut self, file_sandbox: LoopProgramFileSandbox) -> Self {
        self.file_sandbox = file_sandbox;
        self
    }

    pub fn with_started_at_ms(mut self, started_at_ms: u64) -> Self {
        self.started_at_ms = started_at_ms;
        self
    }

    pub fn with_observed_at_ms(mut self, observed_at_ms: u64) -> Self {
        self.observed_at_ms = observed_at_ms;
        self
    }

    pub async fn execute(
        &self,
        context: &RuntimeContext,
        execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> LoopProgramRuntimeSideEffectReceipt {
        let mut tool_processes = Vec::with_capacity(execution.tool_process_projections.len());
        let mut file_writes = Vec::new();
        for projection in execution.tool_process_projections.iter().cloned() {
            let tool_process_request = self.tool_process_resolver.resolve(&projection);
            let file_write_request = self.file_write_resolver.resolve(&projection);

            if tool_process_request.is_none() && file_write_request.is_none() {
                tool_processes.push(LoopProgramToolProcessSideEffectReceipt::skipped(projection));
                continue;
            }

            if let Some(request) = tool_process_request {
                let blocked_diag = if let Some(binding) = &self.governance_gate {
                    let tool_name = GovernedSubjectName::new(format!(
                        "tool_process:{}",
                        projection.command.command_kind
                    ))
                    .unwrap_or_else(|_| GovernedSubjectName::new("tool_process").unwrap());
                    let resource_name =
                        GovernedSubjectName::new(projection.command.command_kind.as_str())
                            .unwrap_or_else(|_| GovernedSubjectName::new("workspace").unwrap());
                    binding
                        .gate
                        .authorize_side_effect_on_resource(
                            binding.derivation_id.as_ref(),
                            &binding.actor,
                            &tool_name,
                            &resource_name,
                            &self.ifc_context,
                        )
                        .err()
                        .map(|rejection| {
                            format!("loop_program.tool_process.governance_blocked:{rejection}")
                        })
                } else if let Some(breaker) = &self.execution_breaker {
                    breaker
                        .authorize_side_effect(
                            format!("tool_process:{}", projection.command.command_kind),
                            &self.ifc_context,
                        )
                        .err()
                        .map(|rejection| {
                            format!("loop_program.tool_process.breaker_blocked:{rejection}")
                        })
                } else {
                    None
                };

                if let Some(diag) = blocked_diag {
                    tool_processes.push(LoopProgramToolProcessSideEffectReceipt::blocked(
                        projection.clone(),
                        diag,
                    ));
                } else {
                    let request = request
                        .with_started_at_ms(self.started_at_ms)
                        .with_observed_at_ms(self.observed_at_ms);
                    match spawn_loop_program_tool_process(context, request)
                        .instrument(observability::runtime_tool_span())
                        .await
                    {
                        Ok(receipt) => {
                            tool_processes
                                .push(LoopProgramToolProcessSideEffectReceipt::completed(receipt));
                        }
                        Err(error) => {
                            tool_processes.push(LoopProgramToolProcessSideEffectReceipt::failed(
                                projection.clone(),
                                error,
                            ));
                        }
                    }
                }
            }

            if let Some(request) = file_write_request {
                let blocked_diag = if let Some(binding) = &self.governance_gate {
                    let rel_display = request.relative_path.display().to_string();
                    let tool_name = GovernedSubjectName::new(format!("file_write:{rel_display}"))
                        .unwrap_or_else(|_| GovernedSubjectName::new("file_write").unwrap());
                    let resource_name = GovernedSubjectName::new(rel_display)
                        .unwrap_or_else(|_| GovernedSubjectName::new("workspace").unwrap());
                    binding
                        .gate
                        .authorize_side_effect_on_resource(
                            binding.derivation_id.as_ref(),
                            &binding.actor,
                            &tool_name,
                            &resource_name,
                            &self.ifc_context,
                        )
                        .err()
                        .map(|rejection| {
                            format!("loop_program.file_write.governance_blocked:{rejection}")
                        })
                } else if let Some(breaker) = &self.execution_breaker {
                    breaker
                        .authorize_side_effect(
                            format!("file_write:{}", request.relative_path.display()),
                            &self.ifc_context,
                        )
                        .err()
                        .map(|rejection| {
                            format!("loop_program.file_write.breaker_blocked:{rejection}")
                        })
                } else {
                    None
                };

                if let Some(diag) = blocked_diag {
                    file_writes.push(LoopProgramFileWriteSideEffectReceipt::denied(request, diag));
                } else {
                    file_writes.push(self.execute_file_write(request));
                }
            }
        }

        LoopProgramRuntimeSideEffectReceipt::new(
            execution.program_id.clone(),
            tool_processes,
            file_writes,
        )
    }

    pub async fn execute_loop_execution(
        &self,
        context: &RuntimeContext,
        execution: &LoopProgramExecutionReceipt,
    ) -> LoopProgramExecutionReplayBundleReceipt {
        let mut step_replay_bundles = Vec::new();
        for step in execution.steps.iter() {
            if !has_projected_runtime_side_effects(&step.runtime_handoff_execution) {
                continue;
            }
            let side_effects = self.execute(context, &step.runtime_handoff_execution).await;
            step_replay_bundles.push(
                LoopProgramRuntimeReplayBundleReceipt::from_runtime_receipts(
                    step.runtime_handoff_execution.clone(),
                    side_effects,
                ),
            );
        }

        LoopProgramExecutionReplayBundleReceipt::new(
            execution.program_id.clone(),
            execution.status.clone(),
            step_replay_bundles,
        )
    }

    fn execute_file_write(
        &self,
        request: LoopProgramFileWriteRequest,
    ) -> LoopProgramFileWriteSideEffectReceipt {
        let runtime_tool_span = observability::runtime_tool_span();
        let _runtime_tool_span_guard = runtime_tool_span.enter();
        let (path, normalized_relative_path) =
            match self.file_sandbox.resolve(&request.relative_path) {
                Ok(path) => path,
                Err(diagnostic) => {
                    return LoopProgramFileWriteSideEffectReceipt::denied(request, diagnostic);
                }
            };

        let before_hash = match read_existing_digest(&path) {
            Ok(before_hash) => before_hash,
            Err(error) => {
                return LoopProgramFileWriteSideEffectReceipt::failed(request, error);
            }
        };

        if let Some(parent) = path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            return LoopProgramFileWriteSideEffectReceipt::failed(request, error);
        }

        if let Err(diagnostic) = self.file_sandbox.resolve(&request.relative_path) {
            return LoopProgramFileWriteSideEffectReceipt::denied(request, diagnostic);
        }

        if let Err(error) = write_file_atomically(&path, request.contents.as_ref()) {
            return LoopProgramFileWriteSideEffectReceipt::failed(request, error);
        }

        let after_hash = stable_bytes_digest(request.contents.as_ref());
        let bytes_written = request.contents.len();
        LoopProgramFileWriteSideEffectReceipt::completed(LoopProgramFileWriteReceipt {
            projection: request.projection,
            relative_path: normalized_relative_path,
            path,
            before_hash,
            after_hash,
            bytes_written,
        })
    }
}

impl Default for LoopProgramRuntimeSideEffectExecutor {
    fn default() -> Self {
        Self::new(StaticLoopProgramToolProcessResolver::default())
    }
}

fn has_projected_runtime_side_effects(
    execution: &LoopProgramRuntimeHandoffExecutionReceipt,
) -> bool {
    execution.agent_flow_receipt.is_some()
        || !execution.tool_process_projections.is_empty()
        || !execution.memory_projections.is_empty()
}

fn write_file_atomically(path: &Path, contents: &[u8]) -> io::Result<()> {
    static NEXT_TEMP_SEQ: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned());
    let seq = NEXT_TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp_path = parent.join(format!(
        ".{file_name}.tmp.{}.{seq}",
        std::process::id()
    ));

    let write_res = (|| -> io::Result<()> {
        let mut tmp_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        tmp_file.write_all(contents)?;
        tmp_file.sync_all()?;
        drop(tmp_file);
        fs::rename(&tmp_path, path)?;
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
        Ok(())
    })();

    if write_res.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    write_res
}

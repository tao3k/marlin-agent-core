//! Runtime executor that replays `LoopProgram` projections into owned side effects.

use std::{fs, sync::Arc};

use marlin_agent_runtime::RuntimeContext;

use crate::{
    LoopProgramExecutionReceipt, LoopProgramRuntimeHandoffExecutionReceipt,
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

/// Async side-effect executor for handoff projections that require runtime ownership.
#[derive(Clone)]
pub struct LoopProgramRuntimeSideEffectExecutor {
    tool_process_resolver: Arc<dyn LoopProgramToolProcessResolver>,
    file_write_resolver: Arc<dyn LoopProgramFileWriteResolver>,
    file_sandbox: LoopProgramFileSandbox,
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
            started_at_ms: 0,
            observed_at_ms: 0,
        }
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
                let request = request
                    .with_started_at_ms(self.started_at_ms)
                    .with_observed_at_ms(self.observed_at_ms);
                match spawn_loop_program_tool_process(context, request).await {
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

            if let Some(request) = file_write_request {
                file_writes.push(self.execute_file_write(request));
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

        if let Err(error) = fs::write(&path, request.contents.as_ref()) {
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

//! Process-command runtime used by debug graph executors.

use std::{collections::BTreeMap, path::PathBuf, process::Command};

use crate::{
    ExecutorName, GraphNodeExecutionReceipt, GraphNodeExecutionStatus, GraphNodeInvocation, NodeId,
    RuntimeContext, RuntimeFuture, ToolRuntime,
};

/// Explicit argv process binding for debug graph nodes.
#[derive(Clone, Debug)]
pub(super) struct ProcessCommandBinding {
    command: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env: BTreeMap<String, String>,
}

impl ProcessCommandBinding {
    pub(super) fn new(
        command: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
        env: BTreeMap<String, String>,
    ) -> Self {
        Self {
            command,
            args,
            cwd,
            env,
        }
    }

    /// Process command boundary: debug smoke owns argv/cwd/env shaping and typed receipt conversion.
    fn run(self, invocation: GraphNodeInvocation) -> ProcessCommandOutput {
        let mut command = Command::new(&self.command);
        command.args(&self.args);
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }
        command.envs(&self.env);

        match command.output() {
            Ok(output) => ProcessCommandOutput {
                invocation,
                status_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                spawn_error: None,
            },
            Err(error) => ProcessCommandOutput {
                invocation,
                status_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some(error.to_string()),
            },
        }
    }
}

/// Tool runtime adapter for `process-command` bindings.
#[derive(Clone, Debug)]
pub(super) struct DebugProcessCommandRuntime {
    binding: ProcessCommandBinding,
}

impl DebugProcessCommandRuntime {
    pub(super) fn new(binding: ProcessCommandBinding) -> Self {
        Self { binding }
    }
}

impl ToolRuntime for DebugProcessCommandRuntime {
    type Invocation = GraphNodeInvocation;
    type Output = ProcessCommandOutput;

    fn run_tool(
        &self,
        invocation: GraphNodeInvocation,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let binding = self.binding.clone();
        Box::pin(async move { binding.run(invocation) })
    }
}

#[derive(Clone, Debug)]
pub(super) struct ProcessCommandOutput {
    invocation: GraphNodeInvocation,
    status_code: Option<i32>,
    stdout: String,
    stderr: String,
    spawn_error: Option<String>,
}

pub(super) fn process_command_receipt(
    output: ProcessCommandOutput,
    node_id: NodeId,
    executor: ExecutorName,
) -> GraphNodeExecutionReceipt {
    let mut diagnostics = Vec::new();
    let spawn_failed = output.spawn_error.is_some();
    if let Some(error) = output.spawn_error {
        diagnostics.push(format!("process-command.spawn_failed:{error}"));
    }
    let status = match output.status_code {
        Some(code) => {
            diagnostics.push(format!("process-command.exit_status:{code}"));
            if code == 0 && !spawn_failed {
                GraphNodeExecutionStatus::Completed
            } else {
                GraphNodeExecutionStatus::Failed
            }
        }
        None => {
            diagnostics.push("process-command.exit_status:unknown".to_owned());
            GraphNodeExecutionStatus::Failed
        }
    };
    push_process_stream_diagnostic("stdout", &output.stdout, &mut diagnostics);
    push_process_stream_diagnostic("stderr", &output.stderr, &mut diagnostics);
    diagnostics.push(format!(
        "process-command.node:{}",
        output.invocation.node_id.as_str()
    ));
    GraphNodeExecutionReceipt {
        status,
        node_id,
        executor,
        diagnostics,
    }
}

fn push_process_stream_diagnostic(name: &str, stream: &str, diagnostics: &mut Vec<String>) {
    let stream = stream.trim();
    if !stream.is_empty() {
        diagnostics.push(format!("process-command.{name}:{stream}"));
    }
}

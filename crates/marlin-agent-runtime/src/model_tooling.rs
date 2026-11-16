// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed admission of Scheme tool declarations and reusable host capabilities.

use std::{path::PathBuf, time::Duration};

use marlin_agent_protocol::{
    BoundedProcessExecutor, BoundedProcessRequest, ModelToolCall, ModelToolDefinition,
    ModelToolName, ProcessEnvironmentPolicy, ProcessExecutionError,
};

use crate::{
    ManagedProcessExecutor, ModelToolOutcome, RuntimeContext, RuntimeFuture, RuntimeOperationId,
    ToolRuntime, process::RuntimeProcessKind,
};

/// A host implementation family. Adding another declaration of an existing
/// capability does not add a Rust dispatcher branch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelToolCapability {
    Shell,
}

/// The Scheme policy's required authorization. A declaration cannot grant it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelToolGovernance {
    Ask,
}

/// A bounded, admitted snapshot of a selected Scheme declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelToolDeclaration {
    pub(crate) name: ModelToolName,
    description: String,
    capability: ModelToolCapability,
    pub(crate) governance: ModelToolGovernance,
    argument: String,
    timeout: Duration,
    output_limit: usize,
}

/// Named fields from a Scheme shell declaration admitted by the Rust bridge.
pub struct ShellToolDeclarationRequest {
    pub name: ModelToolName,
    pub description: String,
    pub argument: String,
    pub timeout_seconds: u64,
    pub output_limit: usize,
}

impl ModelToolDeclaration {
    pub fn name(&self) -> &ModelToolName {
        &self.name
    }

    pub fn capability(&self) -> ModelToolCapability {
        self.capability
    }

    pub fn governance(&self) -> ModelToolGovernance {
        self.governance
    }
    pub fn shell(request: ShellToolDeclarationRequest) -> Result<Self, &'static str> {
        let ShellToolDeclarationRequest {
            name,
            description,
            argument,
            timeout_seconds,
            output_limit,
        } = request;
        if description.trim().is_empty()
            || description.len() > 256
            || argument.is_empty()
            || argument.len() > 64
            || !argument
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
            || !(1..=120).contains(&timeout_seconds)
            || !(1..=1024 * 1024).contains(&output_limit)
        {
            return Err("invalid shell tool declaration");
        }
        Ok(Self {
            name,
            description,
            capability: ModelToolCapability::Shell,
            governance: ModelToolGovernance::Ask,
            argument,
            timeout: Duration::from_secs(timeout_seconds),
            output_limit,
        })
    }

    pub fn definition(&self) -> ModelToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(self.argument.clone(), serde_json::json!({"type": "string"}));
        ModelToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": [self.argument],
                "additionalProperties": false
            }),
        }
    }

    pub fn admits(&self, call: &ModelToolCall) -> bool {
        call.name == self.name
            && call.arguments.len() == 1
            && call
                .arguments
                .get(&self.argument)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|command| {
                    !command.trim().is_empty()
                        && command.len() <= 8 * 1024
                        && !command.contains('\0')
                })
    }
}

/// One reusable shell capability. It receives only declarations admitted at
/// startup; the caller must first consume a durable execution claim.
pub struct ShellToolHandler {
    workspace: PathBuf,
    declaration: ModelToolDeclaration,
}

impl ShellToolHandler {
    pub fn new(workspace: PathBuf, declaration: ModelToolDeclaration) -> Self {
        Self {
            workspace,
            declaration,
        }
    }
}

impl ToolRuntime for ShellToolHandler {
    type Invocation = ModelToolCall;
    type Output = ModelToolOutcome;

    fn run_tool(
        &self,
        call: ModelToolCall,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let workspace = self.workspace.clone();
        let declaration = self.declaration.clone();
        Box::pin(async move {
            if !declaration.admits(&call) {
                return ModelToolOutcome::Failed("invalid shell tool arguments".into());
            }
            let command = call.arguments[&declaration.argument]
                .as_str()
                .expect("admitted string command");
            let script = format!("exec 2>&1\n{command}");
            let executor = ManagedProcessExecutor::new(
                context,
                RuntimeProcessKind::Tool,
                RuntimeOperationId::new("model.tool.shell"),
            );
            let request = BoundedProcessRequest {
                program: "/bin/bash".into(),
                arguments: vec![
                    "--noprofile".into(),
                    "--norc".into(),
                    "-c".into(),
                    script.into(),
                ],
                working_directory: workspace,
                remove_environment: Vec::new(),
                environment: ProcessEnvironmentPolicy::Clean,
                stdout_limit: declaration.output_limit,
                timeout: declaration.timeout,
            };
            match executor.execute(request).await {
                Ok(output) => ModelToolOutcome::Completed(format!(
                    "exit code: {}\n{}",
                    output
                        .exit_code
                        .map_or_else(|| "signal".into(), |code| code.to_string()),
                    String::from_utf8_lossy(&output.stdout)
                )),
                Err(ProcessExecutionError::OutputLimit) => {
                    ModelToolOutcome::Failed("shell output limit exceeded".into())
                }
                Err(ProcessExecutionError::TimedOut) => {
                    ModelToolOutcome::Failed("shell command timed out".into())
                }
                Err(ProcessExecutionError::Io(error)) => {
                    ModelToolOutcome::Failed(error.to_string())
                }
            }
        })
    }
}

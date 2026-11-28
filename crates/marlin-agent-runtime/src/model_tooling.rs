// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed admission of Scheme tool declarations and reusable host capabilities.

use std::{collections::HashSet, path::PathBuf, time::Duration};

use marlin_agent_protocol::{
    BoundedProcessExecutor, BoundedProcessRequest, ModelToolCall, ModelToolDefinition,
    ModelToolName, ProcessEnvironmentPolicy, ProcessExecutionError,
};

use crate::{
    ManagedProcessExecutor, ModelToolOutcome, RuntimeContext, RuntimeFuture, RuntimeOperationId,
    ToolRuntime, process::RuntimeProcessKind,
};

const MAX_TOOL_PARAMETERS: usize = 16;
const MAX_PARAM_DESCRIPTION_BYTES: usize = 1024;
const MAX_ENUM_VALUES: usize = 32;
const MAX_ENUM_VALUE_BYTES: usize = 128;
const MAX_STRING_ARG_BYTES: usize = 8 * 1024;
const MAX_ARRAY_ARG_ITEMS: usize = 64;
const MAX_ARRAY_ITEM_BYTES: usize = 1024;

/// A host implementation family. Adding another declaration of an existing
/// capability does not add a Rust dispatcher branch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelToolCapability {
    Shell,
    Search,
}

/// The Scheme policy's required authorization. A declaration cannot grant it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelToolGovernance {
    Ask,
}

/// Validated parameter identifier for a declared model tool (1..=64 ASCII
/// lowercase letters, digits, or underscores; starts with an ASCII letter).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ToolParameterName(String);

impl ToolParameterName {
    /// Admit a bounded tool parameter name.
    pub fn new(raw: impl Into<String>) -> Result<Self, &'static str> {
        let name = raw.into();
        let valid = !name.is_empty()
            && name.len() <= 64
            && name.as_bytes()[0].is_ascii_lowercase()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if valid {
            Ok(Self(name))
        } else {
            Err("invalid tool parameter name")
        }
    }

    /// Borrow the validated parameter name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed JSON Schema primitive/collection kind admitted from Scheme tooling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolParameterType {
    String,
    Integer,
    Boolean,
    StringArray,
}

/// Named fields for admitting one typed [`ToolParameterSpec`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolParameterSpecRequest {
    pub name: ToolParameterName,
    pub param_type: ToolParameterType,
    pub description: String,
    pub required: bool,
    pub allowed_values: Vec<String>,
}

/// One admitted typed parameter slot of a [`ModelToolDeclaration`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolParameterSpec {
    name: ToolParameterName,
    param_type: ToolParameterType,
    description: String,
    required: bool,
    allowed_values: Vec<String>,
}

impl ToolParameterSpec {
    /// Admit a bounded typed parameter specification.
    pub fn from_request(request: ToolParameterSpecRequest) -> Result<Self, &'static str> {
        if request.description.len() > MAX_PARAM_DESCRIPTION_BYTES
            || request.allowed_values.len() > MAX_ENUM_VALUES
        {
            return Err("invalid tool parameter specification");
        }
        if !request.allowed_values.is_empty() && request.param_type != ToolParameterType::String {
            return Err("enum values require string parameter type");
        }
        let mut seen = HashSet::with_capacity(request.allowed_values.len());
        for value in &request.allowed_values {
            if value.is_empty()
                || value.len() > MAX_ENUM_VALUE_BYTES
                || value.contains('\0')
                || !seen.insert(value.as_str())
            {
                return Err("invalid tool parameter enum value");
            }
        }
        Ok(Self {
            name: request.name,
            param_type: request.param_type,
            description: request.description,
            required: request.required,
            allowed_values: request.allowed_values,
        })
    }

    /// Construct a required string parameter specification.
    pub fn required_string(
        name: ToolParameterName,
        description: impl Into<String>,
    ) -> Result<Self, &'static str> {
        Self::from_request(ToolParameterSpecRequest {
            name,
            param_type: ToolParameterType::String,
            description: description.into(),
            required: true,
            allowed_values: Vec::new(),
        })
    }

    /// Parameter identifier.
    #[must_use]
    pub fn name(&self) -> &ToolParameterName {
        &self.name
    }

    /// Parameter JSON Schema type.
    #[must_use]
    pub fn param_type(&self) -> ToolParameterType {
        self.param_type
    }

    /// Optional human/LLM parameter description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Whether this parameter is required in every tool call.
    #[must_use]
    pub fn required(&self) -> bool {
        self.required
    }

    /// Optional closed enum of allowed string values.
    #[must_use]
    pub fn allowed_values(&self) -> &[String] {
        &self.allowed_values
    }

    fn json_schema(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        match self.param_type {
            ToolParameterType::String => {
                obj.insert("type".to_owned(), serde_json::Value::String("string".into()));
            }
            ToolParameterType::Integer => {
                obj.insert(
                    "type".to_owned(),
                    serde_json::Value::String("integer".into()),
                );
            }
            ToolParameterType::Boolean => {
                obj.insert(
                    "type".to_owned(),
                    serde_json::Value::String("boolean".into()),
                );
            }
            ToolParameterType::StringArray => {
                obj.insert("type".to_owned(), serde_json::Value::String("array".into()));
                obj.insert(
                    "items".to_owned(),
                    serde_json::json!({ "type": "string" }),
                );
            }
        }
        if !self.description.trim().is_empty() {
            obj.insert(
                "description".to_owned(),
                serde_json::Value::String(self.description.clone()),
            );
        }
        if !self.allowed_values.is_empty() {
            obj.insert(
                "enum".to_owned(),
                serde_json::Value::Array(
                    self.allowed_values
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        serde_json::Value::Object(obj)
    }

    fn admits_value(&self, value: &serde_json::Value) -> bool {
        match self.param_type {
            ToolParameterType::String => value.as_str().is_some_and(|text| {
                !text.trim().is_empty()
                    && text.len() <= MAX_STRING_ARG_BYTES
                    && !text.contains('\0')
                    && (self.allowed_values.is_empty()
                        || self.allowed_values.iter().any(|allowed| allowed == text))
            }),
            ToolParameterType::Integer => value.is_i64(),
            ToolParameterType::Boolean => value.is_boolean(),
            ToolParameterType::StringArray => value.as_array().is_some_and(|items| {
                items.len() <= MAX_ARRAY_ARG_ITEMS
                    && items.iter().all(|item| {
                        item.as_str().is_some_and(|text| {
                            !text.trim().is_empty()
                                && text.len() <= MAX_ARRAY_ITEM_BYTES
                                && !text.contains('\0')
                        })
                    })
            }),
        }
    }
}

/// Named fields for admitting one Scheme tool declaration into the Rust host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDeclarationRequest {
    pub name: ModelToolName,
    pub description: String,
    pub capability: ModelToolCapability,
    pub parameters: Vec<ToolParameterSpec>,
    pub timeout_seconds: u64,
    pub output_limit: usize,
}

/// A bounded, admitted snapshot of a selected Scheme tool declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelToolDeclaration {
    pub(crate) name: ModelToolName,
    description: String,
    capability: ModelToolCapability,
    pub(crate) governance: ModelToolGovernance,
    parameters: Vec<ToolParameterSpec>,
    primary_argument_index: usize,
    timeout: Duration,
    output_limit: usize,
}

impl ModelToolDeclaration {
    /// Admit a typed [`ToolDeclarationRequest`] projected from Scheme tooling.
    pub fn from_request(request: ToolDeclarationRequest) -> Result<Self, &'static str> {
        if request.description.trim().is_empty()
            || request.description.len() > 4096
            || request.parameters.is_empty()
            || request.parameters.len() > MAX_TOOL_PARAMETERS
            || !(1..=120).contains(&request.timeout_seconds)
            || !(1..=1024 * 1024).contains(&request.output_limit)
        {
            return Err("invalid tool declaration");
        }
        let mut seen = HashSet::with_capacity(request.parameters.len());
        for param in &request.parameters {
            if !seen.insert(param.name.as_str()) {
                return Err("duplicate tool parameter name");
            }
        }
        let primary_argument_index = request
            .parameters
            .iter()
            .position(|param| param.required && param.param_type == ToolParameterType::String)
            .ok_or("capability requires a required string primary parameter")?;
        Ok(Self {
            name: request.name,
            description: request.description,
            capability: request.capability,
            governance: ModelToolGovernance::Ask,
            parameters: request.parameters,
            primary_argument_index,
            timeout: Duration::from_secs(request.timeout_seconds),
            output_limit: request.output_limit,
        })
    }

    /// Declared tool name.
    #[must_use]
    pub fn name(&self) -> &ModelToolName {
        &self.name
    }

    /// Host capability family backing this declaration.
    #[must_use]
    pub fn capability(&self) -> ModelToolCapability {
        self.capability
    }

    /// Required runtime governance gate.
    #[must_use]
    pub fn governance(&self) -> ModelToolGovernance {
        self.governance
    }

    /// Primary required string argument name (e.g. `command` for shell, `query` for search).
    #[must_use]
    pub fn argument(&self) -> &str {
        self.parameters[self.primary_argument_index].name.as_str()
    }

    /// Full ordered list of admitted typed parameter specifications.
    #[must_use]
    pub fn parameters(&self) -> &[ToolParameterSpec] {
        &self.parameters
    }

    /// Execution timeout bound.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Output byte ceiling.
    #[must_use]
    pub fn output_limit(&self) -> usize {
        self.output_limit
    }

    /// Synthesize the provider-facing LLM JSON Schema definition from admitted Scheme specs.
    #[must_use]
    pub fn definition(&self) -> ModelToolDefinition {
        let mut properties = serde_json::Map::with_capacity(self.parameters.len());
        let mut required = Vec::new();
        for param in &self.parameters {
            properties.insert(param.name.as_str().to_owned(), param.json_schema());
            if param.required {
                required.push(serde_json::Value::String(param.name.as_str().to_owned()));
            }
        }
        ModelToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false
            }),
        }
    }

    /// Fail-closed argument validation against every admitted parameter contract.
    #[must_use]
    pub fn admits(&self, call: &ModelToolCall) -> bool {
        if call.name != self.name {
            return false;
        }
        let all_keys_known = call
            .arguments
            .keys()
            .all(|key| self.parameters.iter().any(|p| p.name.as_str() == key));
        if !all_keys_known {
            return false;
        }
        self.parameters.iter().all(|param| {
            match call.arguments.get(param.name.as_str()) {
                Some(value) => param.admits_value(value),
                None => !param.required,
            }
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
    /// Construct a workspace-scoped shell tool handler for `declaration`.
    #[must_use]
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
            let command = call.arguments[declaration.argument()]
                .as_str()
                .expect("admitted string command");
            let script = format!("exec 2>&1\n{command}");
            let workspace = if let Some(binding) = context.active_working_copy() {
                binding.working_copy.path.clone()
            } else {
                match tokio::time::timeout(declaration.timeout, context.project_envrc()).await {
                    Ok(Ok(Some(envrc))) if workspace.starts_with(envrc.root()) => {
                        envrc.root().to_path_buf()
                    }
                    Ok(Ok(_)) => workspace,
                    Ok(Err(error)) => return ModelToolOutcome::Failed(error.to_string()),
                    Err(_) => {
                        return ModelToolOutcome::Failed("worktree discovery timed out".into());
                    }
                }
            };
            let executor = ManagedProcessExecutor::new(
                context,
                RuntimeProcessKind::Tool,
                RuntimeOperationId::new("model.tool.shell"),
            )
            .with_project_envrc();
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

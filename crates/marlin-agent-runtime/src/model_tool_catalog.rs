// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime-owned model tool registrations and offer admission.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use marlin_agent_protocol::{
    ModelGatewayCompletionOptions, ModelToolCall, ModelToolCallId, ModelToolDefinition,
    ModelToolName,
};
use marlin_agent_sessions::ConversationToolExecutionClaim;

use crate::{
    ModelToolDeclaration, ModelToolGovernance, RuntimeContext, ShellToolHandler, ToolRuntime,
};
use std::path::PathBuf;

/// Typed result of one handler invocation. The caller must commit it before
/// using it as a provider continuation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelToolOutcome {
    Completed(String),
    Failed(String),
}

/// A registered handler for provider-visible model tool calls.
pub type ModelToolHandler = dyn ToolRuntime<Invocation = ModelToolCall, Output = ModelToolOutcome>;

#[derive(Clone)]
struct Entry {
    definition: ModelToolDefinition,
    handler: Arc<ModelToolHandler>,
    declaration: ModelToolDeclaration,
}

/// Immutable-after-setup tool directory. The model only sees definitions
/// backed by a live handler in this catalog.
#[derive(Default)]
pub struct ModelToolCatalog {
    entries: Vec<Entry>,
}

/// The exact handler-backed tool set offered for one model request. Retain
/// this value until the provider's terminal response has been admitted.
pub struct ModelToolOffer {
    entries: Vec<Entry>,
    index: HashMap<ModelToolName, usize>,
}

/// Rejection at the runtime offer boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelToolCatalogError {
    DuplicateName,
    InvalidCallSet,
    UnregisteredTool,
}

impl ModelToolCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a selected Scheme declaration to an existing Rust handler.
    /// Provider schemas can only be derived from the admitted declaration.
    pub fn register(
        &mut self,
        declaration: ModelToolDeclaration,
        handler: Arc<ModelToolHandler>,
    ) -> Result<(), ModelToolCatalogError> {
        let definition = declaration.definition();
        if self.contains(&definition.name) {
            return Err(ModelToolCatalogError::DuplicateName);
        }
        self.entries.push(Entry {
            definition,
            handler,
            declaration,
        });
        Ok(())
    }

    /// Register a selected Scheme declaration against an existing host
    /// capability. This is independent of any particular tool name.
    pub fn register_declaration(
        &mut self,
        declaration: ModelToolDeclaration,
        workspace: PathBuf,
    ) -> Result<(), ModelToolCatalogError> {
        let handler: Arc<ModelToolHandler> =
            Arc::new(ShellToolHandler::new(workspace, declaration.clone()));
        self.register(declaration, handler)
    }

    /// Freeze the handlers and definitions for one provider request.
    pub fn offer(&self) -> ModelToolOffer {
        ModelToolOffer {
            entries: self.entries.clone(),
            index: self
                .entries
                .iter()
                .enumerate()
                .map(|(position, entry)| (entry.definition.name.clone(), position))
                .collect(),
        }
    }

    fn contains(&self, name: &ModelToolName) -> bool {
        self.entries
            .iter()
            .any(|entry| &entry.definition.name == name)
    }
}

impl ModelToolOffer {
    /// Consume the journal's one-shot claim before any handler runs. The caller
    /// must commit outcomes before sending a provider continuation.
    pub async fn execute_claim(
        &self,
        claim: ConversationToolExecutionClaim,
        context: RuntimeContext,
    ) -> Result<Vec<(ModelToolCallId, ModelToolOutcome)>, ModelToolCatalogError> {
        let calls = claim.into_calls();
        self.admit_calls(&calls)?;
        let mut outcomes = Vec::with_capacity(calls.len());
        for call in calls {
            let position = self
                .index
                .get(&call.name)
                .ok_or(ModelToolCatalogError::UnregisteredTool)?;
            let id = call.id.clone();
            outcomes.push((
                id,
                self.entries[*position]
                    .handler
                    .run_tool(call, context.child_context())
                    .await,
            ));
        }
        Ok(outcomes)
    }
    /// Return Scheme's admitted governance for this offered tool. The value
    /// still requires a durable user decision and one-shot execution claim.
    pub fn governance(&self, name: &ModelToolName) -> Option<ModelToolGovernance> {
        self.index
            .get(name)
            .map(|position| self.entries[*position].declaration.governance())
    }
    /// Stable registration order for the provider request.
    pub fn definitions(&self) -> Vec<ModelToolDefinition> {
        self.entries
            .iter()
            .map(|entry| entry.definition.clone())
            .collect()
    }

    /// Options to attach to the model request. An empty offer leaves the
    /// provider tool surface absent.
    pub fn completion_options(&self) -> ModelGatewayCompletionOptions {
        ModelGatewayCompletionOptions {
            tools: (!self.entries.is_empty()).then(|| self.definitions()),
            ..ModelGatewayCompletionOptions::default()
        }
    }

    /// Check that a request carries this exact offer without cloning schemas.
    pub fn matches_definitions(&self, tools: Option<&[ModelToolDefinition]>) -> bool {
        match tools {
            None => self.entries.is_empty(),
            Some(tools) => {
                tools.len() == self.entries.len()
                    && tools
                        .iter()
                        .zip(&self.entries)
                        .all(|(tool, entry)| tool == &entry.definition)
            }
        }
    }

    /// Validate the whole provider call batch before any review or effect.
    pub fn admit_calls(&self, calls: &[ModelToolCall]) -> Result<(), ModelToolCatalogError> {
        if calls.is_empty() || calls.len() > 32 {
            return Err(ModelToolCatalogError::InvalidCallSet);
        }
        if calls.len() == 1 {
            return self.admit_call(&calls[0]);
        }
        let mut ids = HashSet::with_capacity(calls.len());
        for call in calls {
            if !ids.insert(&call.id) {
                return Err(ModelToolCatalogError::InvalidCallSet);
            }
            self.admit_call(call)?;
        }
        Ok(())
    }

    fn admit_call(&self, call: &ModelToolCall) -> Result<(), ModelToolCatalogError> {
        let position = self
            .index
            .get(&call.name)
            .ok_or(ModelToolCatalogError::UnregisteredTool)?;
        let entry = &self.entries[*position];
        if !entry.declaration.admits(call) {
            return Err(ModelToolCatalogError::InvalidCallSet);
        }
        Ok(())
    }
}

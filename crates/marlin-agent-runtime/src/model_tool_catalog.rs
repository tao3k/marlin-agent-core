// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime-owned model tool registrations and offer admission.

use std::{collections::HashSet, sync::Arc};

use marlin_agent_protocol::{
    ModelGatewayCompletionOptions, ModelToolCall, ModelToolDefinition, ModelToolName,
};

use crate::ToolRuntime;

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
}

/// Rejection at the runtime offer boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelToolCatalogError {
    DuplicateName,
    InvalidDefinition,
    InvalidCallSet,
    UnregisteredTool,
}

impl ModelToolCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register exactly one schema and handler before offering tools.
    pub fn register(
        &mut self,
        definition: ModelToolDefinition,
        handler: Arc<ModelToolHandler>,
    ) -> Result<(), ModelToolCatalogError> {
        if definition.description.trim().is_empty() || !definition.parameters.is_object() {
            return Err(ModelToolCatalogError::InvalidDefinition);
        }
        if self.contains(&definition.name) {
            return Err(ModelToolCatalogError::DuplicateName);
        }
        self.entries.push(Entry {
            definition,
            handler,
        });
        Ok(())
    }

    /// Freeze the handlers and definitions for one provider request.
    pub fn offer(&self) -> ModelToolOffer {
        ModelToolOffer {
            entries: self.entries.clone(),
        }
    }

    fn contains(&self, name: &ModelToolName) -> bool {
        self.entries
            .iter()
            .any(|entry| &entry.definition.name == name)
    }
}

impl ModelToolOffer {
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
            return self
                .handler(&calls[0].name)
                .map(|_| ())
                .ok_or(ModelToolCatalogError::UnregisteredTool);
        }
        let mut ids = HashSet::with_capacity(calls.len());
        for call in calls {
            if !ids.insert(&call.id) {
                return Err(ModelToolCatalogError::InvalidCallSet);
            }
            if self.handler(&call.name).is_none() {
                return Err(ModelToolCatalogError::UnregisteredTool);
            }
        }
        Ok(())
    }

    /// A handler is only accessible within the runtime execution boundary;
    /// the UI and provider response cannot invoke it from a name alone.
    fn handler(&self, name: &ModelToolName) -> Option<&Arc<ModelToolHandler>> {
        self.entries
            .iter()
            .find(|entry| &entry.definition.name == name)
            .map(|entry| &entry.handler)
    }
}

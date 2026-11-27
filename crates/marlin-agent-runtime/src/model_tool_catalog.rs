// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime-owned model tool registrations and offer admission.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Write as _,
    sync::Arc,
    time::Instant,
};

use marlin_agent_protocol::{
    AgentEvent, ModelGatewayCompletionOptions, ModelToolCall, ModelToolCallId, ModelToolDefinition,
    ModelToolName,
};
use marlin_agent_sessions::ConversationToolExecutionClaim;
use ring::digest::{Context as Sha256Context, SHA256};
use tracing::Instrument;

use crate::{
    ModelToolDeclaration, ModelToolGovernance, RuntimeContext, ShellToolHandler, ToolRuntime,
    observability::{RUNTIME_KIND_TOOL, TARGET_RUNTIME_SPAN},
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
///
/// B11 anti-rug-pull invariant: freezes a FIPS 180-4 SHA-256 schema digest for
/// every offered tool definition so post-pinning description or parameter schema
/// mutations are mechanically refused via [`ModelToolOffer::verify_pinned_schema`].
pub struct ModelToolOffer {
    entries: Vec<Entry>,
    index: HashMap<ModelToolName, usize>,
    pinned_schema_digests: BTreeMap<String, String>,
}

/// Rejection at the runtime offer boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelToolCatalogError {
    DuplicateName,
    InvalidCallSet,
    UnregisteredTool,
    SchemaRugPullDetected,
}

/// Computes a canonical SHA-256 digest (`sha256:<64 hex>`) binding a tool's
/// name, description, and JSON parameter schema (B11 anti-rug-pull pin).
pub fn schema_digest_for_definition(definition: &ModelToolDefinition) -> String {
    fn feed_canonical_json(ctx: &mut Sha256Context, value: &serde_json::Value) {
        match value {
            serde_json::Value::Null => ctx.update(b"null"),
            serde_json::Value::Bool(b) => ctx.update(if *b { b"true" } else { b"false" }),
            serde_json::Value::Number(n) => ctx.update(n.to_string().as_bytes()),
            serde_json::Value::String(s) => {
                ctx.update(b"s:");
                ctx.update(&(s.len() as u64).to_be_bytes());
                ctx.update(s.as_bytes());
            }
            serde_json::Value::Array(items) => {
                ctx.update(b"[");
                for item in items {
                    feed_canonical_json(ctx, item);
                    ctx.update(b",");
                }
                ctx.update(b"]");
            }
            serde_json::Value::Object(map) => {
                ctx.update(b"{");
                let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
                keys.sort_unstable();
                for key in keys {
                    ctx.update(b"k:");
                    ctx.update(&(key.len() as u64).to_be_bytes());
                    ctx.update(key.as_bytes());
                    ctx.update(b"=");
                    feed_canonical_json(ctx, &map[key]);
                    ctx.update(b";");
                }
                ctx.update(b"}");
            }
        }
    }

    let mut ctx = Sha256Context::new(&SHA256);
    ctx.update(b"marlin-tool-schema-v1\0");
    ctx.update(&(definition.name.as_str().len() as u64).to_be_bytes());
    ctx.update(definition.name.as_str().as_bytes());
    ctx.update(b"\0");
    ctx.update(&(definition.description.len() as u64).to_be_bytes());
    ctx.update(definition.description.as_bytes());
    ctx.update(b"\0");
    feed_canonical_json(&mut ctx, &definition.parameters);
    let digest = ctx.finish();
    let mut out = String::with_capacity(7 + 64);
    out.push_str("sha256:");
    for byte in digest.as_ref() {
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
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

    /// Freeze the handlers, definitions, and B11 SHA-256 schema digests for one provider request.
    pub fn offer(&self) -> ModelToolOffer {
        let pinned_schema_digests = self
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.definition.name.as_str().to_owned(),
                    schema_digest_for_definition(&entry.definition),
                )
            })
            .collect();
        ModelToolOffer {
            entries: self.entries.clone(),
            index: self
                .entries
                .iter()
                .enumerate()
                .map(|(position, entry)| (entry.definition.name.clone(), position))
                .collect(),
            pinned_schema_digests,
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
            outcomes.push(self.execute_single_call(call, &context).await?);
        }
        Ok(outcomes)
    }

    async fn execute_single_call(
        &self,
        call: ModelToolCall,
        context: &RuntimeContext,
    ) -> Result<(ModelToolCallId, ModelToolOutcome), ModelToolCatalogError> {
        let position = self
            .index
            .get(&call.name)
            .ok_or(ModelToolCatalogError::UnregisteredTool)?;
        let id = call.id.clone();
        let tool_name = call.name.as_str().to_owned();
        let args_json = match serde_json::to_string(&call.arguments) {
            Ok(serialized) => serialized,
            Err(err) => format!("<unserializable-arguments:{err}>"),
        };
        let args_preview = compact_tool_preview(&args_json, 140);
        let span = tracing::info_span!(
            target: TARGET_RUNTIME_SPAN,
            "runtime.tool.execute",
            runtime_kind = RUNTIME_KIND_TOOL,
            tool.name = tool_name.as_str(),
            tool.call_id = id.as_str(),
            tool.status = tracing::field::Empty,
            tool.duration_ms = tracing::field::Empty,
        );
        let _ = context
            .emit(AgentEvent::new(
                "runtime.tool.start",
                format!("{tool_name} [{}] {args_preview}", id.as_str()),
            ))
            .await;
        let started = Instant::now();
        let outcome = self.entries[*position]
            .handler
            .run_tool(call, context.child_context())
            .instrument(span.clone())
            .await;
        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let (status, detail_preview) = match &outcome {
            ModelToolOutcome::Completed(output) => ("completed", compact_tool_preview(output, 160)),
            ModelToolOutcome::Failed(error) => ("failed", compact_tool_preview(error, 160)),
        };
        span.record("tool.status", status);
        span.record("tool.duration_ms", elapsed_ms);
        span.in_scope(|| {
            tracing::info!(
                target: TARGET_RUNTIME_SPAN,
                runtime_kind = RUNTIME_KIND_TOOL,
                tool.name = tool_name.as_str(),
                tool.call_id = id.as_str(),
                tool.status = status,
                tool.duration_ms = elapsed_ms,
                "model tool execution finished"
            );
        });
        let _ = context
            .emit(AgentEvent::new(
                "runtime.tool.finish",
                format!(
                    "{tool_name} [{}] {status} ({elapsed_ms}ms) · {detail_preview}",
                    id.as_str()
                ),
            ))
            .await;
        Ok((id, outcome))
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

    /// Stable pinned SHA-256 schema digest map frozen at offer creation (B11).
    pub fn pinned_schema_digests(&self) -> &BTreeMap<String, String> {
        &self.pinned_schema_digests
    }

    /// B11 anti-rug-pull gate: verifies that an observed tool definition matches
    /// the exact SHA-256 schema digest pinned when this offer was frozen.
    pub fn verify_pinned_schema(
        &self,
        observed: &ModelToolDefinition,
    ) -> Result<(), ModelToolCatalogError> {
        let Some(expected_digest) = self.pinned_schema_digests.get(observed.name.as_str()) else {
            return Err(ModelToolCatalogError::UnregisteredTool);
        };
        let observed_digest = schema_digest_for_definition(observed);
        if observed_digest != *expected_digest {
            return Err(ModelToolCatalogError::SchemaRugPullDetected);
        }
        Ok(())
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
                        .all(|(tool, entry)| {
                            tool == &entry.definition && self.verify_pinned_schema(tool).is_ok()
                        })
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

fn compact_tool_preview(raw: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut chars = 0usize;
    let mut prev_space = false;
    for ch in raw.chars() {
        let mapped = if ch.is_control() || ch.is_whitespace() {
            ' '
        } else {
            ch
        };
        if mapped == ' ' {
            if prev_space || out.is_empty() {
                continue;
            }
            prev_space = true;
        } else {
            prev_space = false;
        }
        if chars >= max_chars {
            out.push('…');
            break;
        }
        out.push(mapped);
        chars += 1;
    }
    out.trim_end().to_owned()
}

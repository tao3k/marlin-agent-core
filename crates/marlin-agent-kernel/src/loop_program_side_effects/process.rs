//! Typed tool-process side effects for `LoopProgram` handoff projections.

use std::io;

use crate::{
    LoopProgramToolProcessProgram, LoopProgramToolProcessProjectionReceipt,
    LoopProgramToolProcessSpawnReceipt, LoopProgramToolProcessSpawnRequest,
};

use super::LoopProgramToolProcessSideEffectStatus;

/// Tool-process side-effect receipt with typed projection provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramToolProcessSideEffectReceipt {
    pub projection: LoopProgramToolProcessProjectionReceipt,
    pub status: LoopProgramToolProcessSideEffectStatus,
    pub spawn_receipt: Option<LoopProgramToolProcessSpawnReceipt>,
    pub diagnostic: Option<String>,
}

impl LoopProgramToolProcessSideEffectReceipt {
    pub(crate) fn completed(spawn_receipt: LoopProgramToolProcessSpawnReceipt) -> Self {
        Self {
            projection: spawn_receipt.projection.clone(),
            status: if spawn_receipt.output.status.success() {
                LoopProgramToolProcessSideEffectStatus::Completed
            } else {
                LoopProgramToolProcessSideEffectStatus::Failed
            },
            diagnostic: (!spawn_receipt.output.status.success()).then(|| {
                format!(
                    "loop_program.tool_process.exit_status={:?}",
                    spawn_receipt.output.status.code()
                )
            }),
            spawn_receipt: Some(spawn_receipt),
        }
    }

    pub(crate) fn skipped(projection: LoopProgramToolProcessProjectionReceipt) -> Self {
        Self {
            projection,
            status: LoopProgramToolProcessSideEffectStatus::Skipped,
            spawn_receipt: None,
            diagnostic: Some("loop_program.tool_process.unresolved_projection".to_owned()),
        }
    }

    pub(crate) fn failed(
        projection: LoopProgramToolProcessProjectionReceipt,
        error: io::Error,
    ) -> Self {
        Self {
            projection,
            status: LoopProgramToolProcessSideEffectStatus::Failed,
            spawn_receipt: None,
            diagnostic: Some(format!("loop_program.tool_process.spawn_failed:{error}")),
        }
    }
}

/// Resolves a typed tool projection into an explicit runtime process request.
pub trait LoopProgramToolProcessResolver: Send + Sync + 'static {
    fn resolve(
        &self,
        projection: &LoopProgramToolProcessProjectionReceipt,
    ) -> Option<LoopProgramToolProcessSpawnRequest>;
}

/// Static projection resolver for tests, smoke fixtures, and TOML-backed configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StaticLoopProgramToolProcessResolver {
    templates: Box<[LoopProgramToolProcessCommandTemplate]>,
}

impl StaticLoopProgramToolProcessResolver {
    pub fn new(templates: impl Into<Box<[LoopProgramToolProcessCommandTemplate]>>) -> Self {
        Self {
            templates: templates.into(),
        }
    }
}

impl LoopProgramToolProcessResolver for StaticLoopProgramToolProcessResolver {
    fn resolve(
        &self,
        projection: &LoopProgramToolProcessProjectionReceipt,
    ) -> Option<LoopProgramToolProcessSpawnRequest> {
        self.templates
            .iter()
            .find(|template| template.matches(projection))
            .map(|template| template.spawn_request(projection.clone()))
    }
}

/// One static mapping from a projected command observation to an executable process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramToolProcessCommandTemplate {
    command_kind: String,
    argv: Box<[String]>,
    program: LoopProgramToolProcessProgram,
    args: Box<[String]>,
}

impl LoopProgramToolProcessCommandTemplate {
    pub fn new<I, S>(
        command_kind: impl Into<String>,
        argv: I,
        program: LoopProgramToolProcessProgram,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            command_kind: command_kind.into(),
            argv: argv.into_iter().map(Into::into).collect(),
            program,
            args: Box::new([]),
        }
    }

    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    fn matches(&self, projection: &LoopProgramToolProcessProjectionReceipt) -> bool {
        projection.command.command_kind.as_str() == self.command_kind.as_str()
            && projection.command.argv.as_slice() == self.argv.as_ref()
    }

    fn spawn_request(
        &self,
        projection: LoopProgramToolProcessProjectionReceipt,
    ) -> LoopProgramToolProcessSpawnRequest {
        LoopProgramToolProcessSpawnRequest::new(projection, self.program.clone())
            .with_args(self.args.clone())
    }
}

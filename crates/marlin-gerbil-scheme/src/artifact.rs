//! Compiled `Gerbil` artifact envelopes.

use marlin_agent_protocol::AgentScenarioContract;
use marlin_gerbil_ir::{
    CompiledLoopGraph, MemoryDispatchPolicySpec, ReleaseTopologySpec, WorkspacePatchIntentSpec,
    WorkspaceSchemaSpec, WorkspaceValidationPolicySpec, WorkspaceViewPolicySpec,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

/// Typed artifact emitted by a `Gerbil` compiler.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GerbilCompiledArtifact {
    LoopGraph(CompiledLoopGraph),
    WorkspaceSchema(WorkspaceSchemaSpec),
    WorkspaceViewPolicy(WorkspaceViewPolicySpec),
    WorkspaceValidationPolicy(WorkspaceValidationPolicySpec),
    MemoryDispatchPolicy(MemoryDispatchPolicySpec),
    WorkspacePatchIntent(WorkspacePatchIntentSpec),
    AgentScenarioContract(AgentScenarioContract),
    ReleaseTopology(ReleaseTopologySpec),
}

impl GerbilCompiledArtifact {
    /// Return the typed artifact class emitted by the compiler boundary.
    pub fn kind(&self) -> GerbilArtifactKind {
        match self {
            Self::LoopGraph(_) => GerbilArtifactKind::LoopGraph,
            Self::WorkspaceSchema(_) => GerbilArtifactKind::WorkspaceSchema,
            Self::WorkspaceViewPolicy(_) => GerbilArtifactKind::WorkspaceViewPolicy,
            Self::WorkspaceValidationPolicy(_) => GerbilArtifactKind::WorkspaceValidationPolicy,
            Self::MemoryDispatchPolicy(_) => GerbilArtifactKind::MemoryDispatchPolicy,
            Self::WorkspacePatchIntent(_) => GerbilArtifactKind::WorkspacePatchIntent,
            Self::AgentScenarioContract(_) => GerbilArtifactKind::AgentScenarioContract,
            Self::ReleaseTopology(_) => GerbilArtifactKind::ReleaseTopology,
        }
    }

    /// Validate that the compiler returned the requested artifact class.
    pub fn ensure_kind(
        self,
        expected: GerbilArtifactKind,
    ) -> Result<Self, GerbilArtifactKindMismatch> {
        let actual = self.kind();
        if actual == expected {
            Ok(self)
        } else {
            Err(GerbilArtifactKindMismatch { expected, actual })
        }
    }
}

/// Expected artifact class requested from a `Gerbil` compile pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilArtifactKind {
    LoopGraph,
    WorkspaceSchema,
    WorkspaceViewPolicy,
    WorkspaceValidationPolicy,
    MemoryDispatchPolicy,
    WorkspacePatchIntent,
    AgentScenarioContract,
    ReleaseTopology,
}

/// Error returned when a compiler emits a different artifact class than requested.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilArtifactKindMismatch {
    pub expected: GerbilArtifactKind,
    pub actual: GerbilArtifactKind,
}

impl fmt::Display for GerbilArtifactKindMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "gerbil artifact kind mismatch: expected {:?}, got {:?}",
            self.expected, self.actual
        )
    }
}

impl Error for GerbilArtifactKindMismatch {}

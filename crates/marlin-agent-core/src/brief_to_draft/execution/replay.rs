// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Verify an authenticated graph and its closed file effects without a VM.

use marlin_agent_protocol::{
    GraphCAbiReadinessReceipt, GraphLoopExecutionResult, GraphLoopExecutionStatus,
    GraphLoopStrategy, GraphNodeExecutionStatus, GraphPolicyProposal, GraphPolicyProposalReceipt,
};
use marlin_agent_runtime::plugin::{
    ClosedWorkflowEvidence, EffectId, EffectOutcome, PluginHost, VerifiedRelease,
    WorkflowEvidenceError,
};
use serde::{Deserialize, Serialize};

use super::super::{INPUT_SCHEMA, SETTINGS_SCHEMA, file_capabilities, policy_module};
use super::{BriefRenderInput, BriefRun, capability, digest_ref, hash, workflow_abi_requirement};

#[cfg(test)]
mod tests;

const SCHEMA: &str = "marlin.brief-to-draft.run.v1";
// The nested effect evidence is already limited by its canonical encoder.
const LIMIT: usize = 18 * 1024 * 1024;

/// A settled workflow execution with typed graph receipts and file evidence.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BriefRunEvidence {
    schema: String,
    readiness: GraphCAbiReadinessReceipt,
    proposal: GraphPolicyProposal,
    admission: GraphPolicyProposalReceipt,
    execution: GraphLoopExecutionResult,
    render_input: BriefRenderInput,
    read: EffectId,
    create: Option<EffectId>,
    effects: ClosedWorkflowEvidence,
}

impl BriefRun {
    /// Export a completed or failed graph once its activation is closed.
    /// The caller retains the resulting bytes and authenticates their digest.
    /// This archive requires the closed generation to contain only this run's effects.
    pub fn export(self, host: &PluginHost<'_>) -> Result<Vec<u8>, WorkflowEvidenceError> {
        let (read, input) = self.input;
        let mut payloads = std::collections::BTreeMap::from([(read, input)]);
        let create = match self.output {
            Ok(Some((id, bytes))) => {
                payloads.insert(id, bytes);
                Some(id)
            }
            Err(marlin_agent_runtime::plugin::WorkspaceEffectError::Effect { id, .. }) => Some(id),
            _ => None,
        };
        let effects = host.export_closed_effects(&self.activation, payloads)?;
        effects.encode()?;
        let evidence = BriefRunEvidence {
            schema: SCHEMA.into(),
            readiness: self.readiness,
            proposal: self.proposal,
            admission: self.admission,
            execution: self.execution,
            render_input: self.render_input,
            read,
            create,
            effects,
        };
        evidence.check_graph()?;
        let encoded =
            serde_json::to_vec(&evidence).map_err(|_| WorkflowEvidenceError::InvalidEncoding)?;
        if encoded.len() > LIMIT {
            return Err(WorkflowEvidenceError::InvalidEncoding);
        }
        Ok(encoded)
    }
}

impl BriefRunEvidence {
    /// Verify the persisted graph and file effects under an authenticated root digest.
    pub fn verify(
        bytes: &[u8],
        expected: &[u8; 32],
        release: &VerifiedRelease,
    ) -> Result<Self, WorkflowEvidenceError> {
        if bytes.len() > LIMIT {
            return Err(WorkflowEvidenceError::InvalidEncoding);
        }
        if hash(bytes) != *expected {
            return Err(WorkflowEvidenceError::Unauthenticated);
        }
        let evidence: Self =
            serde_json::from_slice(bytes).map_err(|_| WorkflowEvidenceError::InvalidEncoding)?;
        evidence.effects.validate_release(release)?;
        let workflow = release
            .workflow(evidence.effects.activation().id.workflow())
            .ok_or(WorkflowEvidenceError::Inconsistent)?;
        let capabilities = file_capabilities();
        if workflow.policy_module != policy_module()
            || workflow.input_schema.as_str() != INPUT_SCHEMA
            || workflow.settings_schema.as_str() != SETTINGS_SCHEMA
            || workflow.requires.len() != capabilities.len()
            || capabilities.iter().any(|(id, effect)| {
                !workflow.requires.contains(id) || release.capability(id) != Some(effect)
            })
        {
            return Err(WorkflowEvidenceError::Inconsistent);
        }
        evidence.check_graph()?;
        Ok(evidence)
    }

    /// The recorded terminal outcome and ordered per-node receipts.
    pub fn execution(&self) -> &GraphLoopExecutionResult {
        &self.execution
    }

    fn check_graph(&self) -> Result<(), WorkflowEvidenceError> {
        use WorkflowEvidenceError::{Incomplete, Inconsistent};
        let graph = &self.proposal.proposed_graph;
        let run = &self.execution;
        let planned = serde_json::to_vec(graph).map_err(|_| Inconsistent)?;
        let compiled = marlin_agent_kernel::compile_graph_policy_proposal_with_c_abi_readiness(
            &run.snapshot.run_id,
            &self.proposal,
            &self.readiness,
        );
        if self.schema != SCHEMA
            || self.proposal.strategy != GraphLoopStrategy::gerbil_c_abi("brief-to-draft", "v0")
            || self.proposal.c_abi.as_ref() != Some(&workflow_abi_requirement())
            || !self.render_input.is_valid()
            || self.render_input.settings_digest() != self.effects.activation().settings_digest
            || !compiled.is_accepted()
            || compiled.receipt != self.admission
            || self.proposal.output_digest.as_str() != digest_ref(hash(&planned))
            || run.snapshot.graph_id != graph.graph_id
            || run.snapshot.active_node.is_some()
        {
            return Err(Inconsistent);
        }
        if *graph != super::graph() {
            return Err(Inconsistent);
        }
        let receipts = &run.node_receipts;
        if receipts.len() > 2
            || receipts.iter().zip(&graph.nodes).any(|(receipt, node)| {
                receipt.node_id.as_str() != node.id || receipt.executor.as_str() != node.executor
            })
            || run.visited_nodes.iter().map(String::as_str).ne(receipts
                .iter()
                .filter(|r| r.status == GraphNodeExecutionStatus::Completed)
                .map(|r| r.node_id.as_str()))
        {
            return Err(Inconsistent);
        }
        let read = self
            .effects
            .effects()
            .iter()
            .find(|effect| effect.id == self.read)
            .ok_or(Incomplete)?;
        let EffectOutcome::Committed { output_digest } = read.outcome else {
            return Err(Inconsistent);
        };
        if read.capability != capability("read-brief")
            || self.proposal.input_digest.as_str() != digest_ref(output_digest)
        {
            return Err(Inconsistent);
        }
        let create = self
            .create
            .map(|id| {
                self.effects
                    .effects()
                    .iter()
                    .find(|effect| effect.id == id)
                    .ok_or(Incomplete)
            })
            .transpose()?;
        if self.effects.effects().len() != 1 + usize::from(create.is_some())
            || create.is_some_and(|effect| {
                effect.id <= self.read || effect.capability != capability("create-draft")
            })
        {
            return Err(Inconsistent);
        }
        match (
            run.status.clone(),
            receipts.as_slice(),
            create.map(|effect| effect.outcome),
        ) {
            (
                GraphLoopExecutionStatus::Completed,
                [render, create],
                Some(EffectOutcome::Committed { output_digest }),
            ) if render.status == GraphNodeExecutionStatus::Completed
                && create.status == GraphNodeExecutionStatus::Completed
                && output_digest == hash(&self.render_input.render()) =>
            {
                Ok(())
            }
            (GraphLoopExecutionStatus::Failed, [render, create], outcome)
                if render.status == GraphNodeExecutionStatus::Completed
                    && create.status == GraphNodeExecutionStatus::Failed
                    && !matches!(outcome, Some(EffectOutcome::Committed { .. })) =>
            {
                Ok(())
            }
            (GraphLoopExecutionStatus::Cancelled, [], None) => Ok(()),
            (GraphLoopExecutionStatus::Cancelled, [render], None)
                if render.status == GraphNodeExecutionStatus::Completed =>
            {
                Ok(())
            }
            _ => Err(Inconsistent),
        }
    }
}

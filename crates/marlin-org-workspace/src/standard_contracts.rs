//! Built-in Org contract libraries shipped with the workspace adapter.

use marlin_workspace_protocol::WorkspaceResult;

use crate::{OrgDocument, OrgDocumentLoader, OrgDocumentWorkspace};

/// Stable document id for the built-in agent plan contract.
pub const STANDARD_AGENT_PLAN_CONTRACT_DOCUMENT_ID: &str = "org:contracts/agent.plan.v1";

/// Stable document id for the built-in agent task contract.
pub const STANDARD_AGENT_TASK_CONTRACT_DOCUMENT_ID: &str = "org:contracts/agent.task.v1";

/// Stable document id for the built-in agent loop contract.
pub const STANDARD_AGENT_LOOP_CONTRACT_DOCUMENT_ID: &str = "org:contracts/agent.loop.v1";

/// Stable document id for the built-in agent memory contract.
pub const STANDARD_AGENT_MEMORY_CONTRACT_DOCUMENT_ID: &str = "org:contracts/agent.memory.v1";

/// Stable document id for the built-in agent topology contract.
pub const STANDARD_AGENT_TOPOLOGY_CONTRACT_DOCUMENT_ID: &str = "org:contracts/agent.topology.v1";

/// Built-in Contract Org for agent plans.
pub const STANDARD_AGENT_PLAN_CONTRACT_ORG: &str =
    include_str!("../../../org/contracts/agent.plan.v1.org");

/// Built-in Contract Org for agent tasks.
pub const STANDARD_AGENT_TASK_CONTRACT_ORG: &str =
    include_str!("../../../org/contracts/agent.task.v1.org");

/// Built-in Contract Org for agent loops.
pub const STANDARD_AGENT_LOOP_CONTRACT_ORG: &str =
    include_str!("../../../org/contracts/agent.loop.v1.org");

/// Built-in Contract Org for agent memory records.
pub const STANDARD_AGENT_MEMORY_CONTRACT_ORG: &str =
    include_str!("../../../org/contracts/agent.memory.v1.org");

/// Built-in Contract Org for agent project topology records.
pub const STANDARD_AGENT_TOPOLOGY_CONTRACT_ORG: &str =
    include_str!("../../../org/contracts/agent.topology.v1.org");

/// Returns the built-in agent Contract Org documents, one document per contract.
pub fn standard_agent_contract_documents() -> Vec<OrgDocument> {
    vec![
        OrgDocument::new(
            STANDARD_AGENT_PLAN_CONTRACT_DOCUMENT_ID,
            STANDARD_AGENT_PLAN_CONTRACT_ORG,
        ),
        OrgDocument::new(
            STANDARD_AGENT_TASK_CONTRACT_DOCUMENT_ID,
            STANDARD_AGENT_TASK_CONTRACT_ORG,
        ),
        OrgDocument::new(
            STANDARD_AGENT_LOOP_CONTRACT_DOCUMENT_ID,
            STANDARD_AGENT_LOOP_CONTRACT_ORG,
        ),
        OrgDocument::new(
            STANDARD_AGENT_MEMORY_CONTRACT_DOCUMENT_ID,
            STANDARD_AGENT_MEMORY_CONTRACT_ORG,
        ),
        OrgDocument::new(
            STANDARD_AGENT_TOPOLOGY_CONTRACT_DOCUMENT_ID,
            STANDARD_AGENT_TOPOLOGY_CONTRACT_ORG,
        ),
    ]
}

/// Loads all built-in agent Contract Org documents into typed workspace facts.
pub fn load_standard_agent_contract_workspace() -> WorkspaceResult<OrgDocumentWorkspace> {
    let mut merged = OrgDocumentWorkspace::default();
    for document in standard_agent_contract_documents() {
        let workspace = OrgDocumentLoader::load_workspace(&document)?;
        merge_contract_workspace(&mut merged, workspace);
    }
    Ok(merged)
}

fn merge_contract_workspace(target: &mut OrgDocumentWorkspace, workspace: OrgDocumentWorkspace) {
    target.nodes.extend(workspace.nodes);
    target
        .contracts
        .contracts
        .extend(workspace.contracts.contracts);
    target
        .contract_resolutions
        .references
        .extend(workspace.contract_resolutions.references);
    target
        .contract_resolutions
        .diagnostics
        .extend(workspace.contract_resolutions.diagnostics);
    target
        .contract_validations
        .receipts
        .extend(workspace.contract_validations.receipts);
    target
        .contract_validations
        .diagnostics
        .extend(workspace.contract_validations.diagnostics);
}

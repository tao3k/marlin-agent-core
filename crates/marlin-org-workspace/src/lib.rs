//! First-party `Org` document loading adapter for workspace backends.

mod contract;
mod document;
mod standard_contracts;
mod validation;

pub use document::{OrgDocument, OrgDocumentId, OrgDocumentLoader, OrgDocumentWorkspace};
pub use standard_contracts::{
    STANDARD_AGENT_LOOP_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_LOOP_CONTRACT_ORG,
    STANDARD_AGENT_MEMORY_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_MEMORY_CONTRACT_ORG,
    STANDARD_AGENT_PLAN_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_PLAN_CONTRACT_ORG,
    STANDARD_AGENT_TASK_CONTRACT_DOCUMENT_ID, STANDARD_AGENT_TASK_CONTRACT_ORG,
    load_standard_agent_contract_workspace, standard_agent_contract_documents,
};

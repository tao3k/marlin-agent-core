// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Marlin-native search engine bridge onto Agent Semantic Protocols (ASP).
//!
//! Connects Marlin's three native authorities directly to ASP's pure-Rust
//! converged search engine without gRPC hops, without pseudo S-expression text
//! parsing, without a second Orgize runtime, and without a second Turso DB:
//!
//! 1. **Real Gerbil Scheme search plans** (`scheme/search-engine/*` + `scheme/c-abi/search-engine.ss`)
//!    projected via [`gerbil_scheme::RootedSchemeObject`] into
//!    [`SearchPlaybookPlanInput`].
//! 2. **Marlin Org authority** (`marlin-context-org-model` / `marlin-agent-context`)
//!    wired via [`MarlinContextOrgPort`] implementing [`OrgDocumentProjectionPort`].
//! 3. **Marlin unified storage** (`marlin-agent-storage::AgentStorage`)
//!    wired via [`MarlinAgentStorageSearchPort`] implementing [`SearchEngineStoragePort`].

mod engine;
mod org_port;
mod projection;
mod storage_port;

pub use agent_semantic_search::{
    ConvergedLineMatch, ConvergedSearchExecutionError, ConvergedSearchExecutionReport,
    ConvergedSearchHit, DEFAULT_SEARCH_GENERATION_POINTER_KEY, MarlinConvergedSearchEngine,
    SearchArtifactBlob, SearchArtifactHash, SearchEngineStoragePort, SearchGenerationDigest,
    SearchGenerationPointerRecord, SearchGenerationPointerSwapRequest, SearchPointerKey,
    SearchProjectId, SearchStoragePortError, SearchTopologyEdgeId, SearchTopologyEdgeRecord,
    SearchWorkspaceId, TrigramQuery, TrigramSnapshotError, WorkspaceContextLine,
    WorkspaceDeltaReceipt, WorkspaceFileDelta, WorkspaceIndexConfig, WorkspaceSearchHit,
    WorkspaceSearchIndex, WorkspaceSearchQuery, WorkspaceSearchReport, compile_regex_trigram_query,
    execute_admitted_playbook,
};
pub use agent_semantic_search_playbook::{
    GraphNativeBlock, ProducerNativeBlock, ProgressiveSearchPlaybookError,
    ProgressiveSearchPlaybookRequest, SearchPlaybookComposition, SearchPlaybookLeaf,
    SearchPlaybookPlanInput, TopologyOwnerMembershipBlock, admit_search_playbook_plan,
};
pub use agent_semantic_topology::{
    AgentTopologyRelationship, OrgDocumentProjection, OrgDocumentProjectionPort,
    OrgOverlayDocumentProjection, OrgProjectionPortError, OrgPropertyEntry, OrgSectionProjection,
};
pub use engine::marlin_search_engine;
pub use org_port::{
    MarlinContextOrgPort, parse_live_org_nodes, project_org_nodes_to_document,
    project_org_nodes_to_overlay,
};
pub use projection::{
    SearchPlanProjectionError, admit_rooted_search_plan, decode_search_plan,
    decode_workspace_index_config, project_linked, project_linked_helm_index_config,
    project_linked_helm_prompt,
};
#[cfg(not(feature = "linked-ui"))]
pub use projection::{project_from_package, project_helm_index_config, project_helm_prompt};
pub use storage_port::{MarlinAgentStorageSearchPort, MarlinSearchStorageProvenance};

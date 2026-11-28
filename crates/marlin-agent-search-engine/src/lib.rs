// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Marlin-native search engine bridge onto Agent Semantic Protocols (ASP).
//!
//! Connects Marlin's three native authorities directly to ASP's pure-Rust
//! converged search engine without gRPC hops, without pseudo S-expression text
//! parsing, without a second Orgize runtime, and without a second Turso DB:
//!
//! 1. **Real Gerbil Scheme search plans & `:gerbil-ascent` / `:gerbil-parser` DSL**
//!    (`scheme/search-engine/*`, `scheme/search-dsl/*`, `scheme/c-abi/search-*.ss`)
//!    projected via [`gerbil_scheme::RootedSchemeObject`].
//! 2. **Marlin Org authority** (`marlin-context-org-model` / `marlin-agent-context`)
//!    wired via [`MarlinContextOrgPort`] implementing [`OrgDocumentProjectionPort`].
//! 3. **Marlin unified storage** (`marlin-agent-storage::AgentStorage`)
//!    wired via [`MarlinAgentStorageSearchPort`] implementing [`SearchEngineStoragePort`].

mod asp_http_client;
mod edb_bridge;
mod engine;
mod open_edb_hydrator;
mod org_port;
mod projection;
mod storage_port;

pub use asp_http_client::{
    ASP_FORWARDING_TOKEN_ENV, ASP_FORWARDING_URL_ENV, AspChunkedHydrationRequest,
    AspForwardingEndpoint, AspHttpAstPatchOp, AspHttpAstPatchReceipt, AspHttpAstPatchRequest,
    AspHttpAstPatchStatus, AspHttpHydrationReceipt, AspHttpHydrationRequest, AspLanguageId,
    AspProviderId, AspScopedHydrationRequest, DynamicLanguageDescriptorRequest, DynamicProviderCapabilities,
    DynamicProviderDslPackRegistration, DynamicProviderRegistrationAck,
    DynamicProviderRegistrationManifest, DynamicProviderRegistrationRequest,
    DynamicProviderSyntaxSurface, DynamicProviderTransport, DynamicSpecialRelationSpec,
    ForwardedDslPack, ForwardedKindMapping, ForwardedProjectSurface, ForwardedProviderCatalog,
    ForwardedProviderEnvelope, ForwardedProviderManifest, ForwardedSpecialExtension,
    GeneralOntologyKindName, HttpEndpointPath, NativeAstKindName,
    ProviderRegistrationMode, ProviderRegistrationStatus, ProviderTransportProtocol,
    TargetedJsonSchemaDocument, apply_ast_patch_via_asp_http, fetch_asp_provider_catalog,
    hydrate_workspace_chunked_via_asp_http, hydrate_workspace_pending_via_asp_http,
    hydrate_workspace_scoped_via_asp_http, merge_completion_catalogs,
    project_json_schema_completions, project_provider_catalog_completions,
    project_provider_envelope_completions, register_dynamic_provider_on_asp,
    register_linked_dynamic_language_descriptor, shutdown_asp_gateway,
    sync_linked_asp_provider_descriptors,
};
#[cfg(not(feature = "linked-ui"))]
pub use asp_http_client::{
    hydrate_workspace_from_asp_http, register_package_dynamic_language_descriptor,
    sync_package_asp_provider_descriptors,
};

pub use agent_semantic_search::{
    ConvergedLineMatch, ConvergedSearchExecutionError, ConvergedSearchExecutionReport,
    ConvergedSearchHit, DEFAULT_SEARCH_GENERATION_POINTER_KEY, MarlinConvergedSearchEngine,
    OPEN_EDB_CACHE_SCHEMA_VERSION, SEMANTIC_OPEN_EDB_SCHEMA_ID, SEMANTIC_OPEN_EDB_SCHEMA_VERSION,
    SearchArtifactBlob, SearchArtifactHash, SearchEngineStoragePort, SearchGenerationDigest,
    SearchGenerationPointerRecord, SearchGenerationPointerSwapRequest, SearchPointerKey,
    SearchProjectId, SearchStoragePortError, SearchTopologyEdgeId, SearchTopologyEdgeRecord,
    SearchWorkspaceId, TrigramQuery, TrigramSnapshotError, WorkspaceContextLine,
    WorkspaceDeltaReceipt, WorkspaceEdbAttr, WorkspaceEdbEdgeFact, WorkspaceEdbEntityFact,
    WorkspaceEdbFileFact, WorkspaceEdbPrefilter, WorkspaceEdbSnapshot, WorkspaceFileDelta,
    WorkspaceIndexConfig, WorkspaceOpenEdbCache, WorkspaceOpenEdbCacheFile,
    WorkspaceOpenEdbIngestBatch, WorkspaceOpenEdbIngestReceipt, WorkspaceOpenEdbPendingFile,
    WorkspaceOpenEdbRestoreReceipt, WorkspaceSearchHit, WorkspaceSearchIndex, WorkspaceSearchQuery,
    WorkspaceSearchReport, WorkspaceSymbolKind, WorkspaceSymbolMatch, compile_regex_trigram_query,
    execute_admitted_playbook, execute_admitted_playbook_with_index,
    parse_native_syntax_fact_index_json, parse_provider_projection_batch_json,
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
pub use edb_bridge::{
    EdbHydrationMask, OwnedWorkspaceEdbPrefilter, SearchDslChoiceRuleKind, SearchDslChoiceWitness,
    SearchDslChoiceWitnessKind, SearchDslChoiceWorld, SearchDslCompletionEntry,
    SearchDslCompletionKind, SearchDslExecutionMode, SearchDslExecutionRequest,
    SearchDslLinkedExports, SearchDslSessionExecutionRequest, SearchDslSessionId,
    SearchDslSessionRunner, SearchDslStructuredResult, SearchDslStructuredRow,
    SearchDslStructuredRunner, SearchEpistemicStatus, SearchEvidenceChainReceipt,
    decode_edb_prefilter, decode_rooted_search_dsl_result, decode_search_dsl_completions,
    default_marlin_scheme_dir, default_search_dsl_session_runner,
    default_search_dsl_structured_runner, encode_edb_snapshot,
    execute_linked_search_dsl_session_structured, execute_linked_search_dsl_structured,
    format_rooted_search_dsl_result, project_linked_search_dsl_completions,
};
#[cfg(not(feature = "linked-ui"))]
pub use edb_bridge::{
    default_package_search_dsl_session_runner, default_package_search_dsl_structured_runner,
    execute_package_search_dsl_session_structured, execute_package_search_dsl_structured,
    project_package_search_dsl_completions,
};
pub use engine::{
    default_workspace_edb_cache_path, default_workspace_snapshot_path, marlin_search_engine,
    open_or_build_workspace_index_safe, save_workspace_snapshot_atomic,
};
#[cfg(not(feature = "linked-ui"))]
pub use open_edb_hydrator::hydrate_package_workspace_open_edb;
pub use open_edb_hydrator::{
    OpenEdbProviderId, WorkspaceOpenEdbHydrationReceipt, decode_extracted_open_edb_batch,
    extract_org_open_edb_batch, hydrate_linked_workspace_open_edb,
    hydrate_workspace_open_edb_org_and_cache,
};
pub use org_port::{
    MarlinContextOrgPort, parse_live_org_nodes, project_org_nodes_to_document,
    project_org_nodes_to_overlay,
};
pub use projection::{
    AspHydrationPolicy, HelmIndexPolicyProjection, HelmSearchPresetSpec, SearchPlanProjectionError,
    admit_rooted_search_plan, decode_helm_index_policy, decode_search_plan,
    decode_workspace_index_config, project_linked, project_linked_helm_index_config,
    project_linked_helm_index_policy, project_linked_helm_prompt,
};
#[cfg(not(feature = "linked-ui"))]
pub use projection::{
    project_from_package, project_helm_index_config, project_helm_index_policy, project_helm_prompt,
};
pub use storage_port::{MarlinAgentStorageSearchPort, MarlinSearchStorageProvenance};

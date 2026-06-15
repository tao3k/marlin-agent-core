//! `MemoryOrgWorkspace` owner for the in-memory `Org` workspace backend.

use super::{
    content_graph, contracts, patch, project_graph, query, render, session_graph, status,
    tool_graph,
};
use async_trait::async_trait;
use marlin_agent_protocol::{
    GraphQueryRequest, GraphQueryResponse, ProjectMemoryContextFact, ProjectMemoryContextPack,
    ProjectMemoryRecallRequest,
};
use marlin_gerbil_ir::ReleaseTopologySpec;
use marlin_org_model::{OrgNode, OrgNodeId, OrgSourceSpan};
use marlin_org_store::{
    OrgProjectRoot, OrgProjectRootCandidate, OrgSourceStore, discover_project_roots,
};
use marlin_org_workspace::{
    OrgDocument, OrgDocumentLoader, OrgDocumentWorkspace, standard_agent_contract_documents,
};
use marlin_workspace_patch::{WorkspacePatch, WorkspacePatchReceipt, WorkspaceValidationReport};
use marlin_workspace_protocol::{AgentWorkspace, WorkspaceCtx, WorkspaceError, WorkspaceResult};
use marlin_workspace_query::{QueryMatch, WorkspaceQuery, WorkspaceQueryResult, WorkspaceScope};
use marlin_workspace_status::{
    ReleaseGateReceipt, ReleaseStatus, WorkspaceStatusReport, WorkspaceTarget,
};
use marlin_workspace_view::{
    RenderedContractFacts, RenderedViewNode, RenderedWorkspaceView, WorkspaceField,
    WorkspaceViewSpec,
};
use parking_lot::{RwLock, RwLockReadGuard};
use std::collections::BTreeMap;

/// In-memory implementation of the native `AgentWorkspace` protocol.
pub struct MemoryOrgWorkspace {
    nodes: RwLock<BTreeMap<OrgNodeId, OrgNode>>,
    contract_facts: RwLock<RenderedContractFacts>,
    last_patch_receipt: RwLock<Option<WorkspacePatchReceipt>>,
    release_status: RwLock<Option<ReleaseStatus>>,
}

/// Store-backed project memory recall request with named call-site fields.
pub struct ProjectMemoryStoreRecall<'a, S>
where
    S: OrgSourceStore,
{
    pub context_pack_id: String,
    pub receipt_id: String,
    pub request: ProjectMemoryRecallRequest,
    pub store: &'a S,
    pub candidates: Vec<OrgProjectRootCandidate>,
}

/// Store-backed project memory graph query request with named call-site fields.
pub struct ProjectMemoryGraphStoreQuery<'a, S>
where
    S: OrgSourceStore,
{
    pub receipt_id: String,
    pub request: GraphQueryRequest,
    pub store: &'a S,
    pub candidates: Vec<OrgProjectRootCandidate>,
}

/// Store-backed tool capability graph query request with named call-site fields.
pub struct ToolCapabilityGraphStoreQuery<'a, S>
where
    S: OrgSourceStore,
{
    pub receipt_id: String,
    pub request: GraphQueryRequest,
    pub store: &'a S,
    pub candidates: Vec<OrgProjectRootCandidate>,
}

impl MemoryOrgWorkspace {
    /// Create an empty in-memory workspace.
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(BTreeMap::new()),
            contract_facts: RwLock::new(RenderedContractFacts::default()),
            last_patch_receipt: RwLock::new(None),
            release_status: RwLock::new(None),
        }
    }

    /// Create an in-memory workspace from existing structured `Org` nodes.
    pub fn from_nodes(nodes: impl IntoIterator<Item = OrgNode>) -> Self {
        let mut indexed = BTreeMap::new();
        for node in nodes {
            indexed.insert(node.id.clone(), node);
        }
        Self {
            nodes: RwLock::new(indexed),
            contract_facts: RwLock::new(RenderedContractFacts::default()),
            last_patch_receipt: RwLock::new(None),
            release_status: RwLock::new(None),
        }
    }

    /// Insert or replace one structured `Org` node.
    pub fn insert_node(&self, node: OrgNode) -> WorkspaceResult<()> {
        let mut nodes = self.nodes.write();
        nodes.insert(node.id.clone(), node);
        Ok(())
    }

    /// Record the latest release status projection visible through `status()`.
    pub fn record_release_status(&self, status: ReleaseStatus) -> WorkspaceResult<()> {
        *self.release_status.write() = Some(status);
        Ok(())
    }

    /// Record a pending release status from a `Gerbil` release topology artifact.
    pub fn record_release_topology(
        &self,
        topology: &ReleaseTopologySpec,
    ) -> WorkspaceResult<ReleaseStatus> {
        let status = ReleaseStatus::pending_from_topology(topology);
        self.record_release_status(status.clone())?;
        Ok(status)
    }

    /// Record execution evidence for one gate in the latest release status.
    pub fn record_release_gate_receipt(
        &self,
        receipt: ReleaseGateReceipt,
    ) -> WorkspaceResult<bool> {
        let mut release_status = self.release_status.write();
        Ok(release_status
            .as_mut()
            .is_some_and(|status| status.record_gate_receipt(receipt)))
    }

    /// Load a raw `Org` document into the in-memory workspace.
    pub fn load_document(&self, document: OrgDocument) -> WorkspaceResult<Vec<OrgNodeId>> {
        let workspace = OrgDocumentLoader::load_workspace(&document)?;
        self.load_workspace(workspace)
    }

    /// Load a raw `Org` document with additional external contract documents.
    pub fn load_document_with_contracts(
        &self,
        document: OrgDocument,
        contract_documents: &[OrgDocument],
    ) -> WorkspaceResult<Vec<OrgNodeId>> {
        let workspace =
            OrgDocumentLoader::load_workspace_with_contracts(&document, contract_documents)?;
        self.load_workspace(workspace)
    }

    /// Load a raw `Org` document with the built-in agent Contract Org library.
    pub fn load_document_with_standard_agent_contracts(
        &self,
        document: OrgDocument,
    ) -> WorkspaceResult<Vec<OrgNodeId>> {
        let contract_documents = standard_agent_contract_documents();
        self.load_document_with_contracts(document, &contract_documents)
    }

    /// Load multiple `Org` documents and discover contract registry documents from the batch.
    pub fn load_documents_with_discovered_contracts(
        &self,
        documents: &[OrgDocument],
    ) -> WorkspaceResult<Vec<OrgNodeId>> {
        let contract_documents = OrgDocumentLoader::discover_contract_documents(documents);
        self.load_documents_with_contracts(documents, &contract_documents)
    }

    /// Load multiple `Org` documents with discovered contracts plus built-in agent contracts.
    pub fn load_documents_with_standard_agent_contracts(
        &self,
        documents: &[OrgDocument],
    ) -> WorkspaceResult<Vec<OrgNodeId>> {
        let mut contract_documents = OrgDocumentLoader::discover_contract_documents(documents);
        contract_documents.extend(standard_agent_contract_documents());
        self.load_documents_with_contracts(documents, &contract_documents)
    }

    /// Load multiple `Org` documents with explicit contract registry documents.
    pub fn load_documents_with_contracts(
        &self,
        documents: &[OrgDocument],
        contract_documents: &[OrgDocument],
    ) -> WorkspaceResult<Vec<OrgNodeId>> {
        let mut merged_workspace = OrgDocumentWorkspace::default();
        for document in documents {
            let external_contract_documents = contract_documents
                .iter()
                .filter(|contract_document| contract_document.id != document.id)
                .cloned()
                .collect::<Vec<_>>();
            let workspace = OrgDocumentLoader::load_workspace_with_contracts(
                document,
                &external_contract_documents,
            )?;
            contracts::merge_document_workspace(&mut merged_workspace, workspace);
        }
        self.load_workspace(merged_workspace)
    }

    fn load_workspace(&self, workspace: OrgDocumentWorkspace) -> WorkspaceResult<Vec<OrgNodeId>> {
        let ids = workspace
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        {
            let mut nodes = self.nodes.write();
            for node in workspace.nodes {
                nodes.insert(node.id.clone(), node);
            }
        }
        let incoming_contract_facts = contracts::contract_facts_from_workspace(
            workspace.contracts,
            workspace.contract_resolutions,
            workspace.contract_validations,
        );
        let mut contract_facts = self.contract_facts.write();
        contracts::merge_contract_facts(&mut contract_facts, incoming_contract_facts);
        Ok(ids)
    }

    /// Project memory graph projection over the structured in-memory `Org` nodes.
    pub fn query_project_memory_graph(
        &self,
        receipt_id: impl Into<String>,
        request: GraphQueryRequest,
    ) -> WorkspaceResult<GraphQueryResponse> {
        let nodes = self.read_nodes()?;
        let matches = project_graph::project_memory_matches(nodes.values(), &request);
        let mut response = GraphQueryResponse::new(receipt_id, request);
        response.matches = matches;
        Ok(response)
    }

    /// Query compact tool capability cards from loaded Org nodes.
    pub fn query_tool_capability_graph(
        &self,
        receipt_id: impl Into<String>,
        request: GraphQueryRequest,
    ) -> WorkspaceResult<GraphQueryResponse> {
        let nodes = self.read_nodes()?;
        let matches = tool_graph::tool_capability_matches(nodes.values(), &request);
        let mut response = GraphQueryResponse::new(receipt_id, request);
        response.matches = matches;
        Ok(response)
    }

    /// Query compact session boundary facts from loaded Org nodes.
    pub fn query_session_graph(
        &self,
        receipt_id: impl Into<String>,
        request: GraphQueryRequest,
    ) -> WorkspaceResult<GraphQueryResponse> {
        let nodes = self.read_nodes()?;
        let matches = session_graph::session_matches(nodes.values(), &request);
        let mut response = GraphQueryResponse::new(receipt_id, request);
        response.matches = matches;
        Ok(response)
    }

    /// Query compact content cards from loaded Org nodes.
    pub fn query_content_graph(
        &self,
        receipt_id: impl Into<String>,
        request: GraphQueryRequest,
    ) -> WorkspaceResult<GraphQueryResponse> {
        let nodes = self.read_nodes()?;
        let matches = content_graph::content_matches(nodes.values(), &request);
        let mut response = GraphQueryResponse::new(receipt_id, request);
        response.matches = matches;
        Ok(response)
    }

    /// Recall project memory from discovered Org roots and pack compact facts.
    pub fn recall_project_memory_from_roots(
        &self,
        context_pack_id: impl Into<String>,
        receipt_id: impl Into<String>,
        request: ProjectMemoryRecallRequest,
        roots: &[OrgProjectRoot],
    ) -> WorkspaceResult<ProjectMemoryContextPack> {
        let documents = roots
            .iter()
            .map(|root| OrgDocument::new(root.document.clone(), root.body.clone()))
            .collect::<Vec<_>>();
        self.load_documents_with_discovered_contracts(&documents)?;

        let response =
            self.query_project_memory_graph(receipt_id, request.as_graph_query_request())?;
        let mut pack = ProjectMemoryContextPack::new(context_pack_id, request)
            .with_source_receipt(response.receipt_id.as_str());
        let nodes = self.read_nodes()?;
        for query_match in response.matches {
            pack = pack.with_fact(project_memory_context_fact(query_match, &nodes));
        }
        Ok(pack)
    }

    /// Discover project memory roots from a store and pack compact recall facts.
    pub fn recall_project_memory_from_store<S>(
        &self,
        recall: ProjectMemoryStoreRecall<'_, S>,
    ) -> WorkspaceResult<ProjectMemoryContextPack>
    where
        S: OrgSourceStore,
    {
        let ProjectMemoryStoreRecall {
            context_pack_id,
            receipt_id,
            request,
            store,
            candidates,
        } = recall;
        let roots = discover_project_roots(store, candidates);
        self.recall_project_memory_from_roots(context_pack_id, receipt_id, request, &roots)
    }

    /// Discover project memory roots from a store and run a compact graph query.
    pub fn query_project_memory_graph_from_store<S>(
        &self,
        query: ProjectMemoryGraphStoreQuery<'_, S>,
    ) -> WorkspaceResult<GraphQueryResponse>
    where
        S: OrgSourceStore,
    {
        let ProjectMemoryGraphStoreQuery {
            receipt_id,
            request,
            store,
            candidates,
        } = query;
        let roots = discover_project_roots(store, candidates);
        let documents = roots
            .iter()
            .map(|root| OrgDocument::new(root.document.clone(), root.body.clone()))
            .collect::<Vec<_>>();
        self.load_documents_with_discovered_contracts(&documents)?;
        self.query_project_memory_graph(receipt_id, request)
    }

    /// Discover tool capability roots from a store and run a compact graph query.
    pub fn query_tool_capability_graph_from_store<S>(
        &self,
        query: ToolCapabilityGraphStoreQuery<'_, S>,
    ) -> WorkspaceResult<GraphQueryResponse>
    where
        S: OrgSourceStore,
    {
        let ToolCapabilityGraphStoreQuery {
            receipt_id,
            request,
            store,
            candidates,
        } = query;
        let roots = discover_project_roots(store, candidates);
        let documents = roots
            .iter()
            .map(|root| OrgDocument::new(root.document.clone(), root.body.clone()))
            .collect::<Vec<_>>();
        self.load_documents_with_discovered_contracts(&documents)?;
        self.query_tool_capability_graph(receipt_id, request)
    }
}

fn project_memory_context_fact(
    query_match: marlin_agent_protocol::GraphQueryMatch,
    nodes: &BTreeMap<OrgNodeId, OrgNode>,
) -> ProjectMemoryContextFact {
    let claim = query_match.summary.clone();
    let memory_id = query_match
        .memory_id
        .as_ref()
        .map(|memory_id| memory_id.as_str().to_owned());
    let source_span = query_match
        .source_anchor_id
        .as_ref()
        .and_then(|source_anchor_id| project_memory_source_span(nodes, source_anchor_id.as_str()));
    let mut fact = ProjectMemoryContextFact::new(query_match, claim);
    if let Some(source_span) = source_span {
        fact = fact.with_source_span(source_span);
    }
    if let Some(memory_id) = memory_id {
        fact = fact.with_evidence(memory_id);
    }
    fact
}

fn project_memory_source_span(
    nodes: &BTreeMap<OrgNodeId, OrgNode>,
    source_anchor_id: &str,
) -> Option<String> {
    nodes
        .get(&OrgNodeId::new(source_anchor_id))
        .and_then(|node| node.source.as_ref())
        .map(render_org_source_span)
}

fn render_org_source_span(source: &OrgSourceSpan) -> String {
    format!(
        "{}:L{}-L{}",
        source.document, source.start_line, source.end_line
    )
}

impl Default for MemoryOrgWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentWorkspace for MemoryOrgWorkspace {
    async fn query(
        &self,
        query: WorkspaceQuery,
        _ctx: WorkspaceCtx,
    ) -> WorkspaceResult<WorkspaceQueryResult> {
        let nodes = self.read_nodes()?;
        let mut candidates = query::scope_nodes(&nodes, &query.scope);
        candidates.retain(|node| {
            query
                .filters
                .iter()
                .all(|filter| query::matches_filter(node, filter))
        });
        query::order_nodes(&mut candidates, &query.order);

        let truncated = query.limit.is_some_and(|limit| candidates.len() > limit);
        if let Some(limit) = query.limit {
            candidates.truncate(limit);
        }

        Ok(WorkspaceQueryResult {
            matches: candidates
                .into_iter()
                .map(|node| QueryMatch {
                    node_id: node.id.clone(),
                    node: Some(node.clone()),
                    score: Some(1),
                    reason: Some(query::match_reason(node)),
                })
                .collect(),
            truncated,
        })
    }

    async fn read_node(&self, id: OrgNodeId, _ctx: WorkspaceCtx) -> WorkspaceResult<OrgNode> {
        self.read_nodes()?
            .get(&id)
            .cloned()
            .ok_or_else(|| WorkspaceError::NodeNotFound(id.as_str().to_string()))
    }

    async fn patch(
        &self,
        patch: WorkspacePatch,
        _ctx: WorkspaceCtx,
    ) -> WorkspaceResult<WorkspacePatchReceipt> {
        let mut nodes = self.nodes.write();
        let receipt = patch::apply_workspace_patch(&mut nodes, patch)?;
        *self.last_patch_receipt.write() = Some(receipt.clone());
        Ok(receipt)
    }

    async fn render_view(
        &self,
        view: WorkspaceViewSpec,
        _ctx: WorkspaceCtx,
    ) -> WorkspaceResult<RenderedWorkspaceView> {
        let nodes = self.read_nodes()?;
        let contract_facts = self.read_contract_facts()?;
        let mut rendered_nodes = Vec::new();
        let mut text = String::new();

        for root in &view.roots {
            if let Some(node) = nodes.get(root) {
                let lines = render::render_node_lines(node, &view);
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&lines.join("\n"));
                rendered_nodes.push(RenderedViewNode {
                    node_id: node.id.clone(),
                    title: node.title.clone(),
                    lines,
                });
            }
        }

        let selected_contract_facts = if render::includes(&view, WorkspaceField::ContractFacts) {
            if !contract_facts.rendered_lines.is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&contract_facts.rendered_lines.join("\n"));
            }
            Some(contract_facts.clone())
        } else {
            None
        };

        let token_estimate = text.split_whitespace().count();
        Ok(RenderedWorkspaceView {
            spec_hash: render::view_hash(&view),
            token_estimate,
            nodes: rendered_nodes,
            contract_facts: selected_contract_facts,
            text,
        })
    }

    async fn validate(
        &self,
        _scope: WorkspaceScope,
        _ctx: WorkspaceCtx,
    ) -> WorkspaceResult<WorkspaceValidationReport> {
        Ok(WorkspaceValidationReport::accepted())
    }

    async fn status(
        &self,
        target: WorkspaceTarget,
        _ctx: WorkspaceCtx,
    ) -> WorkspaceResult<WorkspaceStatusReport> {
        let nodes = self.read_nodes()?;
        let contract_facts = self.read_contract_facts()?;
        let last_patch_receipt = self.read_last_patch_receipt()?;
        let release_status = self.read_release_status()?;
        let target_node = status::target_node(&nodes, &target);
        Ok(status::status_for_node(
            target_node,
            &contract_facts,
            last_patch_receipt.as_ref(),
            release_status,
        ))
    }
}

impl MemoryOrgWorkspace {
    fn read_nodes(&self) -> WorkspaceResult<RwLockReadGuard<'_, BTreeMap<OrgNodeId, OrgNode>>> {
        Ok(self.nodes.read())
    }

    fn read_contract_facts(&self) -> WorkspaceResult<RwLockReadGuard<'_, RenderedContractFacts>> {
        Ok(self.contract_facts.read())
    }

    fn read_last_patch_receipt(&self) -> WorkspaceResult<Option<WorkspacePatchReceipt>> {
        Ok(self.last_patch_receipt.read().clone())
    }

    fn read_release_status(&self) -> WorkspaceResult<Option<ReleaseStatus>> {
        Ok(self.release_status.read().clone())
    }
}

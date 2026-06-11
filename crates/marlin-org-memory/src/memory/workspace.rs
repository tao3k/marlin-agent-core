//! `MemoryOrgWorkspace` owner for the in-memory `Org` workspace backend.

use super::{contracts, patch, query, render, status};
use async_trait::async_trait;
use marlin_gerbil_ir::ReleaseTopologySpec;
use marlin_org_model::{OrgNode, OrgNodeId};
use marlin_org_workspace::{OrgDocument, OrgDocumentLoader, OrgDocumentWorkspace};
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
use std::collections::BTreeMap;
use std::sync::RwLock;

/// In-memory implementation of the native `AgentWorkspace` protocol.
pub struct MemoryOrgWorkspace {
    nodes: RwLock<BTreeMap<OrgNodeId, OrgNode>>,
    contract_facts: RwLock<RenderedContractFacts>,
    last_patch_receipt: RwLock<Option<WorkspacePatchReceipt>>,
    release_status: RwLock<Option<ReleaseStatus>>,
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
        let mut nodes = self
            .nodes
            .write()
            .map_err(|error| WorkspaceError::Backend(error.to_string()))?;
        nodes.insert(node.id.clone(), node);
        Ok(())
    }

    /// Record the latest release status projection visible through `status()`.
    pub fn record_release_status(&self, status: ReleaseStatus) -> WorkspaceResult<()> {
        *self
            .release_status
            .write()
            .map_err(|error| WorkspaceError::Backend(error.to_string()))? = Some(status);
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
        let mut release_status = self
            .release_status
            .write()
            .map_err(|error| WorkspaceError::Backend(error.to_string()))?;
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

    /// Load multiple `Org` documents and discover contract registry documents from the batch.
    pub fn load_documents_with_discovered_contracts(
        &self,
        documents: &[OrgDocument],
    ) -> WorkspaceResult<Vec<OrgNodeId>> {
        let contract_documents = OrgDocumentLoader::discover_contract_documents(documents);
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
            let mut nodes = self
                .nodes
                .write()
                .map_err(|error| WorkspaceError::Backend(error.to_string()))?;
            for node in workspace.nodes {
                nodes.insert(node.id.clone(), node);
            }
        }
        let incoming_contract_facts = contracts::contract_facts_from_workspace(
            workspace.contracts,
            workspace.contract_resolutions,
            workspace.contract_validations,
        );
        let mut contract_facts = self
            .contract_facts
            .write()
            .map_err(|error| WorkspaceError::Backend(error.to_string()))?;
        contracts::merge_contract_facts(&mut contract_facts, incoming_contract_facts);
        Ok(ids)
    }
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
        let mut nodes = self
            .nodes
            .write()
            .map_err(|error| WorkspaceError::Backend(error.to_string()))?;
        let receipt = patch::apply_workspace_patch(&mut nodes, patch)?;
        *self
            .last_patch_receipt
            .write()
            .map_err(|error| WorkspaceError::Backend(error.to_string()))? = Some(receipt.clone());
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
    fn read_nodes(
        &self,
    ) -> WorkspaceResult<std::sync::RwLockReadGuard<'_, BTreeMap<OrgNodeId, OrgNode>>> {
        self.nodes
            .read()
            .map_err(|error| WorkspaceError::Backend(error.to_string()))
    }

    fn read_contract_facts(
        &self,
    ) -> WorkspaceResult<std::sync::RwLockReadGuard<'_, RenderedContractFacts>> {
        self.contract_facts
            .read()
            .map_err(|error| WorkspaceError::Backend(error.to_string()))
    }

    fn read_last_patch_receipt(&self) -> WorkspaceResult<Option<WorkspacePatchReceipt>> {
        self.last_patch_receipt
            .read()
            .map(|receipt| receipt.clone())
            .map_err(|error| WorkspaceError::Backend(error.to_string()))
    }

    fn read_release_status(&self) -> WorkspaceResult<Option<ReleaseStatus>> {
        self.release_status
            .read()
            .map(|status| status.clone())
            .map_err(|error| WorkspaceError::Backend(error.to_string()))
    }
}

//! In-memory `Org` workspace backend for protocol tests and local agents.

use async_trait::async_trait;
use marlin_gerbil_ir::ReleaseTopologySpec;
use marlin_org_model::{
    CheckboxState, LinkKind, OrgCheckbox, OrgContractRegistry, OrgContractResolutionReport,
    OrgContractValidationReport, OrgLink, OrgNode, OrgNodeId, OrgNodeKind, TodoState,
};
use marlin_org_workspace::{OrgDocument, OrgDocumentLoader, OrgDocumentWorkspace};
use marlin_workspace_patch::{
    AffectedNodeSource, DecisionRecord, EvidenceRef, EvidenceTrust, MemoryDispatchReceipt,
    MetricPoint, PatchId, WorkspacePatch, WorkspacePatchExecutionMode,
    WorkspacePatchExecutionReceipt, WorkspacePatchOp, WorkspacePatchReceipt,
    WorkspaceValidationReport,
};
use marlin_workspace_protocol::{AgentWorkspace, WorkspaceCtx, WorkspaceError, WorkspaceResult};
use marlin_workspace_query::{
    PropertyFilter, QueryFilter, QueryMatch, QueryOrder, SourceRange, WorkspaceQuery,
    WorkspaceQueryResult, WorkspaceScope,
};
use marlin_workspace_status::{
    ChecklistStatus, ContractStatus, DecisionTrace, EvidenceStatus, GoalState, GoalStatus,
    MetricTrace, PatchExecutionMode, PatchStatus, ReleaseGateReceipt, ReleaseStatus, SddStatus,
    WorkspaceStatusReport, WorkspaceTarget,
};
use marlin_workspace_view::{
    RenderedContractFacts, RenderedViewNode, RenderedWorkspaceView, WorkspaceField,
    WorkspaceViewSpec,
};
use std::collections::{BTreeMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
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

    fn load_workspace(&self, workspace: OrgDocumentWorkspace) -> WorkspaceResult<Vec<OrgNodeId>> {
        let ids = workspace
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let mut nodes = self
            .nodes
            .write()
            .map_err(|error| WorkspaceError::Backend(error.to_string()))?;
        for node in workspace.nodes {
            nodes.insert(node.id.clone(), node);
        }
        *self
            .contract_facts
            .write()
            .map_err(|error| WorkspaceError::Backend(error.to_string()))? =
            contract_facts_from_workspace(
                workspace.contracts,
                workspace.contract_resolutions,
                workspace.contract_validations,
            );
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
        let mut candidates = self.scope_nodes(&nodes, &query.scope);
        candidates.retain(|node| {
            query
                .filters
                .iter()
                .all(|filter| matches_filter(node, filter))
        });
        order_nodes(&mut candidates, &query.order);

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
                    reason: Some(match_reason(node)),
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
        let before_hash = workspace_hash(&nodes);
        let mut affected_nodes = Vec::new();
        let mut memory_dispatch = Vec::new();

        for op in patch.ops {
            apply_op(&mut nodes, op, &mut affected_nodes, &mut memory_dispatch)?;
        }

        affected_nodes.sort();
        affected_nodes.dedup();
        let affected_sources = affected_sources(&nodes, &affected_nodes);
        let after_hash = workspace_hash(&nodes);

        let receipt = WorkspacePatchReceipt {
            patch_id: PatchId::new(format!("patch:{after_hash}")),
            affected_nodes,
            affected_sources,
            before_hash,
            after_hash,
            validation: WorkspaceValidationReport::accepted(),
            execution: WorkspacePatchExecutionReceipt::commit_accepted(
                "in-memory workspace patch applied",
            ),
            memory_dispatch,
        };
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
                let lines = render_node_lines(node, &view);
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

        let selected_contract_facts = if includes(&view, WorkspaceField::ContractFacts) {
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
            spec_hash: view_hash(&view),
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
        let target_node = target_node(&nodes, &target);
        Ok(status_for_node(
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

    fn scope_nodes<'a>(
        &self,
        nodes: &'a BTreeMap<OrgNodeId, OrgNode>,
        scope: &WorkspaceScope,
    ) -> Vec<&'a OrgNode> {
        match scope {
            WorkspaceScope::WholeWorkspace => nodes.values().collect(),
            WorkspaceScope::Document(path) => nodes
                .values()
                .filter(|node| node_in_document(node, path))
                .collect(),
            WorkspaceScope::SourceRange(range) => nodes
                .values()
                .filter(|node| node_in_source_range(node, range))
                .collect(),
            WorkspaceScope::Subtree(root) => {
                let mut ids = HashSet::new();
                collect_subtree_ids(nodes, root, &mut ids);
                nodes
                    .values()
                    .filter(|node| ids.contains(&node.id))
                    .collect()
            }
            WorkspaceScope::Nodes(ids) => ids.iter().filter_map(|id| nodes.get(id)).collect(),
        }
    }
}

fn affected_sources(
    nodes: &BTreeMap<OrgNodeId, OrgNode>,
    affected_nodes: &[OrgNodeId],
) -> Vec<AffectedNodeSource> {
    affected_nodes
        .iter()
        .filter_map(|node_id| {
            nodes.get(node_id).and_then(|node| {
                node.source.clone().map(|source| AffectedNodeSource {
                    node: node_id.clone(),
                    source,
                    tokens: node.tokens.clone(),
                })
            })
        })
        .collect()
}

fn apply_op(
    nodes: &mut BTreeMap<OrgNodeId, OrgNode>,
    op: WorkspacePatchOp,
    affected_nodes: &mut Vec<OrgNodeId>,
    memory_dispatch: &mut Vec<MemoryDispatchReceipt>,
) -> WorkspaceResult<()> {
    match op {
        WorkspacePatchOp::SetTodo { node, state } => {
            let target = node_mut(nodes, &node)?;
            target.todo = Some(state);
            affected_nodes.push(node);
        }
        WorkspacePatchOp::SetProperty { node, key, value } => {
            let target = node_mut(nodes, &node)?;
            target.properties.insert(key, value);
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddCheckbox { node, text, state } => {
            let target = node_mut(nodes, &node)?;
            target.checkboxes.push(OrgCheckbox::new(text, state));
            affected_nodes.push(node);
        }
        WorkspacePatchOp::MarkCheckbox { node, index, state } => {
            let target = node_mut(nodes, &node)?;
            let checkbox = target.checkboxes.get_mut(index).ok_or_else(|| {
                WorkspaceError::PatchRejected(format!(
                    "checkbox index {index} not found on {}",
                    node.as_str()
                ))
            })?;
            checkbox.state = state;
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AppendSection {
            node,
            heading,
            body,
        } => {
            let child_id =
                OrgNodeId::new(format!("{}/section/{}", node.as_str(), stable_id(&heading)));
            {
                let parent = node_mut(nodes, &node)?;
                parent.children.push(child_id.clone());
            }
            let mut child = OrgNode::heading(child_id.clone(), heading);
            child.body = Some(body);
            nodes.insert(child_id, child);
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddLink { node, link } => {
            let target = node_mut(nodes, &node)?;
            target.links.push(link);
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddEvidenceRef { node, evidence } => {
            let target = node_mut(nodes, &node)?;
            target.links.push(evidence_link(&evidence));
            target.properties.insert(
                "LAST_EVIDENCE_TRUST".to_string(),
                evidence_trust(&evidence.trust),
            );
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddMetricPoint { node, metric } => {
            let target = node_mut(nodes, &node)?;
            target
                .properties
                .insert(metric_key(&metric), metric.value.to_string());
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddDecision { node, decision } => {
            let target = node_mut(nodes, &node)?;
            target.properties.insert(
                format!("DECISION_{}", target.properties.len()),
                decision_text(&decision),
            );
            affected_nodes.push(node);
        }
        WorkspacePatchOp::AddTraceEvent { node, body } => {
            let target = node_mut(nodes, &node)?;
            target
                .properties
                .insert(format!("TRACE_{}", target.properties.len()), body);
            affected_nodes.push(node);
        }
        WorkspacePatchOp::MarkMemoryCandidate { node, dispatch } => {
            let target = node_mut(nodes, &node)?;
            target
                .properties
                .insert("MEMORY_DISPATCH".to_string(), dispatch.clone());
            memory_dispatch.push(MemoryDispatchReceipt {
                target: dispatch,
                accepted: true,
                reason: Some("memory candidate marked".to_string()),
            });
            affected_nodes.push(node);
        }
    }

    Ok(())
}

fn node_mut<'a>(
    nodes: &'a mut BTreeMap<OrgNodeId, OrgNode>,
    id: &OrgNodeId,
) -> WorkspaceResult<&'a mut OrgNode> {
    nodes
        .get_mut(id)
        .ok_or_else(|| WorkspaceError::NodeNotFound(id.as_str().to_string()))
}

fn matches_filter(node: &OrgNode, filter: &QueryFilter) -> bool {
    match filter {
        QueryFilter::FullText(term) => node_text(node).contains(&term.to_lowercase()),
        QueryFilter::Property(property) => matches_property(node, property),
        QueryFilter::Tag(tag) => node.tags.iter().any(|candidate| candidate == tag),
        QueryFilter::TodoState(state) => node
            .todo
            .as_ref()
            .is_some_and(|todo| todo_state(todo) == *state),
        QueryFilter::Kind(kind) => node_kind(&node.kind) == *kind,
        QueryFilter::OpenCheckbox => node
            .checkboxes
            .iter()
            .any(|checkbox| checkbox.state == CheckboxState::Open),
        QueryFilter::EvidenceLinked => node
            .links
            .iter()
            .any(|link| matches!(&link.kind, LinkKind::Custom(kind) if kind == "evidence")),
        QueryFilter::MemoryDispatch(dispatch) => {
            node.properties.get("MEMORY_DISPATCH") == Some(dispatch)
        }
        QueryFilter::SourceDocument(document) => node_in_document(node, document),
        QueryFilter::SourceRange(range) => node_in_source_range(node, range),
    }
}

fn node_in_document(node: &OrgNode, document: &str) -> bool {
    node.source
        .as_ref()
        .is_some_and(|source| source.document == document)
        || node
            .properties
            .get("DOCUMENT")
            .is_some_and(|value| value == document)
}

fn node_in_source_range(node: &OrgNode, range: &SourceRange) -> bool {
    node.source.as_ref().is_some_and(|source| {
        source.document == range.document
            && source.start_line <= range.end_line
            && source.end_line >= range.start_line
    })
}

fn matches_property(node: &OrgNode, property: &PropertyFilter) -> bool {
    match &property.value {
        Some(value) => node.properties.get(&property.key) == Some(value),
        None => node.properties.contains_key(&property.key),
    }
}

fn order_nodes(nodes: &mut Vec<&OrgNode>, order: &QueryOrder) {
    match order {
        QueryOrder::DocumentOrder | QueryOrder::RecentlyUpdated | QueryOrder::Priority => {}
        QueryOrder::Explicit(ids) => nodes.sort_by_key(|node| {
            ids.iter()
                .position(|id| id == node.id.as_str())
                .unwrap_or(usize::MAX)
        }),
    }
}

fn collect_subtree_ids(
    nodes: &BTreeMap<OrgNodeId, OrgNode>,
    current: &OrgNodeId,
    ids: &mut HashSet<OrgNodeId>,
) {
    if !ids.insert(current.clone()) {
        return;
    }
    if let Some(node) = nodes.get(current) {
        for child in &node.children {
            collect_subtree_ids(nodes, child, ids);
        }
    }
}

fn render_node_lines(node: &OrgNode, view: &WorkspaceViewSpec) -> Vec<String> {
    let mut lines = Vec::new();
    if includes(view, WorkspaceField::Title)
        && let Some(title) = &node.title
    {
        lines.push(format!("title: {title}"));
    }
    if includes(view, WorkspaceField::Todo)
        && let Some(todo) = &node.todo
    {
        lines.push(format!("todo: {}", todo_state(todo)));
    }
    if includes(view, WorkspaceField::SourceSpan)
        && let Some(source) = &node.source
    {
        lines.push(format!(
            "source: {}:{}-{} bytes={}..{}",
            source.document, source.start_line, source.end_line, source.start_byte, source.end_byte
        ));
    }
    if includes(view, WorkspaceField::SelectedProperties) {
        for (key, value) in &node.properties {
            lines.push(format!("property.{key}: {value}"));
        }
    }
    if includes(view, WorkspaceField::OpenCheckboxes) {
        for checkbox in &node.checkboxes {
            if checkbox.state == CheckboxState::Open {
                lines.push(format!("open: {}", checkbox.text));
            }
        }
    }
    if includes(view, WorkspaceField::EvidenceLinks) {
        for link in &node.links {
            if matches!(&link.kind, LinkKind::Custom(kind) if kind == "evidence") {
                let description = link
                    .description
                    .as_ref()
                    .map(|value| format!(" - {value}"))
                    .unwrap_or_default();
                lines.push(format!("evidence: {}{}", link.target, description));
            }
        }
    }
    if includes(view, WorkspaceField::Decisions) {
        for (key, value) in &node.properties {
            if key.starts_with("DECISION_") {
                lines.push(format!("decision: {value}"));
            }
        }
    }

    lines
}

fn contract_facts_from_workspace(
    registry: OrgContractRegistry,
    resolutions: OrgContractResolutionReport,
    validations: OrgContractValidationReport,
) -> RenderedContractFacts {
    let templates = registry
        .contracts
        .iter()
        .flat_map(|contract| contract.assertions.iter())
        .flat_map(|assertion| assertion.templates.iter().cloned())
        .collect();

    RenderedContractFacts::new(
        resolutions.references,
        resolutions.diagnostics,
        templates,
        validations,
    )
}

fn contract_status(contract_facts: &RenderedContractFacts) -> ContractStatus {
    let summary = &contract_facts.summary;

    ContractStatus {
        resolved_references: summary.resolved_references,
        unresolved_references: summary.unresolved_references,
        diagnostics: summary.diagnostics,
        templates: summary.templates,
        validation_receipts: summary.validation_receipts,
        validation_passed: summary.validation_passed,
        validation_failed: summary.validation_failed,
        validation_skipped: summary.validation_skipped,
        reference_resolutions: contract_facts.resolutions.clone(),
        diagnostic_records: contract_facts.diagnostics.clone(),
        template_records: contract_facts.templates.clone(),
        validation_report: contract_facts.validations.clone(),
        rendered_summary: contract_facts.rendered_lines.clone(),
    }
}

fn includes(view: &WorkspaceViewSpec, field: WorkspaceField) -> bool {
    view.include.contains(&field) && !view.exclude.contains(&field)
}

fn match_reason(node: &OrgNode) -> String {
    node.source
        .as_ref()
        .map(|source| {
            format!(
                "in-memory match at {}:{}-{}",
                source.document, source.start_line, source.end_line
            )
        })
        .unwrap_or_else(|| "in-memory match".to_string())
}

fn status_for_node(
    node: Option<&OrgNode>,
    contract_facts: &RenderedContractFacts,
    last_patch_receipt: Option<&WorkspacePatchReceipt>,
    release: Option<ReleaseStatus>,
) -> WorkspaceStatusReport {
    let contracts = Some(contract_status(contract_facts));
    let patch = last_patch_receipt.map(patch_status);
    let Some(node) = node else {
        return WorkspaceStatusReport {
            goal: None,
            sdd: None,
            checklist: None,
            evidence: None,
            contracts,
            patch,
            release,
            metrics: Vec::new(),
            decisions: DecisionTrace { recent: Vec::new() },
            next_actions: Vec::new(),
        };
    };

    let checklist = checklist_status(node);
    let evidence = evidence_status(node);
    WorkspaceStatusReport {
        goal: Some(GoalStatus {
            title: node
                .title
                .clone()
                .unwrap_or_else(|| node.id.as_str().to_string()),
            state: node
                .todo
                .as_ref()
                .map(goal_state)
                .unwrap_or(GoalState::Active),
            open_blockers: node
                .checkboxes
                .iter()
                .filter(|checkbox| checkbox.state == CheckboxState::Open)
                .map(|checkbox| checkbox.text.clone())
                .collect(),
        }),
        sdd: Some(SddStatus {
            title: node
                .title
                .clone()
                .unwrap_or_else(|| node.id.as_str().to_string()),
            accepted: checklist.open == 0,
            missing_evidence: usize::from(evidence.linked == 0),
        }),
        checklist: Some(checklist),
        evidence: Some(evidence),
        contracts,
        patch,
        release,
        metrics: metric_traces(node),
        decisions: decision_trace(node),
        next_actions: node
            .checkboxes
            .iter()
            .filter(|checkbox| checkbox.state == CheckboxState::Open)
            .map(|checkbox| checkbox.text.clone())
            .collect(),
    }
}

fn patch_status(receipt: &WorkspacePatchReceipt) -> PatchStatus {
    let mut affected_source_documents = receipt
        .affected_sources
        .iter()
        .map(|source| source.source.document.clone())
        .collect::<Vec<_>>();
    affected_source_documents.sort();
    affected_source_documents.dedup();
    PatchStatus {
        latest_patch_id: receipt.patch_id.as_str().to_string(),
        execution_mode: match receipt.execution.mode {
            WorkspacePatchExecutionMode::DryRun => PatchExecutionMode::DryRun,
            WorkspacePatchExecutionMode::Commit => PatchExecutionMode::Commit,
        },
        policy_accepted: receipt.execution.policy.accepted,
        policy_reason: receipt.execution.policy.reason.clone(),
        affected_nodes: receipt.affected_nodes.len(),
        affected_sources: receipt.affected_sources.len(),
        affected_source_documents,
        validation_accepted: receipt.validation.accepted,
        validation_diagnostics: receipt.validation.diagnostics.len(),
        memory_dispatches: receipt.memory_dispatch.len(),
        memory_dispatch_accepted: receipt
            .memory_dispatch
            .iter()
            .filter(|dispatch| dispatch.accepted)
            .count(),
        memory_dispatch_failed: receipt
            .memory_dispatch
            .iter()
            .filter(|dispatch| !dispatch.accepted)
            .count(),
    }
}

fn checklist_status(node: &OrgNode) -> ChecklistStatus {
    let done = node
        .checkboxes
        .iter()
        .filter(|checkbox| checkbox.state == CheckboxState::Checked)
        .count();
    let open = node
        .checkboxes
        .iter()
        .filter(|checkbox| checkbox.state == CheckboxState::Open)
        .count();
    let blocked = node
        .checkboxes
        .iter()
        .filter(|checkbox| checkbox.text.to_lowercase().contains("block"))
        .count();
    ChecklistStatus {
        done,
        open,
        blocked,
    }
}

fn evidence_status(node: &OrgNode) -> EvidenceStatus {
    let linked = node
        .links
        .iter()
        .filter(|link| matches!(&link.kind, LinkKind::Custom(kind) if kind == "evidence"))
        .count();
    let quarantined = usize::from(
        node.properties
            .get("LAST_EVIDENCE_TRUST")
            .is_some_and(|trust| trust == "quarantined"),
    );
    EvidenceStatus {
        linked,
        missing: usize::from(linked == 0),
        quarantined,
    }
}

fn metric_traces(node: &OrgNode) -> Vec<MetricTrace> {
    node.properties
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix("METRIC_").map(|name| MetricTrace {
                name: name.to_string(),
                latest: value.parse().ok(),
                target: None,
            })
        })
        .collect()
}

fn decision_trace(node: &OrgNode) -> DecisionTrace {
    DecisionTrace {
        recent: node
            .properties
            .iter()
            .filter(|(key, _value)| key.starts_with("DECISION_"))
            .map(|(_key, value)| value.clone())
            .collect(),
    }
}

fn target_node<'a>(
    nodes: &'a BTreeMap<OrgNodeId, OrgNode>,
    target: &WorkspaceTarget,
) -> Option<&'a OrgNode> {
    match target {
        WorkspaceTarget::Goal(id) | WorkspaceTarget::Sdd(id) | WorkspaceTarget::Checklist(id) => {
            nodes.get(id)
        }
        WorkspaceTarget::Workspace => nodes.values().next(),
    }
}

fn workspace_hash(nodes: &BTreeMap<OrgNodeId, OrgNode>) -> String {
    let mut hasher = DefaultHasher::new();
    format!("{nodes:?}").hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn view_hash(view: &WorkspaceViewSpec) -> String {
    let mut hasher = DefaultHasher::new();
    format!("{view:?}").hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn stable_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn node_text(node: &OrgNode) -> String {
    let mut text = String::new();
    if let Some(title) = &node.title {
        text.push_str(title);
    }
    if let Some(body) = &node.body {
        text.push(' ');
        text.push_str(body);
    }
    for value in node.properties.values() {
        text.push(' ');
        text.push_str(value);
    }
    text.to_lowercase()
}

fn node_kind(kind: &OrgNodeKind) -> String {
    format!("{kind:?}").to_lowercase()
}

fn todo_state(todo: &TodoState) -> String {
    match todo {
        TodoState::Todo => "TODO".to_string(),
        TodoState::Next => "NEXT".to_string(),
        TodoState::Wait => "WAIT".to_string(),
        TodoState::Blocked => "BLOCKED".to_string(),
        TodoState::Done => "DONE".to_string(),
        TodoState::Custom(value) => value.clone(),
    }
}

fn goal_state(todo: &TodoState) -> GoalState {
    match todo {
        TodoState::Todo => GoalState::Todo,
        TodoState::Next => GoalState::Next,
        TodoState::Wait => GoalState::Waiting,
        TodoState::Blocked => GoalState::Blocked,
        TodoState::Done => GoalState::Done,
        TodoState::Custom(value) => GoalState::Custom(value.clone()),
    }
}

fn evidence_link(evidence: &EvidenceRef) -> OrgLink {
    OrgLink {
        kind: LinkKind::Custom("evidence".to_string()),
        target: evidence.target.clone(),
        description: Some(evidence.summary.clone()),
    }
}

fn evidence_trust(trust: &EvidenceTrust) -> String {
    match trust {
        EvidenceTrust::Internal => "internal",
        EvidenceTrust::External => "external",
        EvidenceTrust::Quarantined => "quarantined",
        EvidenceTrust::Verified => "verified",
    }
    .to_string()
}

fn metric_key(metric: &MetricPoint) -> String {
    format!("METRIC_{}", metric.name)
}

fn decision_text(decision: &DecisionRecord) -> String {
    format!("{}: {}", decision.decision, decision.rationale)
}

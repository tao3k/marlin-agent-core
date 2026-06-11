use futures_executor::block_on;
use marlin_org_memory::MemoryOrgWorkspace;
use marlin_org_model::{CheckboxState, OrgNode, OrgNodeId, TodoState};
use marlin_org_workspace::OrgDocument;
use marlin_workspace_patch::{
    DecisionRecord, EvidenceRef, EvidenceTrust, MetricPoint, WorkspacePatch, WorkspacePatchOp,
};
use marlin_workspace_protocol::{AgentWorkspace, WorkspaceCtx};
use marlin_workspace_query::{
    PropertyFilter, QueryFilter, QueryOrder, SourceRange, WorkspaceQuery, WorkspaceScope,
};
use marlin_workspace_status::{GoalState, WorkspaceTarget};
use marlin_workspace_view::{WorkspaceField, WorkspaceViewSpec};

#[test]
fn memory_workspace_loads_document_for_query_and_view() {
    let workspace = MemoryOrgWorkspace::new();
    let document = OrgDocument::new(
        "doc:goal",
        "* TODO Build native workspace\n\
         :PROPERTIES:\n\
         :OWNER: marlin-org-memory\n\
         :END:\n\
         - [ ] Add file-backed adapter\n",
    );

    let ids = workspace
        .load_document(document)
        .expect("document inserted");
    let ctx = WorkspaceCtx::new("unit-test");
    let mut query = WorkspaceQuery::new(WorkspaceScope::WholeWorkspace);
    query
        .filters
        .push(QueryFilter::TodoState("TODO".to_string()));
    let query_result =
        futures_executor::block_on(workspace.query(query, ctx.clone())).expect("query succeeds");
    let view = futures_executor::block_on(workspace.render_view(
        WorkspaceViewSpec::compact(vec![ids[0].clone()]),
        ctx.clone(),
    ))
    .expect("view renders");

    assert_eq!(query_result.matches.len(), 1);
    assert!(
        query_result.matches[0]
            .reason
            .as_deref()
            .expect("query reason")
            .contains("doc:goal:1-")
    );
    assert!(view.text.contains("Build native workspace"));
    assert!(view.text.contains("Add file-backed adapter"));
    assert!(view.text.contains("source: doc:goal:1-"));

    let source_query = WorkspaceQuery::new(WorkspaceScope::SourceRange(SourceRange {
        document: "doc:goal".to_string(),
        start_line: 1,
        end_line: 1,
    }));
    let source_result = futures_executor::block_on(workspace.query(source_query, ctx.clone()))
        .expect("source query succeeds");
    assert_eq!(source_result.matches.len(), 1);

    let mut document_query = WorkspaceQuery::new(WorkspaceScope::WholeWorkspace);
    document_query
        .filters
        .push(QueryFilter::SourceDocument("doc:goal".to_string()));
    let document_result = futures_executor::block_on(workspace.query(document_query, ctx.clone()))
        .expect("document filter succeeds");
    assert_eq!(document_result.matches.len(), 1);

    let mut patch = WorkspacePatch::new("mark loaded document source");
    patch.ops.push(WorkspacePatchOp::SetProperty {
        node: ids[0].clone(),
        key: "STATUS".to_string(),
        value: "active".to_string(),
    });
    let receipt =
        futures_executor::block_on(workspace.patch(patch, WorkspaceCtx::new("unit-test")))
            .expect("patch succeeds");
    assert_eq!(receipt.affected_sources[0].node, ids[0]);
    assert_eq!(receipt.affected_sources[0].source.document, "doc:goal");
}

#[test]
fn memory_workspace_renders_loaded_contract_facts() {
    let workspace = MemoryOrgWorkspace::new();
    let document = OrgDocument::new(
        "doc:contracts",
        r#"* agent-task-v1
:PROPERTIES:
:CONTRACT_ID: agent.task.v1
:CONTRACT_SCOPE: subtree
:CONTRACT_KIND: org-elements
:END:

** must-have-goal-section
:PROPERTIES:
:ASSERT_ID: task.has-goal
:SEVERITY: error
:END:

#+BEGIN_SRC org-elements-query
category = "section"
kind = "headline"
within = "$scope"
summary.title = "Goal"
#+END_SRC

#+BEGIN_SRC org-elements-expect
count >= 1
#+END_SRC

#+BEGIN_SRC jinja2 :name message
Task `{{ scope.title }}` must contain a Goal section.
#+END_SRC

* TODO Task A
:PROPERTIES:
:CONTRACT_ORG: agent.task.v1
:END:
"#,
    );

    let ids = workspace
        .load_document(document)
        .expect("document inserted with contract facts");
    let ctx = WorkspaceCtx::new("unit-test");
    let view = block_on(workspace.render_view(
        WorkspaceViewSpec::compact(vec![ids[2].clone()]),
        ctx.clone(),
    ))
    .expect("view renders contract facts");

    let contract_facts = view.contract_facts.expect("contract facts selected");
    assert_eq!(contract_facts.resolutions.len(), 1);
    assert_eq!(contract_facts.templates.len(), 1);
    assert_eq!(contract_facts.summary.resolved_references, 1);
    assert_eq!(contract_facts.summary.templates, 1);
    assert!(
        contract_facts
            .rendered_lines
            .contains(&"contracts.validation_receipts: 0".to_string())
    );
    assert!(view.text.contains("contracts.resolved: 1"));
    assert!(view.text.contains("contracts.templates: 1"));

    let status = block_on(workspace.status(WorkspaceTarget::Goal(ids[2].clone()), ctx))
        .expect("status includes contract facts");
    let contracts = status.contracts.expect("contract status");
    assert_eq!(contracts.resolved_references, 1);
    assert_eq!(contracts.unresolved_references, 0);
    assert_eq!(contracts.templates, 1);
    assert_eq!(contracts.validation_receipts, 0);
    assert_eq!(contracts.validation_failed, 0);
    assert!(
        contracts
            .rendered_summary
            .contains(&"contracts.templates: 1".to_string())
    );
}

#[test]
fn memory_workspace_queries_patches_renders_and_reports_status() {
    let goal_id = OrgNodeId::from("goal:workspace");
    let mut goal = OrgNode::heading(goal_id.clone(), "Implement workspace backend");
    goal.properties
        .insert("OWNER".to_string(), "marlin-org-workspace".to_string());

    let workspace = MemoryOrgWorkspace::from_nodes(vec![goal]);
    let ctx = WorkspaceCtx::new("unit-test");

    let query = WorkspaceQuery {
        scope: WorkspaceScope::WholeWorkspace,
        filters: vec![QueryFilter::Property(PropertyFilter {
            key: "OWNER".to_string(),
            value: Some("marlin-org-workspace".to_string()),
        })],
        order: QueryOrder::DocumentOrder,
        limit: None,
    };
    let query_result = block_on(workspace.query(query, ctx.clone())).expect("query succeeds");
    assert_eq!(query_result.matches.len(), 1);

    let mut patch = WorkspacePatch::new("exercise native workspace protocol");
    patch.ops = vec![
        WorkspacePatchOp::SetTodo {
            node: goal_id.clone(),
            state: TodoState::Next,
        },
        WorkspacePatchOp::AddCheckbox {
            node: goal_id.clone(),
            text: "Add parser adapter later".to_string(),
            state: CheckboxState::Open,
        },
        WorkspacePatchOp::AddEvidenceRef {
            node: goal_id.clone(),
            evidence: EvidenceRef {
                target: "docs/20-workspace/20.30-org-workspace-backend.org".to_string(),
                summary: "Backend boundary doc".to_string(),
                trust: EvidenceTrust::Internal,
            },
        },
        WorkspacePatchOp::AddMetricPoint {
            node: goal_id.clone(),
            metric: MetricPoint {
                name: "query_latency_p95_ms".to_string(),
                value: 1.0,
                unit: Some("ms".to_string()),
            },
        },
        WorkspacePatchOp::AddDecision {
            node: goal_id.clone(),
            decision: DecisionRecord {
                decision: "Keep parser out of initial backend".to_string(),
                rationale: "Protocol semantics should stabilize first".to_string(),
            },
        },
        WorkspacePatchOp::MarkMemoryCandidate {
            node: goal_id.clone(),
            dispatch: "semantic-long-term".to_string(),
        },
    ];

    let receipt = block_on(workspace.patch(patch, ctx.clone())).expect("patch succeeds");
    assert_eq!(receipt.affected_nodes, vec![goal_id.clone()]);
    assert!(receipt.affected_sources.is_empty());
    assert_eq!(receipt.memory_dispatch[0].target, "semantic-long-term");
    assert_ne!(receipt.before_hash, receipt.after_hash);

    let mut view_spec = WorkspaceViewSpec::compact(vec![goal_id.clone()]);
    view_spec.include.push(WorkspaceField::Decisions);
    let view = block_on(workspace.render_view(view_spec, ctx.clone())).expect("view renders");
    assert!(view.text.contains("Implement workspace backend"));
    assert!(view.text.contains("Add parser adapter later"));
    assert!(view.text.contains("Backend boundary doc"));

    let status = block_on(workspace.status(WorkspaceTarget::Goal(goal_id), ctx)).expect("status");
    assert_eq!(status.goal.expect("goal status").state, GoalState::Next);
    assert_eq!(status.checklist.expect("checklist status").open, 1);
    assert_eq!(status.evidence.expect("evidence status").linked, 1);
}

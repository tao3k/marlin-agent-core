use futures_executor::block_on;
use marlin_org_memory::MemoryOrgWorkspace;
use marlin_org_workspace::OrgDocument;
use marlin_workspace_protocol::{AgentWorkspace, WorkspaceCtx};
use marlin_workspace_status::WorkspaceTarget;
use marlin_workspace_view::WorkspaceViewSpec;

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
    assert_eq!(contract_facts.registry.contracts.len(), 1);
    assert_eq!(contract_facts.resolutions.len(), 1);
    assert_eq!(contract_facts.templates.len(), 1);
    assert_eq!(contract_facts.summary.resolved_references, 1);
    assert_eq!(contract_facts.summary.contract_assertions, 1);
    assert_eq!(contract_facts.summary.templates, 1);
    assert_eq!(
        contract_facts.summary.contract_expectation_summaries,
        vec!["agent.task.v1/task.has-goal: count >= 1"]
    );
    let view_receipt = &contract_facts.validations.receipts[0];
    assert_eq!(
        view_receipt
            .source
            .as_ref()
            .map(|source| source.document.as_str()),
        Some("doc:contracts")
    );
    assert!(
        contract_facts
            .rendered_lines
            .contains(&"contracts.validation_receipts: 1".to_string())
    );
    assert!(contract_facts.rendered_lines.contains(
        &"contract.validation.expectation: agent.task.v1/task.has-goal: count >= 1".to_string()
    ));
    assert!(view.text.contains("contracts.resolved: 1"));
    assert!(view.text.contains("contracts.assertions: 1"));
    assert!(view.text.contains("contracts.templates: 1"));
    assert!(
        view.text
            .contains("contract.validation.expectation: agent.task.v1/task.has-goal: count >= 1")
    );
    assert!(
        view.text.contains("contracts.validation.failed: 1"),
        "{}",
        view.text
    );

    let status = block_on(workspace.status(WorkspaceTarget::Goal(ids[2].clone()), ctx))
        .expect("status includes contract facts");
    let contracts = status.contracts.expect("contract status");
    assert_eq!(contracts.resolved_references, 1);
    assert_eq!(contracts.unresolved_references, 0);
    assert_eq!(contracts.templates, 1);
    assert_eq!(contracts.contract_assertions, 1);
    assert_eq!(contracts.validation_receipts, 1);
    assert_eq!(contracts.validation_failed, 1);
    assert_eq!(contracts.validation_matched_nodes, 0);
    assert!(contracts.validation_matched_node_ids.is_empty());
    assert_eq!(contracts.reference_resolutions.len(), 1);
    assert_eq!(contracts.diagnostic_records.len(), 0);
    assert_eq!(contracts.registry.contracts.len(), 1);
    assert_eq!(
        contracts.contract_expectation_summaries,
        vec!["agent.task.v1/task.has-goal: count >= 1"]
    );
    assert_eq!(contracts.template_records.len(), 1);
    assert_eq!(contracts.validation_report.receipts.len(), 1);
    let status_receipt = &contracts.validation_report.receipts[0];
    assert!(status_receipt.matched_nodes.is_empty());
    assert_eq!(
        status_receipt
            .source
            .as_ref()
            .map(|source| source.document.as_str()),
        Some("doc:contracts")
    );
    assert!(
        contracts
            .rendered_summary
            .contains(&"contracts.templates: 1".to_string())
    );
    assert!(contracts.rendered_summary.contains(
        &"contract.validation.expectation: agent.task.v1/task.has-goal: count >= 1".to_string()
    ));
}

#[test]
fn memory_workspace_loads_external_contract_documents() {
    let workspace = MemoryOrgWorkspace::new();
    let contract_document = OrgDocument::new(
        "doc:external-contracts",
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
"#,
    );
    let task_document = OrgDocument::new(
        "doc:task",
        r#"* TODO Task A
:PROPERTIES:
:CONTRACT_ORG: agent.task.v1
:END:
"#,
    );

    let ids = workspace
        .load_document_with_contracts(task_document, &[contract_document])
        .expect("document inserted with external contract facts");
    let status = block_on(workspace.status(
        WorkspaceTarget::Goal(ids[0].clone()),
        WorkspaceCtx::new("unit-test"),
    ))
    .expect("status includes external contract facts");
    let contracts = status.contracts.expect("contract status");

    assert_eq!(contracts.resolved_references, 1);
    assert_eq!(contracts.unresolved_references, 0);
    assert_eq!(contracts.template_records.len(), 1);
    assert_eq!(contracts.validation_report.receipts.len(), 1);
    assert_eq!(contracts.validation_matched_nodes, 0);
    assert!(contracts.validation_matched_node_ids.is_empty());
    assert!(
        contracts.validation_report.receipts[0]
            .matched_nodes
            .is_empty()
    );
    assert_eq!(
        contracts.reference_resolutions[0]
            .resolved_contract_id
            .as_ref()
            .map(|contract_id| contract_id.as_str()),
        Some("agent.task.v1")
    );
}

#[test]
fn memory_workspace_loads_documents_with_discovered_contracts() {
    let workspace = MemoryOrgWorkspace::new();
    let contract_document = OrgDocument::new(
        "doc:external-contracts",
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
count == 1
#+END_SRC
"#,
    );
    let task_document = OrgDocument::new(
        "doc:task",
        r#"* TODO Task A
:PROPERTIES:
:CONTRACT_ORG: agent.task.v1
:END:
** Goal
"#,
    );
    let note_document = OrgDocument::new("doc:note", "* NOTE Plain note\n");

    let ids = workspace
        .load_documents_with_discovered_contracts(&[
            note_document,
            task_document,
            contract_document,
        ])
        .expect("documents loaded with discovered contracts");
    let task_id = ids
        .iter()
        .find(|id| id.as_str().starts_with("doc:task:1:task-a"))
        .expect("task id loaded")
        .clone();
    let ctx = WorkspaceCtx::new("unit-test");

    let view = block_on(workspace.render_view(
        WorkspaceViewSpec::compact(vec![task_id.clone()]),
        ctx.clone(),
    ))
    .expect("view renders discovered contract facts");
    assert!(view.text.contains("contracts.validation.passed: 1"));
    assert!(
        view.text
            .contains("contract.validation.matched_node: doc:task:5:goal"),
        "{}",
        view.text
    );

    let status = block_on(workspace.status(WorkspaceTarget::Goal(task_id), ctx))
        .expect("status includes discovered contract facts");
    let contracts = status.contracts.expect("contract status");
    assert_eq!(contracts.resolved_references, 1);
    assert_eq!(contracts.validation_receipts, 1);
    assert_eq!(contracts.validation_passed, 1);
    assert_eq!(contracts.validation_matched_nodes, 1);
    assert_eq!(
        contracts.validation_matched_node_ids[0].as_str(),
        "doc:task:5:goal"
    );
}

#[test]
fn memory_workspace_loads_document_with_standard_agent_contracts() {
    let workspace = MemoryOrgWorkspace::new();
    let document = OrgDocument::new(
        "doc:standard-memory",
        r#"* Agent memory candidate
:PROPERTIES:
:CONTRACT_ORG: agent.memory.v1
:ID: standard-memory-candidate
:MEMORY_KIND: workspace-status
:SOURCE_REF: crates/marlin-org-memory/tests/unit/memory/contracts.rs
:RECALL_QUERY: memory workspace standard contract status receipt
:SALIENCE: medium
:END:

** Claim
Memory workspace status should include standard contract validation receipts.

** Source
Workspace status receipt.

** Recall
- Recall when validating standard agent memory contract loading.

** Evidence
- MemoryOrgWorkspace loads standard contract documents and validates receipts.

** Trust
internal

** Retention
- Keep for project tests while standard memory contract loading exists.
"#,
    );

    let ids = workspace
        .load_document_with_standard_agent_contracts(document)
        .expect("document inserted with standard agent contracts");
    let status = block_on(workspace.status(
        WorkspaceTarget::Goal(ids[0].clone()),
        WorkspaceCtx::new("unit-test"),
    ))
    .expect("status includes standard agent contract facts");
    let contracts = status.contracts.expect("contract status");

    assert_eq!(contracts.resolved_references, 1);
    assert_eq!(contracts.unresolved_references, 0);
    assert_eq!(contracts.contract_assertions, 44);
    assert_eq!(contracts.validation_receipts, 14);
    assert_eq!(contracts.validation_passed, 14);
    assert_eq!(
        contracts.reference_resolutions[0]
            .resolved_contract_id
            .as_ref()
            .map(|contract_id| contract_id.as_str()),
        Some("agent.memory.v1")
    );
    assert!(
        contracts
            .contract_expectation_summaries
            .contains(&"agent.memory.v1/memory.has-retention: count >= 1".to_owned())
    );
}

#[test]
fn memory_workspace_status_reports_unresolved_contract_diagnostics() {
    let workspace = MemoryOrgWorkspace::new();
    let document = OrgDocument::new(
        "doc:missing-contract",
        "#+CONTRACT_ORG: missing.contract\n* TODO Task A\n",
    );

    workspace
        .load_document(document)
        .expect("document inserted with unresolved contract facts");
    let status =
        block_on(workspace.status(WorkspaceTarget::Workspace, WorkspaceCtx::new("unit-test")))
            .expect("workspace status includes unresolved contract diagnostics");
    let contracts = status.contracts.expect("contract status");

    assert_eq!(contracts.resolved_references, 0);
    assert_eq!(contracts.unresolved_references, 1);
    assert_eq!(contracts.diagnostic_records.len(), 1);
    assert_eq!(contracts.diagnostic_records[0].code, "ORG044");
    assert!(
        contracts.diagnostic_records[0]
            .message
            .contains("missing.contract")
    );
    assert_eq!(contracts.reference_resolutions.len(), 1);
    assert_eq!(
        contracts.reference_resolutions[0]
            .reference
            .contract_id
            .as_ref()
            .map(|contract_id| contract_id.as_str()),
        Some("missing.contract")
    );
}

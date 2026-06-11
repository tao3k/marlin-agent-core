use futures_executor::block_on;
use marlin_org_memory::MemoryOrgWorkspace;
use marlin_org_workspace::OrgDocument;
use marlin_workspace_protocol::{AgentWorkspace, WorkspaceCtx};
use marlin_workspace_status::WorkspaceTarget;
use marlin_workspace_view::WorkspaceViewSpec;

#[test]
fn sequential_document_loads_accumulate_contract_facts() {
    let workspace = MemoryOrgWorkspace::new();
    workspace
        .load_document(contract_document(
            "doc:one",
            "agent.one.v1",
            "Task One",
            "Goal",
        ))
        .expect("first document loaded");
    let second_ids = workspace
        .load_document(contract_document(
            "doc:two",
            "agent.two.v1",
            "Task Two",
            "Outcome",
        ))
        .expect("second document loaded");
    let second_task = second_ids
        .iter()
        .find(|id| id.as_str().contains("task-two"))
        .expect("second task id loaded")
        .clone();
    let ctx = WorkspaceCtx::new("unit-test");

    let status = block_on(workspace.status(WorkspaceTarget::Workspace, ctx.clone()))
        .expect("workspace status includes accumulated contract facts");
    let contracts = status.contracts.expect("contract status");

    assert_eq!(contracts.registry.contracts.len(), 2);
    assert_eq!(contracts.resolved_references, 2);
    assert_eq!(contracts.validation_receipts, 2);
    assert_eq!(contracts.validation_passed, 2);
    assert!(
        contracts
            .contract_expectation_summaries
            .contains(&"agent.one.v1/task.has-required-child: count == 1".to_string())
    );
    assert!(
        contracts
            .contract_expectation_summaries
            .contains(&"agent.two.v1/task.has-required-child: count == 1".to_string())
    );

    let view = block_on(workspace.render_view(WorkspaceViewSpec::compact(vec![second_task]), ctx))
        .expect("view renders accumulated contract facts");

    assert!(view.text.contains("contracts.resolved: 2"), "{}", view.text);
    assert!(
        view.text.contains(
            "contract.validation.expectation: agent.one.v1/task.has-required-child: count == 1"
        ),
        "{}",
        view.text
    );
    assert!(
        view.text.contains(
            "contract.validation.expectation: agent.two.v1/task.has-required-child: count == 1"
        ),
        "{}",
        view.text
    );
}

fn contract_document(
    document_id: &str,
    contract_id: &str,
    task_title: &str,
    required_child_title: &str,
) -> OrgDocument {
    OrgDocument::new(
        document_id,
        format!(
            r#"* {contract_id}
:PROPERTIES:
:CONTRACT_ID: {contract_id}
:CONTRACT_SCOPE: subtree
:CONTRACT_KIND: org-elements
:END:

** must-have-required-child
:PROPERTIES:
:ASSERT_ID: task.has-required-child
:SEVERITY: error
:END:

#+BEGIN_SRC org-elements-query
category = "section"
kind = "headline"
within = "$scope"
summary.title = "{required_child_title}"
#+END_SRC

#+BEGIN_SRC org-elements-expect
count == 1
#+END_SRC

* TODO {task_title}
:PROPERTIES:
:CONTRACT_ORG: {contract_id}
:END:
** {required_child_title}
"#
        ),
    )
}

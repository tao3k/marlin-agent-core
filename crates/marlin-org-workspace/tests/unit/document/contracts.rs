use marlin_org_model::{
    OrgContractReferenceScope, OrgContractValidationStatus, OrgContractValidationTarget,
};
use marlin_org_workspace::{OrgDocument, OrgDocumentLoader};

use super::slice;

#[test]
fn org_document_loader_resolves_subtree_contract_references() {
    let text = r#"* agent-task-v1
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

* TODO Task A
:PROPERTIES:
:CONTRACT_ORG: agent.task.v1
:END:
"#;
    let document = OrgDocument::new("doc:resolved-contract", text);

    let workspace = OrgDocumentLoader::load_workspace(&document).expect("document loads");

    assert!(workspace.contract_resolutions.diagnostics.is_empty());
    assert_eq!(workspace.contract_validations.receipts.len(), 1);
    let receipt = &workspace.contract_validations.receipts[0];
    assert_eq!(receipt.contract_id.as_str(), "agent.task.v1");
    assert_eq!(receipt.assertion_id, "task.has-goal");
    assert_eq!(receipt.status, OrgContractValidationStatus::Failed);
    assert!(matches!(
        receipt.target,
        OrgContractValidationTarget::Node(_)
    ));
    assert_eq!(workspace.contract_resolutions.references.len(), 1);
    let resolution = &workspace.contract_resolutions.references[0];
    assert_eq!(
        resolution
            .resolved_contract_id
            .as_ref()
            .map(|contract_id| contract_id.as_str()),
        Some("agent.task.v1")
    );
    assert_eq!(resolution.reference.raw, "agent.task.v1");
    assert_eq!(
        resolution.reference.scope,
        OrgContractReferenceScope::Subtree
    );
    assert!(resolution.reference.target_node.is_some());
    let source = resolution.reference.source.as_ref().expect("source span");
    assert_eq!(slice(text, source), "agent.task.v1");
}

#[test]
fn org_document_loader_validates_subtree_contract_references() {
    let text = r#"* agent-task-v1
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

* TODO Task A
:PROPERTIES:
:CONTRACT_ORG: agent.task.v1
:END:

** Goal
"#;
    let document = OrgDocument::new("doc:validated-contract", text);

    let workspace = OrgDocumentLoader::load_workspace(&document).expect("document loads");

    assert!(workspace.contract_validations.diagnostics.is_empty());
    assert_eq!(workspace.contract_validations.receipts.len(), 1);
    let receipt = &workspace.contract_validations.receipts[0];
    assert_eq!(receipt.contract_id.as_str(), "agent.task.v1");
    assert_eq!(receipt.assertion_id, "task.has-goal");
    assert_eq!(receipt.status, OrgContractValidationStatus::Passed);
    let source = receipt.source.as_ref().expect("validated source span");
    assert!(slice(text, source).contains("* TODO Task A"));
    assert_eq!(receipt.matched_nodes.len(), 1);
}

#[test]
fn org_document_loader_validates_orgize_count_comparison_expectations() {
    let text = r#"* agent-task-v1
:PROPERTIES:
:CONTRACT_ID: agent.task.v1
:CONTRACT_SCOPE: subtree
:CONTRACT_KIND: org-elements
:END:

** exactly-one-goal-section
:PROPERTIES:
:ASSERT_ID: task.one-goal
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

** no-risk-section
:PROPERTIES:
:ASSERT_ID: task.no-risk
:SEVERITY: warning
:END:

#+BEGIN_SRC org-elements-query
category = "section"
kind = "headline"
within = "$scope"
summary.title = "Risk"
#+END_SRC

#+BEGIN_SRC org-elements-expect
not exists
#+END_SRC

** impossible-upper-bound
:PROPERTIES:
:ASSERT_ID: task.no-goal
:SEVERITY: warning
:END:

#+BEGIN_SRC org-elements-query
category = "section"
kind = "headline"
within = "$scope"
summary.title = "Goal"
#+END_SRC

#+BEGIN_SRC org-elements-expect
count < 1
#+END_SRC

* TODO Task A
:PROPERTIES:
:CONTRACT_ORG: agent.task.v1
:END:

** Goal
"#;
    let document = OrgDocument::new("doc:count-contract", text);

    let workspace = OrgDocumentLoader::load_workspace(&document).expect("document loads");

    assert_eq!(workspace.contract_validations.receipts.len(), 3);
    let one_goal = workspace
        .contract_validations
        .receipts
        .iter()
        .find(|receipt| receipt.assertion_id == "task.one-goal")
        .expect("one-goal receipt");
    assert_eq!(one_goal.status, OrgContractValidationStatus::Passed);
    assert_eq!(one_goal.matched_nodes.len(), 1);

    let no_risk = workspace
        .contract_validations
        .receipts
        .iter()
        .find(|receipt| receipt.assertion_id == "task.no-risk")
        .expect("no-risk receipt");
    assert_eq!(no_risk.status, OrgContractValidationStatus::Passed);
    assert!(no_risk.matched_nodes.is_empty());

    let no_goal = workspace
        .contract_validations
        .receipts
        .iter()
        .find(|receipt| receipt.assertion_id == "task.no-goal")
        .expect("no-goal receipt");
    assert_eq!(no_goal.status, OrgContractValidationStatus::Failed);
    assert_eq!(no_goal.matched_nodes.len(), 1);
}

#[test]
fn org_document_loader_resolves_references_from_external_contract_documents() {
    let contract_text = r#"* external-agent-task
:PROPERTIES:
:CONTRACT_ID: external.agent.task
:CONTRACT_SCOPE: subtree
:CONTRACT_KIND: org-elements
:END:
"#;
    let task_text = r#"* TODO Task A
:PROPERTIES:
:CONTRACT_ORG: external.agent.task
:END:
"#;
    let contract_document = OrgDocument::new("doc:external-contracts", contract_text);
    let task_document = OrgDocument::new("doc:task", task_text);

    let workspace =
        OrgDocumentLoader::load_workspace_with_contracts(&task_document, &[contract_document])
            .expect("document loads with external contracts");

    assert!(workspace.contract_resolutions.diagnostics.is_empty());
    assert_eq!(workspace.contracts.contracts.len(), 1);
    let resolution = &workspace.contract_resolutions.references[0];
    assert_eq!(
        resolution
            .resolved_contract_id
            .as_ref()
            .map(|contract_id| contract_id.as_str()),
        Some("external.agent.task")
    );
}

#[test]
fn org_document_loader_discovers_contract_documents_from_candidates() {
    let contract_document = OrgDocument::new(
        "doc:external-contracts",
        r#"* external-agent-task
:PROPERTIES:
:CONTRACT_ID: external.agent.task
:CONTRACT_SCOPE: subtree
:CONTRACT_KIND: org-elements
:END:
"#,
    );
    let task_document = OrgDocument::new(
        "doc:task",
        r#"* TODO Task A
:PROPERTIES:
:CONTRACT_ORG: external.agent.task
:END:
"#,
    );
    let note_document = OrgDocument::new("doc:note", "* NOTE Plain note\n");

    let discovered = OrgDocumentLoader::discover_contract_documents(&[
        task_document.clone(),
        contract_document,
        note_document,
    ]);
    let workspace = OrgDocumentLoader::load_workspace_with_contracts(&task_document, &discovered)
        .expect("document loads with discovered contract documents");

    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].id.as_str(), "doc:external-contracts");
    assert!(workspace.contract_resolutions.diagnostics.is_empty());
    assert_eq!(
        workspace.contract_resolutions.references[0]
            .resolved_contract_id
            .as_ref()
            .map(|contract_id| contract_id.as_str()),
        Some("external.agent.task")
    );
}

#[test]
fn org_document_loader_reports_unresolved_document_contract_references() {
    let text = "#+CONTRACT_ORG: missing.contract\n* Task A\n";
    let document = OrgDocument::new("doc:missing-contract", text);

    let workspace = OrgDocumentLoader::load_workspace(&document).expect("document loads");

    assert_eq!(workspace.contract_resolutions.references.len(), 1);
    let resolution = &workspace.contract_resolutions.references[0];
    assert_eq!(resolution.resolved_contract_id, None);
    assert_eq!(resolution.reference.raw, "missing.contract");
    assert_eq!(
        resolution.reference.scope,
        OrgContractReferenceScope::Document
    );
    let diagnostic = workspace
        .contract_resolutions
        .diagnostics
        .first()
        .expect("unresolved diagnostic");
    assert_eq!(diagnostic.code, "ORG044");
    assert!(diagnostic.message.contains("missing.contract"));
    let source = diagnostic.reference.source.as_ref().expect("source span");
    assert_eq!(slice(text, source), "#+CONTRACT_ORG: missing.contract\n");
}

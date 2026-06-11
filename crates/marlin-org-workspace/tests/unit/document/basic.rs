use marlin_org_model::{CheckboxState, LinkKind, OrgContractTemplateKind, TodoState};
use marlin_org_workspace::{OrgDocument, OrgDocumentLoader};

use super::slice;

#[test]
fn org_document_loader_projects_headings_properties_tasks_and_links() {
    let text = "* NEXT Implement parser adapter :marlin:workspace:\n\
                :PROPERTIES:\n\
                :OWNER: marlin-org-workspace\n\
                :END:\n\
                - [ ] Preserve evidence links\n\
                See [[file:docs/20-workspace/20.30-org-workspace-backend.org][backend doc]].\n\
                ** DONE Child task\n\
                - [X] covered\n";
    let document = OrgDocument::new("doc:workspace", text);

    let nodes = OrgDocumentLoader::load(&document).expect("document loads");

    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].todo, Some(TodoState::Next));
    assert_eq!(nodes[0].tags, vec!["marlin", "workspace"]);
    assert_eq!(
        nodes[0].properties.get("OWNER").map(String::as_str),
        Some("marlin-org-workspace")
    );
    assert_eq!(nodes[0].checkboxes[0].state, CheckboxState::Open);
    assert_eq!(nodes[0].links[0].kind, LinkKind::File);
    assert_eq!(nodes[0].children, vec![nodes[1].id.clone()]);
    let source = nodes[0].source.as_ref().expect("source span");
    assert_eq!(source.document, "doc:workspace");
    assert_eq!(source.start_line, 1);
    let todo_span = nodes[0]
        .tokens
        .todo_keyword
        .as_ref()
        .expect("todo keyword token span");
    assert_eq!(slice(text, todo_span), "NEXT");
    let owner_span = nodes[0]
        .tokens
        .property_values
        .get("OWNER")
        .expect("property value token span");
    assert_eq!(slice(text, owner_span), "marlin-org-workspace");
    let checkbox_span = nodes[0]
        .tokens
        .checkbox_markers
        .first()
        .expect("checkbox marker token span");
    assert_eq!(slice(text, checkbox_span), " ");
    assert_eq!(
        nodes[1]
            .source
            .as_ref()
            .expect("child source span")
            .start_line,
        7
    );
    assert_eq!(
        slice(
            text,
            nodes[1]
                .tokens
                .todo_keyword
                .as_ref()
                .expect("child todo keyword token span")
        ),
        "DONE"
    );
    assert_eq!(
        slice(
            text,
            nodes[1]
                .tokens
                .checkbox_markers
                .first()
                .expect("child checkbox marker token span")
        ),
        "X"
    );
}

#[test]
fn org_document_loader_projects_contract_registry() {
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

#+BEGIN_SRC jinja2 :name message
Task `{{ scope.title }}` must contain a Goal section.
#+END_SRC
"#;
    let document = OrgDocument::new("doc:contracts", text);

    let workspace = OrgDocumentLoader::load_workspace(&document).expect("document loads");

    assert_eq!(workspace.contracts.contracts.len(), 1);
    assert!(workspace.contract_validations.receipts.is_empty());
    assert!(workspace.contract_validations.diagnostics.is_empty());
    let contract = &workspace.contracts.contracts[0];
    assert_eq!(contract.id.as_str(), "agent.task.v1");
    assert_eq!(contract.scope.as_str(), "Subtree");
    assert_eq!(contract.kind.as_str(), "OrgElementsAssertions");
    assert_eq!(contract.assertions.len(), 1);
    let assertion = &contract.assertions[0];
    assert_eq!(assertion.id, "task.has-goal");
    assert_eq!(assertion.severity.as_str(), "Error");
    assert!(assertion.expectation.as_str().contains("Count"));
    assert_eq!(
        assertion.message.as_deref(),
        Some("Task `{{ scope.title }}` must contain a Goal section.\n")
    );
    assert_eq!(assertion.templates.len(), 1);
    let template = &assertion.templates[0];
    assert_eq!(template.kind, OrgContractTemplateKind::Message);
    assert_eq!(template.engine.as_str(), "jinja2");
    assert_eq!(
        template.body,
        "Task `{{ scope.title }}` must contain a Goal section.\n"
    );
    assert_eq!(
        assertion
            .query
            .category
            .as_ref()
            .map(|category| category.as_str()),
        Some("Section")
    );
    assert!(
        assertion
            .query
            .kind
            .as_ref()
            .map(|kind| kind.as_str())
            .is_some_and(|kind| kind.contains("headline"))
    );
    assert_eq!(
        assertion.query.summary_equals,
        vec![("title".to_string(), "Goal".to_string())]
    );
    assert!(assertion.query.use_scope_outline_path);
    let query_source = assertion.query_source.as_ref().expect("query source span");
    assert!(query_source.start_byte < query_source.end_byte);
}

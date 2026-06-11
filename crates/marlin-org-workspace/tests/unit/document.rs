use marlin_org_model::{CheckboxState, LinkKind, TodoState};
use marlin_org_workspace::{OrgDocument, OrgDocumentLoader};

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

fn slice<'a>(text: &'a str, span: &marlin_org_model::OrgSourceSpan) -> &'a str {
    &text[span.start_byte..span.end_byte]
}

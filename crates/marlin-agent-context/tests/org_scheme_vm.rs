// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "gerbil")]

use gerbil_scheme::GerbilRuntime;
use marlin_agent_context::gerbil_org::{
    ContextOrgError, ContextOrgModule, OrgContractSourceId, OrgFieldName, OrgFieldRequest,
    OrgFieldValue, OrgSourceGeneration,
};
use std::path::Path;

#[test]
#[ignore = "requires the selected Marlin Scheme package and complete Lab module closure"]
fn selected_lab_org_graph_preserves_source_and_utf8_byte_spans() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let runtime = GerbilRuntime::initialize().unwrap();
    let module = ContextOrgModule::resolve(&runtime, root.to_str().unwrap()).unwrap();
    let text = "* TODO Open :work:\n:PROPERTIES:\n:ID: stable\n:END:\n正文 中文\n* DONE Closed\n#+begin_src text\n* TODO not a headline\n#+end_src\n";
    let source = module.open(text).unwrap();
    let graph = module.project(&source).unwrap();
    assert_eq!(graph.source, text);
    let headlines: Vec<_> = graph
        .elements
        .iter()
        .filter(|item| item.kind.as_str() == "headline")
        .collect();
    assert_eq!(headlines.len(), 2);
    assert!(
        headlines
            .iter()
            .any(|item| graph.source[item.start_byte..item.end_byte].contains("正文 中文"))
    );
    let open = headlines
        .iter()
        .find(|item| graph.source[item.start_byte..item.end_byte].starts_with("* TODO Open"))
        .unwrap();
    let query = |element, name| OrgFieldRequest {
        element,
        name: OrgFieldName::new(name).unwrap(),
    };
    let requests = [
        query(open.id, "title"),
        query(open.id, "todo-keyword"),
        query(open.id, "tags"),
        query(open.id, "property:ID"),
        query(open.id, "missing-field"),
    ];
    assert_eq!(
        module.fields(&source, &requests).unwrap(),
        vec![
            OrgFieldValue::Text("Open".into()),
            OrgFieldValue::Text("TODO".into()),
            OrgFieldValue::TextList(vec!["work".into()]),
            OrgFieldValue::Text("stable".into()),
            OrgFieldValue::Missing,
        ]
    );
    let reopened = module.open(text).unwrap();
    let reopened_graph = module.project(&reopened).unwrap();
    let reopened_open = reopened_graph
        .elements
        .iter()
        .find(|item| {
            item.kind.as_str() == "headline"
                && reopened_graph.source[item.start_byte..item.end_byte].starts_with("* TODO Open")
        })
        .unwrap();
    assert_eq!(open.id.ordinal(), reopened_open.id.ordinal());
    assert_ne!(open.id, reopened_open.id);
    assert!(matches!(
        module.fields(&reopened, &[query(open.id, "title")]),
        Err(ContextOrgError::Invalid(
            "field request belongs to another Org source"
        ))
    ));
    assert_eq!(
        module
            .fields(&reopened, &[query(reopened_open.id, "title")])
            .unwrap(),
        vec![OrgFieldValue::Text("Open".into())]
    );
    assert!(
        module
            .fields(&source, &vec![requests[0].clone(); 65])
            .is_err()
    );
    let long_text = format!("* TODO Large\n{}\n", "x".repeat(40_000));
    let large = module.open(&long_text).unwrap();
    let large_graph = module.project(&large).unwrap();
    let root = large_graph
        .elements
        .iter()
        .find(|item| item.kind.as_str() == "org-data")
        .unwrap();
    let repeated = vec![query(root.id, "source-text"); 64];
    assert!(
        module.fields(&large, &repeated).is_err(),
        "repeated fields must honor the output byte limit"
    );
    assert!(module.open(&"x".repeat(1024 * 1024 + 1)).is_err());
    assert_eq!(module.project(&source).unwrap(), graph);

    // The selected Orgize implementation rejects an oversized graph while
    // preserving the prior rooted source. This exercises its parser limit.
    let oversized_graph = "* Item\n".repeat(4097);
    let Err(ContextOrgError::Call(failure)) = module.open(&oversized_graph) else {
        panic!("oversized graph must be rejected by the selected Scheme parser");
    };
    assert!(
        failure
            .diagnostic
            .as_deref()
            .is_some_and(|message| message.contains("Org graph exceeds 4096 Elements")),
        "{failure:?}"
    );
    assert_eq!(module.project(&source).unwrap(), graph);

    let contract_text = "#+title: Evidence\n:PROPERTIES:\n:ID: root\n:CONTRACT_ID: example.v1\n:CONTRACT_REVISION: 1\n:END:\n* TODO Work\n:PROPERTIES:\n:ID: work\n:END:\n完整中文内容\n";
    let identity = OrgContractSourceId::new("source-a").unwrap();
    let first = module
        .open_contract(
            &identity,
            OrgSourceGeneration::new(1).unwrap(),
            contract_text,
        )
        .unwrap();
    let cut = module.project_contract(&first).unwrap();
    assert_eq!(cut.source, contract_text);
    assert_eq!(cut.identity, identity);
    assert_eq!(cut.generation.get(), 1);
    assert_eq!(cut.contract.as_str(), "example.v1");
    assert_eq!(cut.contract_revision.as_str(), "1");
    assert_eq!(cut.anchors.len(), 2);
    assert!(
        cut.anchors
            .iter()
            .any(|anchor| anchor.local_id.as_str() == "work"
                && contract_text[anchor.start_byte..].starts_with("* TODO Work"))
    );
    let next = module
        .open_contract(
            &identity,
            OrgSourceGeneration::new(2).unwrap(),
            contract_text,
        )
        .unwrap();
    let next = module.project_contract(&next).unwrap();
    assert_eq!(cut.source_digest, next.source_digest);
    assert_ne!(cut.semantic_digest, next.semantic_digest);
    assert_eq!(cut.anchors, next.anchors);
    assert!(module.open_contract(&identity, OrgSourceGeneration::new(1).unwrap(),
        ":PROPERTIES:\n:ID: same\n:CONTRACT_ID: c\n:CONTRACT_REVISION: 1\n:END:\n* TODO X\n:PROPERTIES:\n:ID: same\n:END:\n").is_err());
    assert!(OrgSourceGeneration::new(0).is_err());
}

#[test]
#[ignore = "diagnostic: requires the selected Marlin Scheme package and complete Lab module closure"]
fn org_field_batch_cost_diagnostic() {
    use std::time::Instant;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let runtime = GerbilRuntime::initialize().unwrap();
    let module = ContextOrgModule::resolve(&runtime, root.to_str().unwrap()).unwrap();
    let text: String = (0..256)
        .map(|i| format!("* TODO Heading {i}\nBody {i}\n"))
        .collect();
    let source = module.open(&text).unwrap();
    let graph = module.project(&source).unwrap();
    let requests: Vec<_> = graph
        .elements
        .iter()
        .filter(|element| element.kind.as_str() == "headline")
        .take(64)
        .map(|element| OrgFieldRequest {
            element: element.id,
            name: OrgFieldName::new("title").unwrap(),
        })
        .collect();
    assert_eq!(requests.len(), 64);
    let expected = module.fields(&source, &requests).unwrap();
    assert_eq!(expected.len(), 64);
    assert_eq!(expected[0], OrgFieldValue::Text("Heading 0".into()));
    let mut samples = Vec::new();
    for _ in 0..24 {
        let start = Instant::now();
        assert_eq!(module.fields(&source, &requests).unwrap(), expected);
        samples.push(start.elapsed().as_micros());
    }
    samples.sort_unstable();
    eprintln!(
        "ORG-FIELD-DIAGNOSTIC records={} requests={} iterations={} median_us={} max_us={}",
        graph.elements.len(),
        requests.len(),
        samples.len(),
        samples[samples.len() / 2],
        samples[samples.len() - 1]
    );
}

// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierKind, RootedSchemeObject, SchemeScalar,
};
use marlin_agent_search_engine::{
    AgentTopologyRelationship, MarlinAgentStorageSearchPort, MarlinContextOrgPort,
    MarlinSearchStorageProvenance, SearchGenerationDigest, SearchProjectId, SearchStoragePortError,
    SearchTopologyEdgeId, SearchTopologyEdgeRecord, SearchWorkspaceId, admit_rooted_search_plan,
    decode_search_plan, marlin_search_engine, project_from_package, project_helm_prompt,
};
use marlin_agent_storage::InMemoryAgentStorage;
use marlin_context_org_model::{OrgNode, OrgNodeKind};

fn scheme_vector<'a>(
    runtime: &'a GerbilRuntime,
    objects: &[RootedSchemeObject<'a>],
) -> RootedSchemeObject<'a> {
    runtime
        .collection_from_objects(CollectionKind::Vector, &objects.iter().collect::<Vec<_>>())
        .unwrap()
        .into_object()
}

fn scheme_symbol<'a>(runtime: &'a GerbilRuntime, value: &str) -> RootedSchemeObject<'a> {
    runtime
        .intern_identifier(IdentifierKind::Symbol, value)
        .unwrap()
}

fn scheme_string<'a>(runtime: &'a GerbilRuntime, value: &str) -> RootedSchemeObject<'a> {
    runtime.string_from_utf8(value).unwrap().into_object()
}

fn scheme_false(runtime: &GerbilRuntime) -> RootedSchemeObject<'_> {
    runtime.scalar_object(SchemeScalar::Boolean(false)).unwrap()
}

fn sample_manifest_org_nodes() -> (String, Vec<OrgNode>) {
    let source = "* Project Workspace\n".to_owned();
    let mut doc = OrgNode::heading("doc", "Document");
    doc.kind = OrgNodeKind::Document;
    doc.properties.insert(
        "CONTRACT_ORG".to_owned(),
        "[[../contracts/project.workspace-manifest.v1.org][project.workspace-manifest.v1]]"
            .to_owned(),
    );
    doc.children.push("workspace".into());

    let mut workspace = OrgNode::heading("workspace", "Project Workspace");
    workspace
        .properties
        .insert("PROJECT_WORKSPACE_ID".to_owned(), "marlin-core".to_owned());
    workspace.properties.insert(
        "PROJECT_WORKSPACE_IDENTITY".to_owned(),
        "git+https://github.com/tao3k/marlin-agent-core.git#workspace/root".to_owned(),
    );
    workspace
        .properties
        .insert("WORKSPACE_ROOT_PATH".to_owned(), ".".to_owned());
    workspace
        .properties
        .insert("PORTABILITY".to_owned(), "cross-machine".to_owned());
    workspace
        .properties
        .insert("REPOSITORY_ALIASES".to_owned(), "[]".to_owned());

    (source, vec![doc, workspace])
}

#[test]
fn rooted_scheme_plans_and_compiled_module_project_into_asp() {
    let runtime = GerbilRuntime::initialize().unwrap();

    // 1. Valid multi-axis progressive search plan projected across RootedSchemeObject ABI.
    let rg_leaf = scheme_vector(
        &runtime,
        &[
            scheme_symbol(&runtime, "rg"),
            scheme_vector(
                &runtime,
                &[
                    scheme_string(&runtime, "-n"),
                    scheme_string(&runtime, "MarlinConvergedSearchEngine"),
                ],
            ),
        ],
    );
    let tantivy_leaf = scheme_vector(
        &runtime,
        &[
            scheme_symbol(&runtime, "tantivy"),
            scheme_string(
                &runtime,
                "title:\"converged search\"^2 OR body:SearchEngineStoragePort",
            ),
        ],
    );
    let topology_leaf = scheme_vector(
        &runtime,
        &[
            scheme_symbol(&runtime, "topology"),
            scheme_string(&runtime, "file"),
            scheme_false(&runtime),
            scheme_string(&runtime, "crates/"),
            scheme_string(&runtime, "rs"),
            scheme_false(&runtime),
        ],
    );
    let graph_leaf = scheme_vector(
        &runtime,
        &[
            scheme_symbol(&runtime, "graph"),
            scheme_symbol(&runtime, "pgql"),
            scheme_vector(
                &runtime,
                &[scheme_string(&runtime, "MATCH (a)-[r]->(b) RETURN a, r, b")],
            ),
        ],
    );
    let intersect = scheme_vector(
        &runtime,
        &[
            scheme_symbol(&runtime, "intersect"),
            scheme_vector(&runtime, &[rg_leaf, tantivy_leaf, topology_leaf]),
        ],
    );
    let chain = scheme_vector(
        &runtime,
        &[
            scheme_symbol(&runtime, "chain"),
            scheme_vector(&runtime, &[intersect, graph_leaf]),
        ],
    );
    let plan_vector = scheme_vector(
        &runtime,
        &[
            scheme_string(&runtime, "marlin-main"),
            scheme_vector(
                &runtime,
                &[
                    scheme_symbol(&runtime, "rust"),
                    scheme_symbol(&runtime, "python"),
                ],
            ),
            scheme_vector(&runtime, &[scheme_symbol(&runtime, "org")]),
            chain,
        ],
    );

    let decoded = decode_search_plan(plan_vector).expect("decode rooted search plan");
    assert_eq!(decoded.workspace.as_deref(), Some("marlin-main"));
    assert_eq!(decoded.languages, vec!["rust", "python"]);
    assert_eq!(decoded.documents, vec!["org"]);

    let (org_source, org_nodes) = sample_manifest_org_nodes();
    let org_port = Arc::new(MarlinContextOrgPort::from_node_map(BTreeMap::from([(
        org_source, org_nodes,
    )])));
    let storage = Arc::new(InMemoryAgentStorage::new());
    let storage_port = Arc::new(MarlinAgentStorageSearchPort::new(
        storage,
        MarlinSearchStorageProvenance::new("session-1", "agent-search", "evt-1").unwrap(),
    ));
    let engine = marlin_search_engine(org_port, storage_port);

    let admitted = engine
        .admit_search_plan(decoded)
        .expect("ASP engine admits typed plan projected from Gerbil Scheme");
    assert_eq!(admitted.language.as_deref(), Some("rust|python"));
    assert_eq!(admitted.documents.as_deref(), Some("org"));
    assert_eq!(admitted.rg.len(), 1);
    assert_eq!(admitted.tantivy.len(), 1);
    assert_eq!(admitted.topology.len(), 1);
    assert_eq!(admitted.graph.len(), 1);

    // 2. Fail-closed rejection when an rg leaf attempts path scope traversal.
    let invalid_rg = scheme_vector(
        &runtime,
        &[
            scheme_symbol(&runtime, "rg"),
            scheme_vector(
                &runtime,
                &[
                    scheme_string(&runtime, "-n"),
                    scheme_string(&runtime, "pattern"),
                    scheme_string(&runtime, "crates/"),
                ],
            ),
        ],
    );
    let invalid_plan = scheme_vector(
        &runtime,
        &[
            scheme_false(&runtime),
            scheme_vector(&runtime, &[scheme_symbol(&runtime, "rust")]),
            scheme_vector(&runtime, &[]),
            invalid_rg,
        ],
    );
    let error = admit_rooted_search_plan(invalid_plan)
        .expect_err("rg path scope conflict must fail closed");
    assert!(
        error.to_string().contains("rg") || error.to_string().contains("scope"),
        "unexpected error: {error}"
    );

    // 3. Compiled real Gerbil Scheme `:marlin-agent-core/c-abi/search-engine` module projection.
    let scheme_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("scheme");
    if scheme_dir
        .join(".gerbil/lib/marlin-agent-core/c-abi/search-engine.ssi")
        .exists()
    {
        let compiled_admitted =
            project_from_package(&runtime, scheme_dir.to_str().expect("utf-8 scheme dir"))
                .expect("project and admit compiled :marlin-agent-core/c-abi/search-engine");
        assert_eq!(compiled_admitted.language.as_deref(), Some("rust"));
        assert_eq!(compiled_admitted.rg.len(), 1);
        assert_eq!(compiled_admitted.tantivy.len(), 1);
        assert_eq!(compiled_admitted.topology.len(), 1);
        assert_eq!(compiled_admitted.graph.len(), 1);
    }

    // 4. Live Modern Helm (`:marlin-agent-core/c-abi/ui-helm`) prompt -> ASP pure-Rust
    //    execution over Marlin's real `crates/marlin-agent-search-engine` source tree!
    if scheme_dir
        .join(".gerbil/lib/marlin-agent-core/c-abi/ui-helm.ssi")
        .exists()
    {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root");
        let helm_admitted = project_helm_prompt(
            &runtime,
            scheme_dir.to_str().expect("utf-8 scheme dir"),
            "@lang:rust @prefix:crates/marlin-agent-search-engine/src/ marlin_search_engine #engine",
        )
        .expect("project and admit live Modern Helm prompt through Gerbil C ABI");
        let report = engine
            .execute_admitted_playbook(workspace_root, &helm_admitted)
            .expect("ASP executes live Modern Helm prompt over real Marlin repo");
        assert_eq!(report.execution_mode, "pure-rust-in-process");
        assert!(report.matched_files >= 1);
        assert!(
            report
                .hits
                .iter()
                .any(|hit| hit.path == "crates/marlin-agent-search-engine/src/engine.rs"),
            "expected engine.rs in live Helm hits: {:?}",
            report.hits
        );
    }
}

#[test]
fn marlin_ports_drive_converged_org_manifest_overlay_and_unified_storage_round_trip() {
    let (org_source, org_nodes) = sample_manifest_org_nodes();
    let org_port = Arc::new(MarlinContextOrgPort::from_node_map(BTreeMap::from([(
        org_source.clone(),
        org_nodes,
    )])));
    let storage = Arc::new(InMemoryAgentStorage::new());
    let storage_port = Arc::new(MarlinAgentStorageSearchPort::new(
        storage,
        MarlinSearchStorageProvenance::new("session-2", "agent-search", "evt-2").unwrap(),
    ));
    let engine = marlin_search_engine(org_port, storage_port);

    // 1. Port 1: Marlin OrgNode -> ASP ProjectTopologyManifest & AgentOrgTopologyOverlay (zero orgize FFI in ASP).
    let manifest = engine
        .admit_topology_manifest(&org_source)
        .expect("admit manifest through MarlinContextOrgPort");
    assert_eq!(
        manifest.project_workspace().project_workspace_identity(),
        "git+https://github.com/tao3k/marlin-agent-core.git#workspace/root"
    );

    let overlay = engine
        .admit_org_overlay(
            "blake3-256:1111111111111111111111111111111111111111111111111111111111111111",
            "blake3-256:2222222222222222222222222222222222222222222222222222222222222222",
            "crates/marlin-agent-search-engine/src/engine.rs#item=marlin_search_engine",
            "blake3-256:3333333333333333333333333333333333333333333333333333333333333333",
            "blake3-256:4444444444444444444444444444444444444444444444444444444444444444",
            "blake3-256:5555555555555555555555555555555555555555555555555555555555555555",
            "Marlin converged search engine bridge",
            org_source,
            vec![AgentTopologyRelationship {
                from_selector:
                    "crates/marlin-agent-search-engine/src/engine.rs#item=marlin_search_engine".to_owned(),
                relation: "delegates-to".to_owned(),
                to_selector:
                    "crates/agent-semantic-search/src/marlin_engine.rs#item=MarlinConvergedSearchEngine"
                        .to_owned(),
            }],
        )
        .expect("admit overlay through MarlinContextOrgPort");
    assert_eq!(overlay.terminal.state, "admitted");
    assert_eq!(overlay.summary, "Marlin converged search engine bridge");

    // 2. Port 2: Marlin AgentStorage CAS artifact + generation pointer + topology edges.
    let project_id = SearchProjectId::new("proj-marlin-core").unwrap();
    let workspace_id = SearchWorkspaceId::new("ws-main").unwrap();
    let gen1 = SearchGenerationDigest::new("blake3-256:gen1").unwrap();
    let gen2 = SearchGenerationDigest::new("blake3-256:gen2").unwrap();

    let ptr1 = engine
        .publish_generation_artifact(
            project_id.clone(),
            workspace_id.clone(),
            None,
            gen1.clone(),
            "application/octet-stream",
            b"generation-1-index-segment".to_vec(),
        )
        .expect("publish generation 1");
    assert_eq!(ptr1.revision, 1);
    assert_eq!(ptr1.generation_digest, gen1);

    let conflict = engine.publish_generation_artifact(
        project_id.clone(),
        workspace_id.clone(),
        None,
        gen2.clone(),
        "application/octet-stream",
        b"generation-2-index-segment".to_vec(),
    );
    assert!(matches!(
        conflict,
        Err(SearchStoragePortError::PointerConflict { .. })
    ));

    let ptr2 = engine
        .publish_generation_artifact(
            project_id.clone(),
            workspace_id.clone(),
            Some(&ptr1.artifact_hash),
            gen2.clone(),
            "application/octet-stream",
            b"generation-2-index-segment".to_vec(),
        )
        .expect("CAS advance to generation 2");
    assert_eq!(ptr2.revision, 2);
    assert_eq!(ptr2.generation_digest, gen2);

    let (loaded_ptr, loaded_blob) = engine
        .load_active_generation_artifact(&project_id, &workspace_id)
        .expect("load active generation")
        .expect("pointer exists");
    assert_eq!(loaded_ptr, ptr2);
    assert_eq!(loaded_blob.body, b"generation-2-index-segment");

    let edge = SearchTopologyEdgeRecord {
        project_id: project_id.clone(),
        generation_digest: gen2.clone(),
        edge_id: SearchTopologyEdgeId::new("edge-search-1").unwrap(),
        from_node: "owner:crates/marlin-agent-search-engine".to_owned(),
        to_node: "owner:crates/agent-semantic-search".to_owned(),
        relation: "bridges".to_owned(),
        payload: b"native-rust-port".to_vec(),
    };
    assert_eq!(
        engine
            .persist_topology_edges(std::slice::from_ref(&edge))
            .unwrap(),
        1
    );
    assert_eq!(
        engine
            .persist_topology_edges(std::slice::from_ref(&edge))
            .unwrap(),
        0,
        "duplicate edge append must be idempotent"
    );
    assert_eq!(
        engine.load_topology_edges(&project_id, &gen2).unwrap(),
        vec![edge]
    );
}

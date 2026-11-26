// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Multi-scale Criterion benchmarks for Marlin's converged native search engine
//! bridge (`RootedSchemeObject` ABI -> ASP playbook admission, unified
//! `AgentStorage` CAS publication/reads, topology edge persistence, and
//! `MarlinContextOrgPort` manifest/overlay admission).

use std::collections::BTreeMap;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierKind, RootedSchemeObject, SchemeScalar,
};
use marlin_agent_search_engine::{
    AgentTopologyRelationship, MarlinAgentStorageSearchPort, MarlinContextOrgPort,
    MarlinSearchStorageProvenance, SearchGenerationDigest, SearchProjectId, SearchTopologyEdgeId,
    SearchTopologyEdgeRecord, SearchWorkspaceId, WorkspaceEdbPrefilter, WorkspaceIndexConfig,
    WorkspaceOpenEdbPendingFile, WorkspaceSearchIndex, WorkspaceSearchQuery,
    admit_rooted_search_plan, encode_edb_snapshot, extract_org_open_edb_batch,
    marlin_search_engine,
};
#[cfg(not(feature = "linked-ui"))]
use marlin_agent_search_engine::{
    SearchDslExecutionRequest, default_marlin_scheme_dir, execute_package_search_dsl_structured,
};
use marlin_agent_storage::InMemoryAgentStorage;
use marlin_context_org_model::{OrgNode, OrgNodeKind};

fn scheme_vector<'a>(
    runtime: &'a GerbilRuntime,
    objects: &[RootedSchemeObject<'a>],
) -> RootedSchemeObject<'a> {
    runtime
        .collection_from_objects(CollectionKind::Vector, &objects.iter().collect::<Vec<_>>())
        .expect("build scheme vector")
        .into_object()
}

fn scheme_symbol<'a>(runtime: &'a GerbilRuntime, value: &str) -> RootedSchemeObject<'a> {
    runtime
        .intern_identifier(IdentifierKind::Symbol, value)
        .expect("intern scheme symbol")
}

fn scheme_string<'a>(runtime: &'a GerbilRuntime, value: &str) -> RootedSchemeObject<'a> {
    runtime
        .string_from_utf8(value)
        .expect("build scheme string")
        .into_object()
}

fn scheme_false(runtime: &GerbilRuntime) -> RootedSchemeObject<'_> {
    runtime
        .scalar_object(SchemeScalar::Boolean(false))
        .expect("build scheme false")
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

fn bench_storage_cas_publish_and_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("marlin_search_engine_storage_cas");

    for payload_len in [256_usize, 4_096, 65_536] {
        let payload = vec![b's'; payload_len];
        group.throughput(Throughput::Bytes(payload_len as u64));
        group.bench_with_input(
            BenchmarkId::new("publish_and_load", format!("{payload_len}B")),
            &payload,
            |bencher, body| {
                let org_port = Arc::new(MarlinContextOrgPort::from_node_map(BTreeMap::new()));
                let storage = Arc::new(InMemoryAgentStorage::new());
                let storage_port = Arc::new(MarlinAgentStorageSearchPort::new(
                    storage,
                    MarlinSearchStorageProvenance::new("bench-session", "bench-agent", "bench-evt")
                        .expect("valid provenance"),
                ));
                let engine = marlin_search_engine(org_port, storage_port);
                let project = SearchProjectId::new("bench-project").expect("project id");
                let workspace = SearchWorkspaceId::new("bench-ws").expect("workspace id");
                let gen_digest = SearchGenerationDigest::new(
                    "blake3-256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                )
                .expect("generation digest");
                let mut current_hash = None;

                bencher.iter(|| {
                    let record = engine
                        .publish_generation_artifact(
                            project.clone(),
                            workspace.clone(),
                            current_hash.as_ref(),
                            gen_digest.clone(),
                            "application/octet-stream",
                            black_box(body).clone(),
                        )
                        .expect("CAS publish");
                    current_hash = Some(record.artifact_hash.clone());
                    let loaded = engine
                        .load_active_generation_artifact(&project, &workspace)
                        .expect("load active")
                        .expect("active generation present");
                    black_box(loaded);
                });
            },
        );
    }

    group.finish();
}

fn bench_topology_edges(c: &mut Criterion) {
    let mut group = c.benchmark_group("marlin_search_engine_topology_edges");

    for edge_count in [16_usize, 128, 512] {
        let project = SearchProjectId::new("bench-project").expect("project id");
        let gen_digest = SearchGenerationDigest::new(
            "blake3-256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .expect("generation digest");
        let edges: Vec<SearchTopologyEdgeRecord> = (0..edge_count)
            .map(|idx| SearchTopologyEdgeRecord {
                project_id: project.clone(),
                generation_digest: gen_digest.clone(),
                edge_id: SearchTopologyEdgeId::new(format!("edge-{idx}")).expect("edge id"),
                from_node: format!("owner:crates/mod_{}/src/lib.rs", idx % 16),
                to_node: format!("owner:crates/mod_{}/src/engine.rs", (idx + 1) % 16),
                relation: "calls".to_owned(),
                payload: b"bench-edge".to_vec(),
            })
            .collect();

        group.throughput(Throughput::Elements(edge_count as u64));
        group.bench_with_input(
            BenchmarkId::new("persist_and_load", edge_count),
            &edges,
            |bencher, batch| {
                bencher.iter(|| {
                    let org_port = Arc::new(MarlinContextOrgPort::from_node_map(BTreeMap::new()));
                    let storage = Arc::new(InMemoryAgentStorage::new());
                    let storage_port = Arc::new(MarlinAgentStorageSearchPort::new(
                        storage,
                        MarlinSearchStorageProvenance::new(
                            "bench-session",
                            "bench-agent",
                            "bench-evt",
                        )
                        .expect("valid provenance"),
                    ));
                    let engine = marlin_search_engine(org_port, storage_port);
                    let inserted = engine
                        .persist_topology_edges(black_box(batch))
                        .expect("persist edges");
                    let loaded = engine
                        .load_topology_edges(&project, &gen_digest)
                        .expect("load edges");
                    black_box((inserted, loaded.len()));
                });
            },
        );
    }

    group.finish();
}

fn bench_org_port_admission(c: &mut Criterion) {
    let (org_source, org_nodes) = sample_manifest_org_nodes();
    let org_port = Arc::new(MarlinContextOrgPort::from_node_map(BTreeMap::from([(
        org_source.clone(),
        org_nodes,
    )])));
    let storage = Arc::new(InMemoryAgentStorage::new());
    let storage_port = Arc::new(MarlinAgentStorageSearchPort::new(
        storage,
        MarlinSearchStorageProvenance::new("bench-session", "bench-agent", "bench-evt")
            .expect("valid provenance"),
    ));
    let engine = marlin_search_engine(org_port, storage_port);

    let mut group = c.benchmark_group("marlin_search_engine_org_port");
    group.throughput(Throughput::Elements(1));
    group.bench_function("admit_topology_manifest", |bencher| {
        bencher.iter(|| {
            let manifest = engine
                .admit_topology_manifest(black_box(&org_source))
                .expect("admit manifest");
            black_box(manifest);
        });
    });

    let relationships = vec![AgentTopologyRelationship {
        from_selector: "crates/marlin-agent-search-engine/src/engine.rs#item=marlin_search_engine"
            .to_owned(),
        relation: "delegates-to".to_owned(),
        to_selector:
            "crates/agent-semantic-search/src/marlin_engine.rs#item=MarlinConvergedSearchEngine"
                .to_owned(),
    }];
    group.bench_function("admit_org_overlay", |bencher| {
        bencher.iter(|| {
            let overlay = engine
                .admit_org_overlay(
                    "blake3-256:1111111111111111111111111111111111111111111111111111111111111111",
                    "blake3-256:2222222222222222222222222222222222222222222222222222222222222222",
                    "crates/marlin-agent-search-engine/src/engine.rs#item=marlin_search_engine",
                    "blake3-256:3333333333333333333333333333333333333333333333333333333333333333",
                    "blake3-256:4444444444444444444444444444444444444444444444444444444444444444",
                    "blake3-256:5555555555555555555555555555555555555555555555555555555555555555",
                    "Marlin converged search engine bridge",
                    black_box(&org_source).clone(),
                    black_box(&relationships).clone(),
                )
                .expect("admit overlay");
            black_box(overlay);
        });
    });

    group.finish();
}

fn bench_rooted_scheme_abi_admission(c: &mut Criterion) {
    let runtime = GerbilRuntime::initialize().expect("initialize Gerbil VM");
    let mut group = c.benchmark_group("marlin_search_engine_rooted_scheme_abi");
    group.throughput(Throughput::Elements(1));

    group.bench_function("multi_axis_rooted_vector_to_admitted_playbook", |bencher| {
        bencher.iter(|| {
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
            let plan = scheme_vector(
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

            let admitted = admit_rooted_search_plan(black_box(plan)).expect("admit rooted plan");
            black_box(admitted);
        });
    });

    group.finish();

    // Open EDB zero-regex Org hydration, zero-JSON RootedSchemeObject EDB projection,
    // and live `:gerbil-ascent` + `repair.ss` execution sharing the single Gerbil VM.
    let mut edb_group = c.benchmark_group("marlin_open_edb_hydration_and_dsl");

    let mut org_body = String::new();
    for i in 0..64_usize {
        use std::fmt::Write as _;
        let _ = writeln!(
            org_body,
            "* Requirement Node {i}\n:PROPERTIES:\n:ID: req-{i}\n:STATUS: active\n:END:\n"
        );
    }
    let pending_org = vec![WorkspaceOpenEdbPendingFile {
        path: "docs/bench.org".to_owned(),
        language: "org".to_owned(),
        body: org_body,
    }];
    edb_group.throughput(Throughput::Elements(64));
    edb_group.bench_function("extract_org_open_edb_64_headings", |bencher| {
        bencher.iter(|| {
            let batch = extract_org_open_edb_batch(black_box(&pending_org));
            black_box(batch.entities.len());
        });
    });

    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let resident =
        WorkspaceSearchIndex::build_with_config(crate_root, WorkspaceIndexConfig::default());
    let rust_langs = vec!["rust".to_owned()];
    let tokens = vec!["marlin_search_engine".to_owned()];
    let snapshot = resident.project_prefiltered_edb(&WorkspaceEdbPrefilter {
        languages: &rust_langs,
        required_tokens: &tokens,
        max_files: 32,
        max_facts: 4096,
        ..Default::default()
    });
    edb_group.throughput(Throughput::Elements(snapshot.entities.len().max(1) as u64));
    edb_group.bench_function("encode_edb_snapshot_zero_json", |bencher| {
        bencher.iter(|| {
            let rooted = encode_edb_snapshot(&runtime, black_box(&snapshot))
                .expect("encode rooted EDB snapshot");
            black_box(rooted);
        });
    });

    #[cfg(not(feature = "linked-ui"))]
    if let Some(scheme_dir) = default_marlin_scheme_dir()
        && scheme_dir
            .join(".gerbil/lib/marlin-agent-core/c-abi/search-dsl.ssi")
            .exists()
    {
        let scheme_dir_str = scheme_dir.to_str().expect("utf-8 scheme dir");
        let query = "((answer path name line) <-- (file f \"rust\" path) (rust_fn _ f name line))";
        edb_group.throughput(Throughput::Elements(1));
        edb_group.bench_function("execute_package_search_dsl_structured", |bencher| {
            bencher.iter(|| {
                let res = execute_package_search_dsl_structured(
                    &runtime,
                    scheme_dir_str,
                    black_box(&SearchDslExecutionRequest {
                        index: &resident,
                        query,
                        output_limit: 4096,
                    }),
                )
                .expect("execute live :gerbil-ascent structured query");
                black_box(res.rows.len());
            });
        });
    }

    edb_group.finish();
}

fn bench_resident_cached_open_sync_and_sym_graph(c: &mut Criterion) {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let tmp = tempfile::tempdir().expect("tempdir for snapshot bench");
    let snapshot_path = tmp.path().join("bench-search-engine.asp-index.bin");
    let cfg = WorkspaceIndexConfig::default();

    // Prime snapshot on disk so warm mmap open + zero-I/O sync are measured accurately.
    let mut resident =
        WorkspaceSearchIndex::open_or_build_cached(crate_root, &snapshot_path, cfg.clone());

    let mut group = c.benchmark_group("marlin_search_engine_resident_axes");
    group.throughput(Throughput::Elements(1));

    group.bench_function("open_or_build_cached_mmap_warm", |bencher| {
        bencher.iter(|| {
            let opened = WorkspaceSearchIndex::open_or_build_cached(
                black_box(crate_root),
                black_box(&snapshot_path),
                cfg.clone(),
            );
            black_box(opened.file_count());
        });
    });

    group.bench_function("sync_workspace_from_disk_unchanged", |bencher| {
        bencher.iter(|| {
            let receipt = resident.sync_workspace_from_disk();
            black_box(receipt.live_files);
        });
    });

    group.bench_function("query_symbol_axis", |bencher| {
        bencher.iter(|| {
            let report = resident.query(black_box(&WorkspaceSearchQuery {
                symbol: Some("marlin_search_engine"),
                max_hits: 16,
                context_radius: 2,
                ..Default::default()
            }));
            black_box(report.matched_files);
        });
    });

    group.bench_function("query_graph_axis_1hop", |bencher| {
        bencher.iter(|| {
            let report = resident.query(black_box(&WorkspaceSearchQuery {
                graph_target: Some("marlin_search_engine"),
                max_hits: 32,
                context_radius: 2,
                ..Default::default()
            }));
            black_box(report.matched_files);
        });
    });

    group.finish();
}

fn benchmark_config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(40)
}

criterion_group! {
    name = benches;
    config = benchmark_config();
    targets =
        bench_storage_cas_publish_and_load,
        bench_topology_edges,
        bench_org_port_admission,
        bench_rooted_scheme_abi_admission,
        bench_resident_cached_open_sync_and_sym_graph
}
criterion_main!(benches);

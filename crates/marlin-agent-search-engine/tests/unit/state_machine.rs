// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Stateful reference-model property tests (`proptest`) for Marlin's converged
//! search engine bridge (`MarlinConvergedSearchEngine` + `MarlinAgentStorageSearchPort`
//! + `MarlinContextOrgPort`).
//!
//! Mirrors the invariants proved in `dev/quint/search-engine.qnt`:
//! - `MAR-SRCH-R04`: CAS pointer atomicity, monotonic revision, no dangling blobs,
//!   idempotent topology edges, and strict `(project_id, generation_digest)` isolation.
//! - `MAR-SRCH-R02`: fail-closed playbook admission.
//! - `MAR-SRCH-R07`: deterministic, monotonically ordered source-index ranking.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use agent_semantic_search::{SearchArtifactHash, SourceIndexRankCandidate, SourceIndexRankRequest};
use agent_semantic_search_playbook::{
    SearchPlaybookComposition, SearchPlaybookLeaf, SearchPlaybookPlanInput,
    TopologyOwnerMembershipBlock,
};
use marlin_agent_search_engine::{
    MarlinAgentStorageSearchPort, MarlinContextOrgPort, MarlinSearchStorageProvenance,
    SearchGenerationDigest, SearchProjectId, SearchStoragePortError, SearchTopologyEdgeId,
    SearchTopologyEdgeRecord, SearchWorkspaceId, marlin_search_engine,
};
use marlin_agent_storage::InMemoryAgentStorage;
use proptest::collection;
use proptest::prelude::{
    ProptestConfig, Strategy, any, prop_assert, prop_assert_eq, prop_oneof, proptest,
};

#[derive(Clone, Debug)]
enum SearchEngineTransition {
    PublishValidCas {
        workspace_idx: u8,
        gen_idx: u8,
        payload_seed: u16,
    },
    PublishStaleCas {
        workspace_idx: u8,
        stale_hash_seed: u16,
        gen_idx: u8,
        payload_seed: u16,
    },
    LoadActive {
        workspace_idx: u8,
    },
    AppendEdge {
        project_idx: u8,
        gen_idx: u8,
        edge_idx: u8,
    },
    QueryEdges {
        project_idx: u8,
        gen_idx: u8,
    },
    AdmitPlaybook {
        with_scope_conflict: bool,
        axis_mask: u8,
    },
    RankSourceIndex {
        query_idx: u8,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RefPointerState {
    generation_digest: SearchGenerationDigest,
    artifact_hash: SearchArtifactHash,
    body: Vec<u8>,
    revision: u64,
}

#[derive(Default)]
struct ReferenceSearchState {
    pointers: BTreeMap<(String, String), RefPointerState>,
    edges: BTreeSet<(String, String, String)>,
}

fn project_id(idx: u8) -> SearchProjectId {
    SearchProjectId::new(format!("proj-{}", idx % 2)).expect("valid project id")
}

fn workspace_id(idx: u8) -> SearchWorkspaceId {
    SearchWorkspaceId::new(format!("ws-{}", idx % 3)).expect("valid workspace id")
}

fn generation_digest(idx: u8) -> SearchGenerationDigest {
    let hex_char = match idx % 4 {
        0 => "a",
        1 => "b",
        2 => "c",
        _ => "d",
    };
    SearchGenerationDigest::new(format!("blake3-256:{}", hex_char.repeat(64)))
        .expect("valid generation digest")
}

fn payload_bytes(seed: u16) -> Vec<u8> {
    format!("search-generation-segment-payload:{seed}").into_bytes()
}

fn transition_strategy() -> impl Strategy<Value = SearchEngineTransition> {
    prop_oneof![
        (0u8..3, 0u8..4, any::<u16>()).prop_map(|(workspace_idx, gen_idx, payload_seed)| {
            SearchEngineTransition::PublishValidCas {
                workspace_idx,
                gen_idx,
                payload_seed,
            }
        }),
        (0u8..3, any::<u16>(), 0u8..4, any::<u16>()).prop_map(
            |(workspace_idx, stale_hash_seed, gen_idx, payload_seed)| {
                SearchEngineTransition::PublishStaleCas {
                    workspace_idx,
                    stale_hash_seed,
                    gen_idx,
                    payload_seed,
                }
            }
        ),
        (0u8..3).prop_map(|workspace_idx| SearchEngineTransition::LoadActive { workspace_idx }),
        (0u8..2, 0u8..4, 0u8..6).prop_map(|(project_idx, gen_idx, edge_idx)| {
            SearchEngineTransition::AppendEdge {
                project_idx,
                gen_idx,
                edge_idx,
            }
        }),
        (0u8..2, 0u8..4).prop_map(
            |(project_idx, gen_idx)| SearchEngineTransition::QueryEdges {
                project_idx,
                gen_idx,
            }
        ),
        (any::<bool>(), 1u8..8).prop_map(|(with_scope_conflict, axis_mask)| {
            SearchEngineTransition::AdmitPlaybook {
                with_scope_conflict,
                axis_mask,
            }
        }),
        (0u8..4).prop_map(|query_idx| SearchEngineTransition::RankSourceIndex { query_idx }),
    ]
}

fn sample_rank_candidates() -> Vec<SourceIndexRankCandidate> {
    vec![
        SourceIndexRankCandidate {
            ordinal: 0,
            path: "crates/marlin-agent-search-engine/src/engine.rs".to_owned(),
            query_keys: vec!["marlin_search_engine".to_owned(), "converged".to_owned()],
        },
        SourceIndexRankCandidate {
            ordinal: 1,
            path: "crates/marlin-agent-search-engine/src/storage_port.rs".to_owned(),
            query_keys: vec![
                "MarlinAgentStorageSearchPort".to_owned(),
                "compare_and_swap".to_owned(),
            ],
        },
        SourceIndexRankCandidate {
            ordinal: 2,
            path: "crates/marlin-agent-search-engine/src/projection.rs".to_owned(),
            query_keys: vec![
                "decode_search_plan".to_owned(),
                "RootedSchemeObject".to_owned(),
            ],
        },
        SourceIndexRankCandidate {
            ordinal: 3,
            path: "scheme/search-engine/funcs.ss".to_owned(),
            query_keys: vec!["search-engine-plan".to_owned(), "playbooks".to_owned()],
        },
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn stateful_converged_search_engine_preserves_cas_edge_admission_and_rank_invariants(
        transitions in collection::vec(transition_strategy(), 1..40),
    ) {
        let org_port = Arc::new(MarlinContextOrgPort::from_node_map(BTreeMap::new()));
        let storage = Arc::new(InMemoryAgentStorage::new());
        let storage_port = Arc::new(MarlinAgentStorageSearchPort::new(
            storage,
            MarlinSearchStorageProvenance::new("session-pbt", "agent-pbt", "evt-pbt")
                .expect("valid provenance"),
        ));
        let engine = marlin_search_engine(org_port, storage_port);
        let mut model = ReferenceSearchState::default();
        let default_project = project_id(0);

        for transition in transitions {
            match transition {
                SearchEngineTransition::PublishValidCas {
                    workspace_idx,
                    gen_idx,
                    payload_seed,
                } => {
                    let ws = workspace_id(workspace_idx);
                    let gen_digest = generation_digest(gen_idx);
                    let body = payload_bytes(payload_seed);
                    let key = (default_project.as_str().to_owned(), ws.as_str().to_owned());
                    let expected = model.pointers.get(&key).map(|state| state.artifact_hash.clone());
                    let prev_revision = model.pointers.get(&key).map_or(0, |state| state.revision);

                    let record = engine
                        .publish_generation_artifact(
                            default_project.clone(),
                            ws.clone(),
                            expected.as_ref(),
                            gen_digest.clone(),
                            "application/octet-stream",
                            body.clone(),
                        )
                        .expect("matching CAS publish must succeed");

                    prop_assert_eq!(record.revision, prev_revision + 1);
                    prop_assert_eq!(&record.generation_digest, &gen_digest);

                    model.pointers.insert(
                        key,
                        RefPointerState {
                            generation_digest: gen_digest,
                            artifact_hash: record.artifact_hash.clone(),
                            body,
                            revision: record.revision,
                        },
                    );
                }
                SearchEngineTransition::PublishStaleCas {
                    workspace_idx,
                    stale_hash_seed,
                    gen_idx,
                    payload_seed,
                } => {
                    let ws = workspace_id(workspace_idx);
                    let gen_digest = generation_digest(gen_idx);
                    let body = payload_bytes(payload_seed);
                    let key = (default_project.as_str().to_owned(), ws.as_str().to_owned());
                    let current = model.pointers.get(&key).cloned();

                    let wrong_expected = match &current {
                        Some(_) => None,
                        None => Some(
                            SearchArtifactHash::new(format!(
                                "blake3-256:{:064x}",
                                u128::from(stale_hash_seed) + 1
                            ))
                            .expect("valid synthetic hash"),
                        ),
                    };

                    let result = engine.publish_generation_artifact(
                        default_project.clone(),
                        ws.clone(),
                        wrong_expected.as_ref(),
                        gen_digest,
                        "application/octet-stream",
                        body,
                    );

                    prop_assert!(
                        matches!(result, Err(SearchStoragePortError::PointerConflict { .. })),
                        "stale expected hash must fail closed with PointerConflict: {:?}",
                        result
                    );

                    let observed = engine
                        .load_active_generation_artifact(&default_project, &ws)
                        .expect("load active after conflict");
                    match current {
                        Some(ref expected_state) => {
                            let (ptr, blob) = observed.expect("active pointer preserved");
                            prop_assert_eq!(ptr.revision, expected_state.revision);
                            prop_assert_eq!(&ptr.artifact_hash, &expected_state.artifact_hash);
                            prop_assert_eq!(&blob.body, &expected_state.body);
                        }
                        None => {
                            prop_assert!(observed.is_none());
                        }
                    }
                }
                SearchEngineTransition::LoadActive { workspace_idx } => {
                    let ws = workspace_id(workspace_idx);
                    let key = (default_project.as_str().to_owned(), ws.as_str().to_owned());
                    let observed = engine
                        .load_active_generation_artifact(&default_project, &ws)
                        .expect("load active generation");
                    match model.pointers.get(&key) {
                        Some(expected_state) => {
                            let (ptr, blob) = observed.expect("published pointer must resolve");
                            prop_assert!(blob.verify_digest().is_ok());
                            prop_assert_eq!(ptr.revision, expected_state.revision);
                            prop_assert_eq!(&ptr.generation_digest, &expected_state.generation_digest);
                            prop_assert_eq!(&ptr.artifact_hash, &expected_state.artifact_hash);
                            prop_assert_eq!(&blob.body, &expected_state.body);
                        }
                        None => {
                            prop_assert!(observed.is_none());
                        }
                    }
                }
                SearchEngineTransition::AppendEdge {
                    project_idx,
                    gen_idx,
                    edge_idx,
                } => {
                    let proj = project_id(project_idx);
                    let gen_digest = generation_digest(gen_idx);
                    let eid = SearchTopologyEdgeId::new(format!("edge-{}", edge_idx % 6))
                        .expect("valid edge id");
                    let edge = SearchTopologyEdgeRecord {
                        project_id: proj.clone(),
                        generation_digest: gen_digest.clone(),
                        edge_id: eid.clone(),
                        from_node: format!("node:from:{}", edge_idx % 3),
                        to_node: format!("node:to:{}", (edge_idx + 1) % 3),
                        relation: "calls".to_owned(),
                        payload: b"state-machine-edge".to_vec(),
                    };

                    let inserted = engine
                        .persist_topology_edges(std::slice::from_ref(&edge))
                        .expect("persist topology edge");
                    let was_new = model.edges.insert((
                        proj.as_str().to_owned(),
                        gen_digest.as_str().to_owned(),
                        eid.as_str().to_owned(),
                    ));
                    prop_assert_eq!(inserted, usize::from(was_new));
                }
                SearchEngineTransition::QueryEdges {
                    project_idx,
                    gen_idx,
                } => {
                    let proj = project_id(project_idx);
                    let gen_digest = generation_digest(gen_idx);
                    let loaded = engine
                        .load_topology_edges(&proj, &gen_digest)
                        .expect("load topology edges");

                    let expected_ids: BTreeSet<String> = model
                        .edges
                        .iter()
                        .filter(|(p, g, _)| p == proj.as_str() && g == gen_digest.as_str())
                        .map(|(_, _, id)| id.clone())
                        .collect();
                    let observed_ids: BTreeSet<String> = loaded
                        .iter()
                        .map(|edge| edge.edge_id.as_str().to_owned())
                        .collect();

                    prop_assert_eq!(observed_ids, expected_ids);
                    for edge in &loaded {
                        prop_assert_eq!(&edge.project_id, &proj);
                        prop_assert_eq!(&edge.generation_digest, &gen_digest);
                    }
                }
                SearchEngineTransition::AdmitPlaybook {
                    with_scope_conflict,
                    axis_mask,
                } => {
                    let mut stages = Vec::new();
                    if with_scope_conflict {
                        stages.push(SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Rg(
                            vec![
                                "-n".to_owned(),
                                "search".to_owned(),
                                "crates/".to_owned(),
                            ],
                        )));
                    } else {
                        if axis_mask & 1 != 0 {
                            stages.push(SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Rg(
                                vec!["-n".to_owned(), "marlin_search_engine".to_owned()],
                            )));
                        }
                        if axis_mask & 2 != 0 {
                            stages.push(SearchPlaybookComposition::Leaf(
                                SearchPlaybookLeaf::Tantivy(vec![
                                    "title:\"converged search\"^2 OR body:SearchEngineStoragePort"
                                        .to_owned(),
                                ]),
                            ));
                        }
                        if axis_mask & 4 != 0 || stages.is_empty() {
                            stages.push(SearchPlaybookComposition::Leaf(
                                SearchPlaybookLeaf::Topology(TopologyOwnerMembershipBlock {
                                    kind: "file".to_owned(),
                                    exact_path: None,
                                    path_prefix: Some("crates/".to_owned()),
                                    extension: Some("rs".to_owned()),
                                    path_glob: None,
                                }),
                            ));
                        }
                    }

                    let composition = if stages.len() == 1 {
                        stages.remove(0)
                    } else {
                        SearchPlaybookComposition::Intersect(stages)
                    };
                    let plan = SearchPlaybookPlanInput {
                        workspace: Some("marlin-pbt".to_owned()),
                        languages: vec!["rust".to_owned()],
                        documents: vec!["org".to_owned()],
                        composition,
                    };
                    let admitted = engine.admit_search_plan(plan);
                    if with_scope_conflict {
                        prop_assert!(admitted.is_err());
                    } else {
                        prop_assert!(admitted.is_ok());
                    }
                }
                SearchEngineTransition::RankSourceIndex { query_idx } => {
                    let query = match query_idx % 4 {
                        0 => "engine.rs",
                        1 => "storage_port",
                        2 => "RootedSchemeObject",
                        _ => "crates/marlin-agent-search-engine",
                    };
                    let request = SourceIndexRankRequest {
                        query: query.to_owned(),
                        candidates: sample_rank_candidates(),
                    };
                    let first = engine.rank_source_index(request.clone());
                    let second = engine.rank_source_index(request);
                    prop_assert_eq!(&first, &second);
                    for pair in first.ranked_candidates.windows(2) {
                        prop_assert!(pair[0].score.total >= pair[1].score.total);
                    }
                }
            }
        }
    }
}

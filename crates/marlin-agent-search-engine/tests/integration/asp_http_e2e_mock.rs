// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Self-contained loopback mock ASP HTTP forwarding gateway fixture used by
//! `asp_http_e2e` so CI always exercises cold hydration, BLAKE3 cache restore,
//! built-in additive schema sync (`rust_gen_fn`), targeted schema validation,
//! and brand-new dynamic language O(1) registration (`odin_proc_decl`).

use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use gerbil_scheme::GerbilRuntime;
use marlin_agent_search_engine::{
    AspForwardingEndpoint, AspHttpAstPatchOp, AspHttpAstPatchRequest, AspHttpAstPatchStatus,
    AspHttpHydrationRequest, AspLanguageId, AspProviderId, DynamicProviderCapabilities,
    DynamicProviderDslPackRegistration, DynamicProviderRegistrationManifest,
    DynamicProviderRegistrationRequest, DynamicProviderSyntaxSurface, DynamicProviderTransport,
    ForwardedKindMapping, GeneralOntologyKindName, HttpEndpointPath, NativeAstKindName,
    ProviderRegistrationStatus, ProviderTransportProtocol, SEMANTIC_OPEN_EDB_SCHEMA_ID,
    SEMANTIC_OPEN_EDB_SCHEMA_VERSION, SearchDslExecutionRequest, WorkspaceIndexConfig,
    WorkspaceSearchIndex, apply_ast_patch_via_asp_http, execute_package_search_dsl_structured,
    hydrate_workspace_from_asp_http, project_package_search_dsl_completions,
    register_dynamic_provider_on_asp, shutdown_asp_gateway,
};
use tempfile::TempDir;

fn handle_mock_gateway_request(
    method: &str,
    path: &str,
    body_str: &str,
    registered_odin: &Mutex<Option<serde_json::Value>>,
) -> (u16, serde_json::Value, bool) {
    match (method, path) {
        ("GET", "/v1/providers") => {
            let mut providers = vec![serde_json::json!({
                "manifest": {
                    "languageId": "rust",
                    "providerId": "asp-rust",
                    "extensions": [".rs"]
                },
                "targetedSchema": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["function", "item"] },
                        "languageKind": { "type": "string", "enum": ["fn_item", "impl_item", "gen_fn"] }
                    }
                },
                "dslPack": {
                    "languageId": "rust",
                    "providerLanguageId": "rust",
                    "extensions": [".rs"],
                    "kindMappings": [
                        {
                            "nativeLanguageKind": "gen_fn",
                            "generalKind": "function",
                            "description": "Rust 2027 generator function"
                        }
                    ],
                    "specialRelations": ["rust_fn", "rust_gen_fn"]
                }
            })];
            if let Some(odin) = registered_odin.lock().expect("lock").clone() {
                providers.push(odin);
            }
            (
                200,
                serde_json::json!({
                    "schemaId": "agent.semantic-protocols.provider-forwarding-catalog",
                    "schemaVersion": "1",
                    "generation": 1,
                    "registryDigest": "blake3-256:self-contained",
                    "providers": providers
                }),
                false,
            )
        }
        ("POST", "/v1/providers/register") => {
            let req: serde_json::Value =
                serde_json::from_str(body_str).unwrap_or(serde_json::Value::Null);
            let env = serde_json::json!({
                "manifest": req["manifest"],
                "targetedSchema": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["function", "struct"] },
                        "languageKind": {
                            "type": "string",
                            "enum": ["proc_decl", "struct_decl"]
                        }
                    }
                },
                "dslPack": req["dslPack"]
            });
            *registered_odin.lock().expect("lock") = Some(env);
            (
                200,
                serde_json::json!({
                    "status": "registered",
                    "mode": "descriptor-only",
                    "languageId": "odin",
                    "providerId": "asp-odin"
                }),
                false,
            )
        }
        ("POST", "/v1/open-edb/inspect-files") => (
            200,
            serde_json::json!({
                "inspectedFiles": 1,
                "elapsedMicros": 50,
                "batches": [{
                    "schemaId": SEMANTIC_OPEN_EDB_SCHEMA_ID,
                    "schemaVersion": SEMANTIC_OPEN_EDB_SCHEMA_VERSION,
                    "languageId": "rust",
                    "providerId": "asp-rust",
                    "entities": [{
                        "entityId": "rs1",
                        "fileId": 0,
                        "path": "src/lib.rs",
                        "line": 1,
                        "name": "DemoRegistry",
                        "language": "rust",
                        "kind": "item",
                        "languageKind": "impl_item",
                        "source": "native-parser",
                        "attrs": []
                    }],
                    "edges": []
                }]
            }),
            false,
        ),
        ("POST", "/shutdown") => (200, serde_json::json!({"status": "stopping"}), true),
        _ => (404, serde_json::json!({"error": "not found"}), false),
    }
}

fn odin_registration_request() -> DynamicProviderRegistrationRequest {
    let odin_lang = AspLanguageId::new("odin").expect("odin lang id");
    let odin_provider = AspProviderId::new("asp-odin").expect("odin provider id");
    DynamicProviderRegistrationRequest {
        manifest: DynamicProviderRegistrationManifest {
            schema_id: "agent.semantic-protocols.provider-core-manifest".to_owned(),
            schema_version: "1".to_owned(),
            language_id: odin_lang.clone(),
            provider_id: odin_provider,
            display_name: "Odin".to_owned(),
            extensions: vec![".odin".to_owned()],
            project_markers: vec!["ols.json".to_owned()],
            syntax_surface: DynamicProviderSyntaxSurface {
                line_comment: Some("//".to_owned()),
                block_comment: None,
                delimiter_pairs: vec![["(".to_owned(), ")".to_owned()]],
                statement_terminator: None,
                string_delimiters: vec!["\"".to_owned()],
            },
            transport: DynamicProviderTransport {
                transport: ProviderTransportProtocol::HttpJson,
                command: vec!["serve".to_owned()],
                health_path: HttpEndpointPath::new("/health").expect("health"),
                request_path: HttpEndpointPath::new("/v1/provider-runtime").expect("req"),
                shutdown_path: HttpEndpointPath::new("/shutdown").expect("shutdown"),
            },
            capabilities: DynamicProviderCapabilities {
                ast_inspect: true,
                ast_verify: true,
                open_edb: true,
                project_resolution: false,
            },
        },
        endpoint: None,
        dsl_pack: Some(DynamicProviderDslPackRegistration {
            schema_id: "agent.semantic-protocols.language-conformance-pack".to_owned(),
            schema_version: "1".to_owned(),
            language_id: odin_lang.clone(),
            provider_language_id: odin_lang,
            extensions: vec![".odin".to_owned()],
            project_markers: vec!["ols.json".to_owned()],
            kind_mappings: vec![ForwardedKindMapping {
                native_language_kind: NativeAstKindName::new("proc_decl").expect("kind"),
                general_kind: GeneralOntologyKindName::new("function").expect("kind"),
                description: "Odin procedure declaration".to_owned(),
            }],
            special_relations: vec!["odin_proc_decl".to_owned()],
            probes: Vec::new(),
        }),
    }
}

fn read_full_http_request(stream: &mut TcpStream) -> Option<(String, String, String)> {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos;
        }
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        raw.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&raw[..header_end]).into_owned();
    let content_len = head
        .lines()
        .find_map(|line| {
            let (name, val) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                val.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);
    let body_start = header_end + 4;
    while raw.len() < body_start + content_len {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&chunk[..n]);
    }
    let first_line = head.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_owned();
    let path = parts.next().unwrap_or("").to_owned();
    let body_str =
        String::from_utf8_lossy(&raw[body_start..(body_start + content_len).min(raw.len())])
            .into_owned();
    Some((method, path, body_str))
}

pub(crate) fn run_self_contained_asp_http_forwarding_and_dynamic_o1_leverage(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback mock ASP gateway");
    let addr = listener.local_addr().expect("mock gateway local addr");
    let endpoint = AspForwardingEndpoint::new(format!("http://{addr}")).expect("mock endpoint");

    let registered_odin: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let registered_odin_srv = Arc::clone(&registered_odin);
    let server_handle = std::thread::spawn(move || {
        for stream_res in listener.incoming() {
            let Ok(mut stream) = stream_res else {
                break;
            };
            let Some((method, path, body_str)) = read_full_http_request(&mut stream) else {
                continue;
            };

            let (status, resp_json, stop) =
                handle_mock_gateway_request(&method, &path, &body_str, &registered_odin_srv);
            let body = resp_json.to_string();
            let resp = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
            let _ = stream.shutdown(Shutdown::Both);
            if stop {
                break;
            }
        }
    });

    let tmp = TempDir::new().expect("temp workspace");
    let ws = tmp.path();
    fs::create_dir_all(ws.join("src")).expect("create src");
    fs::write(ws.join("src/lib.rs"), "impl DemoRegistry {}\n").expect("write lib.rs");

    let snap_path = ws.join(".marlin-test-cache/index.snap");
    let edb_cache_path = ws.join(".marlin-test-cache/open-edb.cache.json");
    let mut index =
        WorkspaceSearchIndex::open_or_build_cached(ws, &snap_path, WorkspaceIndexConfig::default());

    let cold = hydrate_workspace_from_asp_http(
        runtime,
        &mut index,
        &AspHttpHydrationRequest {
            scheme_dir,
            endpoint: &endpoint,
            cache_path: &edb_cache_path,
            sync_new_descriptors: true,
        },
    )
    .expect("self-contained cold hydration");
    assert_eq!(cold.http_inspected_files, 1);
    assert_eq!(cold.http_ingested_entities, 1);
    assert_eq!(cold.schema_validated_entities, 1);
    assert_eq!(cold.schema_drift_entities, 0);
    assert_eq!(cold.failed_files, 0);
    assert_eq!(cold.dynamic_descriptors_synced, 1);
    assert!(cold.cache_saved);

    let rust_gen_comps = project_package_search_dsl_completions(
        runtime,
        scheme_dir,
        "(ascent-fragment ((a n) <-- (rust_gen",
    )
    .expect("additive built-in rust_gen_fn completions");
    assert!(rust_gen_comps.iter().any(|c| c.form == "rust_gen_fn"));

    let mut warm_index =
        WorkspaceSearchIndex::open_or_build_cached(ws, &snap_path, WorkspaceIndexConfig::default());
    let warm = hydrate_workspace_from_asp_http(
        runtime,
        &mut warm_index,
        &AspHttpHydrationRequest {
            scheme_dir,
            endpoint: &endpoint,
            cache_path: &edb_cache_path,
            sync_new_descriptors: false,
        },
    )
    .expect("self-contained warm cache hydration");
    assert_eq!(warm.http_inspected_files, 0);
    assert_eq!(warm.restored_from_cache_entities, 1);

    let rust_res = execute_package_search_dsl_structured(
        runtime,
        scheme_dir,
        &SearchDslExecutionRequest {
            index: &warm_index,
            query: "(ascent-fragment ((answer name) <-- (rust_impl _ _ name _)))",
            output_limit: 32,
        },
    )
    .expect("self-contained rust_impl query");
    assert_eq!(rust_res.rows.len(), 1);
    assert_eq!(rust_res.rows[0].cells, vec!["DemoRegistry"]);

    let ack = register_dynamic_provider_on_asp(&endpoint, &odin_registration_request())
        .expect("register odin on mock gateway");
    assert_eq!(ack.status, ProviderRegistrationStatus::Registered);

    let synced = hydrate_workspace_from_asp_http(
        runtime,
        &mut warm_index,
        &AspHttpHydrationRequest {
            scheme_dir,
            endpoint: &endpoint,
            cache_path: &edb_cache_path,
            sync_new_descriptors: true,
        },
    )
    .expect("sync dynamic odin descriptor");
    // Scheme registry deduplication is idempotent: `rust_gen_fn` was already
    // absorbed during cold hydration, so only the newly registered `odin`
    // descriptor increments `dynamic_descriptors_synced`.
    assert_eq!(synced.dynamic_descriptors_synced, 1);

    let completions = project_package_search_dsl_completions(
        runtime,
        scheme_dir,
        "(ascent-fragment ((a n) <-- (odin_",
    )
    .expect("dynamic odin completions");
    assert!(completions.iter().any(|c| c.form == "odin_proc_decl"));

    let _ = shutdown_asp_gateway(&endpoint);
    let _ = server_handle.join();
}

/// Live P3 verification:
/// 1. Queries structural `contains_edge` and `imports_edge` across real hydrated files.
/// 2. Executes the full closed-loop agent edit cycle against live `asp-gateway` + `asp-rust`:
///    Search DSL -> `entity_locator` -> `POST /v1/ast/patch` with a broken missing-`}`
///    snippet -> `asp-rust` syn oracle auto-calibrates -> atomic disk + Open EDB sync
///    -> instant follow-up Search DSL sees the renamed symbol and new locator.
pub(crate) fn run_live_p3_closed_loop_patch_phase(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    endpoint: &AspForwardingEndpoint,
    workspace_root: &std::path::Path,
    edb_cache_path: &std::path::Path,
    index: &mut WorkspaceSearchIndex,
) -> (usize, usize) {
    let contains_res = execute_package_search_dsl_structured(
        runtime,
        scheme_dir,
        &SearchDslExecutionRequest {
            index,
            query: "(ascent-fragment ((answer parent child) <-- (contains_edge parent child _ _)))",
            output_limit: 512,
        },
    )
    .expect("live contains_edge query");
    assert!(
        !contains_res.rows.is_empty(),
        "expected structural contains_edge rows from real hydrated files"
    );

    let imports_res = execute_package_search_dsl_structured(
        runtime,
        scheme_dir,
        &SearchDslExecutionRequest {
            index,
            query: "(ascent-fragment ((answer src dst) <-- (imports_edge src dst _ _)))",
            output_limit: 512,
        },
    )
    .expect("live imports_edge query");
    assert!(
        !imports_res.rows.is_empty(),
        "expected imports_edge rows from real hydrated files"
    );

    // Stage a real Rust file, hydrate via ASP HTTP, and retrieve its authoritative locator.
    let rel_path = "src/rust/closed_loop_demo.rs";
    fs::write(
        workspace_root.join(rel_path),
        "// Lossless header comment preserved across AST splice\npub fn compute_score(x: usize) -> usize {\n    x + 1\n}\n",
    )
    .expect("write closed_loop_demo.rs");
    let _ = index.sync_workspace_from_disk();
    let _ = hydrate_workspace_from_asp_http(
        runtime,
        index,
        &AspHttpHydrationRequest {
            scheme_dir,
            endpoint,
            cache_path: edb_cache_path,
            sync_new_descriptors: false,
        },
    )
    .expect("hydrate closed_loop_demo.rs");

    let loc_res = execute_package_search_dsl_structured(
        runtime,
        scheme_dir,
        &SearchDslExecutionRequest {
            index,
            query: "(ascent-fragment ((answer loc) <-- (rust_fn id _ \"compute_score\" _) (entity_locator id loc)))",
            output_limit: 16,
        },
    )
    .expect("entity_locator search for compute_score");
    assert_eq!(loc_res.rows.len(), 1);
    let locator = loc_res.rows[0].cells[0].clone();
    assert_eq!(locator, "src/rust/closed_loop_demo.rs#fn:compute_score");

    // Deliberately omit the closing `}` to prove live `asp-rust` syn oracle calibration!
    let patch_receipt = apply_ast_patch_via_asp_http(
        endpoint,
        index,
        &AspHttpAstPatchRequest {
            workspace_root,
            path: rel_path,
            language_override: None,
            expected_preimage_hash: None,
            operation: AspHttpAstPatchOp::ReplaceItem {
                target: locator,
                snippet: "pub fn compute_weighted_score(x: usize) -> usize {\n    x * 2 + 1\n"
                    .to_owned(),
            },
            dry_run: false,
            auto_calibrate: true,
        },
    )
    .expect("closed-loop POST /v1/ast/patch with calibration");
    assert_eq!(
        patch_receipt.status,
        AspHttpAstPatchStatus::CalibratedAndApplied
    );
    assert!(patch_receipt.reparse_verified);
    assert!(patch_receipt.synced_entities >= 1);

    let on_disk =
        fs::read_to_string(workspace_root.join(rel_path)).expect("read patched file from disk");
    assert!(on_disk.contains("// Lossless header comment preserved across AST splice"));
    assert!(on_disk.contains("pub fn compute_weighted_score"));

    // Immediate follow-up Search DSL sees the renamed function + new locator (0 extra HTTP!).
    let after_res = execute_package_search_dsl_structured(
        runtime,
        scheme_dir,
        &SearchDslExecutionRequest {
            index,
            query: "(ascent-fragment ((answer loc) <-- (rust_fn id _ \"compute_weighted_score\" _) (entity_locator id loc)))",
            output_limit: 16,
        },
    )
    .expect("follow-up search for compute_weighted_score");
    assert_eq!(after_res.rows.len(), 1);
    assert_eq!(
        after_res.rows[0].cells[0],
        "src/rust/closed_loop_demo.rs#fn:compute_weighted_score"
    );
    (contains_res.rows.len(), imports_res.rows.len())
}

/// Live S1 Dusa POPL 2025 verification over REAL `asp-rust` (`syn`) +
/// `asp-gerbil-scheme` extracted AST entities and `calls` edges:
/// executes BOTH verbatim Scenario B queries (callsite disambiguation +
/// architecture tier constraint verification) and asserts exact world counts,
/// `[deduction]` vs `[branch]` witnesses, test-mock exclusion, and jumpable paths.
pub(crate) fn run_live_s1_dusa_scenario_b_phase(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    endpoint: &AspForwardingEndpoint,
) -> (usize, usize) {
    use marlin_agent_search_engine::{SearchDslChoiceWitnessKind, SearchDslExecutionMode};

    let tmp = TempDir::new().expect("scenario B temp workspace");
    let ws = tmp.path();
    fs::create_dir_all(ws.join("src")).expect("create src");
    fs::create_dir_all(ws.join("tests")).expect("create tests");
    fs::create_dir_all(ws.join("crates/marlin-agent-search-engine/src"))
        .expect("create engine dir");
    fs::create_dir_all(ws.join("scheme/search-dsl")).expect("create scheme dir");

    fs::write(
        ws.join("src/app.rs"),
        "pub fn local_init() -> usize { 1 }\n\
         pub fn run_app() -> usize {\n\
             let a = local_init();\n\
             let b = parse_payload();\n\
             a + b\n\
         }\n",
    )
    .expect("write src/app.rs");
    fs::write(
        ws.join("src/parser_a.rs"),
        "pub fn parse_payload() -> usize { 10 }\n",
    )
    .expect("write src/parser_a.rs");
    fs::write(
        ws.join("src/parser_b.rs"),
        "pub fn parse_payload() -> usize { 20 }\n",
    )
    .expect("write src/parser_b.rs");
    fs::write(
        ws.join("tests/mock_parser.rs"),
        "pub fn parse_payload() -> usize { 99 }\n",
    )
    .expect("write tests/mock_parser.rs");

    let snap_path = ws.join(".cache/index.snap");
    let edb_cache_path = ws.join(".cache/open-edb.cache.json");
    let mut index =
        WorkspaceSearchIndex::open_or_build_cached(ws, &snap_path, WorkspaceIndexConfig::default());
    let receipt = hydrate_workspace_from_asp_http(
        runtime,
        &mut index,
        &AspHttpHydrationRequest {
            scheme_dir,
            endpoint,
            cache_path: &edb_cache_path,
            sync_new_descriptors: false,
        },
    )
    .expect("hydrate scenario B.1 workspace via real asp-rust");
    assert_eq!(receipt.failed_files, 0);

    // Scenario B.1: VERBATIM same-name call-site disambiguation query
    let query_b1 = "(search-choice-program
      (use rust)
      (choice resolve (callsite target))
      (rule (closed (resolve ?call) (?def))
            (ref-at ?call ?name)
            (same-module-def ?call ?name ?def))
      (rule (open (resolve ?call) ?def)
            (ref-at ?call ?name)
            (workspace-def ?name ?def))
      (rule (closed (resolve ?call) ())
            (resolve ?call ?def)
            (in-test-file ?def))
      (solve resolve state-budget: 32 provenance?: #t))";
    let res_b1 = execute_package_search_dsl_structured(
        runtime,
        scheme_dir,
        &SearchDslExecutionRequest {
            index: &index,
            query: query_b1,
            output_limit: 8192,
        },
    )
    .expect("live Dusa Scenario B.1");

    assert_eq!(res_b1.mode, SearchDslExecutionMode::Choice);
    assert_eq!(res_b1.relation, "resolve");
    assert_eq!(
        res_b1.choice_worlds.len(),
        2,
        "test mock must be excluded leaving 2 worlds (parser_a vs parser_b), got:\n{}",
        res_b1.rendered_text
    );
    assert!(
        !res_b1.rendered_text.contains("tests/mock_parser.rs"),
        "forbidden test mock leaked into Dusa worlds:\n{}",
        res_b1.rendered_text
    );
    assert!(
        res_b1.choice_worlds[0]
            .witnesses
            .iter()
            .any(|w| w.kind == SearchDslChoiceWitnessKind::Deduction),
        "expected same-module Deduction witness in B.1:\n{}",
        res_b1.rendered_text
    );
    assert!(
        res_b1.choice_worlds[0]
            .witnesses
            .iter()
            .any(|w| w.kind == SearchDslChoiceWitnessKind::Branch),
        "expected cross-module Branch witness in B.1:\n{}",
        res_b1.rendered_text
    );
    assert!(
        res_b1
            .rows
            .iter()
            .any(|r| r.path.as_deref() == Some("src/app.rs") && r.line.is_some()),
        "Dusa callsite rows must project jumpable path + line for TUI Helm"
    );

    // Scenario B.2: Architecture tier boundary verification over real Rust + Scheme files
    let tmp_b2 = TempDir::new().expect("scenario B.2 temp workspace");
    let ws2 = tmp_b2.path();
    fs::create_dir_all(ws2.join("crates/marlin-agent-search-engine/src/edb_bridge"))
        .expect("create engine");
    fs::create_dir_all(ws2.join("scheme/search-dsl")).expect("create scheme");
    fs::create_dir_all(ws2.join("crates/marlin-agent-cli/src")).expect("create cli");
    fs::write(
        ws2.join("crates/marlin-agent-search-engine/src/edb_bridge/codec.rs"),
        "pub fn decode() {}\n",
    )
    .expect("write codec.rs");
    fs::write(
        ws2.join("scheme/search-dsl/funcs.ss"),
        "(def (eval-query x) x)\n",
    )
    .expect("write funcs.ss");
    fs::write(
        ws2.join("crates/marlin-agent-cli/src/main.rs"),
        "pub fn main() {}\n",
    )
    .expect("write main.rs");
    let mut index2 = WorkspaceSearchIndex::open_or_build_cached(
        ws2,
        &ws2.join(".cache/index.snap"),
        WorkspaceIndexConfig::default(),
    );
    let _ = hydrate_workspace_from_asp_http(
        runtime,
        &mut index2,
        &AspHttpHydrationRequest {
            scheme_dir,
            endpoint,
            cache_path: &ws2.join(".cache/open-edb.cache.json"),
            sync_new_descriptors: false,
        },
    )
    .expect("hydrate scenario B.2 workspace");

    let query_b2 = "(search-choice-program
      (use rust scheme)
      (choice tier (mod level))
      (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m))
      (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m 'provider))
      (solve tier state-budget: 64 provenance?: #t))";
    let res_b2 = execute_package_search_dsl_structured(
        runtime,
        scheme_dir,
        &SearchDslExecutionRequest {
            index: &index2,
            query: query_b2,
            output_limit: 8192,
        },
    )
    .expect("live Dusa Scenario B.2");
    assert_eq!(
        res_b2.choice_worlds.len(),
        2,
        "2 hot-path modules forced resident + 1 cli module branching => 2 worlds, got:\n{}",
        res_b2.rendered_text
    );
    for world in &res_b2.choice_worlds {
        assert!(
            world.bindings.iter().any(|b| {
                b.first().map(String::as_str)
                    == Some("crates/marlin-agent-search-engine/src/edb_bridge/codec.rs")
                    && b.get(1).map(String::as_str) == Some("resident")
            }),
            "hot-path Rust engine module must be forced resident in all worlds"
        );
        assert!(
            world.bindings.iter().any(|b| {
                b.first().map(String::as_str) == Some("scheme/search-dsl/funcs.ss")
                    && b.get(1).map(String::as_str) == Some("resident")
            }),
            "hot-path Scheme search-dsl module must be forced resident in all worlds"
        );
    }

    println!(
        "\n--- LIVE DUSA SCENARIO B.1 OUTPUT ---\n{}\n--- LIVE DUSA SCENARIO B.2 OUTPUT ---\n{}",
        res_b1.rendered_text, res_b2.rendered_text
    );
    (res_b1.choice_worlds.len(), res_b2.choice_worlds.len())
}

// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! End-to-End Live 5-Language ASP HTTP Forwarding -> Marlin Zero-JSON C-ABI
//! Search DSL & Dynamic O(1) Language Extension Integration + Latency Suite.
//!
//! Exercises the complete production stack across live OS processes:
//!   1. Spawns ASP's `forwarding_gateway` (backing ALL 5 real native compilers:
//!      `asp-rust` syn, `asp-python` stdlib ast, `asp-typescript` tsc,
//!      `AspJulia.jl` JuliaSyntax, `asp-gerbil-scheme` Gerbil 0.19).
//!   2. Hydrates REAL production source files from all 5 repos over HTTP
//!      (`GET /v1/providers` + `POST /v1/open-edb/inspect-files`) into Marlin's
//!      `WorkspaceSearchIndex` + BLAKE3 cache.
//!   3. Verifies 100% BLAKE3 warm cache hit (`http_inspected_files == 0`).
//!   4. Runs polyglot General (`callable`, `type_decl`) and all 5 Special
//!      (`rust_impl`, `scheme_poo_def`, `py_fn`, `ts_fn`, `julia_fn`)
//!      S-expression queries over Marlin's zero-JSON C-ABI and measures p50/p95.
//!   5. Proves O(1) Leverage: hot-registers a brand-new 6th language (`zig`)
//!      with `POST /v1/providers/register`, forwards its manifest + targeted
//!      JSON Schema + DSL pack over HTTP into Marlin Scheme without recompiling
//!      ASP or Marlin, and immediately queries `(zig_comptime_fn ...)` and
//!      `(callable ... "zig" ...)` plus live Helm TAB completions.

#![cfg(not(feature = "linked-ui"))]

mod asp_http_e2e_mock;

use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use gerbil_scheme::{GerbilRuntime, IdentifierKind};
use marlin_agent_search_engine::{
    AspForwardingEndpoint, AspHttpHydrationRequest, AspLanguageId, AspProviderId,
    DynamicProviderCapabilities, DynamicProviderDslPackRegistration,
    DynamicProviderRegistrationManifest, DynamicProviderRegistrationRequest,
    DynamicProviderSyntaxSurface, DynamicProviderTransport, ForwardedKindMapping,
    GeneralOntologyKindName, HttpEndpointPath, NativeAstKindName, ProviderRegistrationStatus,
    ProviderTransportProtocol, SEMANTIC_OPEN_EDB_SCHEMA_ID, SEMANTIC_OPEN_EDB_SCHEMA_VERSION,
    SearchDslExecutionRequest, WorkspaceEdbEntityFact, WorkspaceIndexConfig,
    WorkspaceOpenEdbIngestBatch, WorkspaceSearchIndex, decode_edb_prefilter,
    decode_rooted_search_dsl_result, default_marlin_scheme_dir, encode_edb_snapshot,
    execute_package_search_dsl_structured, fetch_asp_provider_catalog,
    hydrate_workspace_from_asp_http, project_package_search_dsl_completions,
    register_dynamic_provider_on_asp, shutdown_asp_gateway,
};
use tempfile::TempDir;

struct GatewayGuard {
    child: Child,
    stdout: Option<BufReader<ChildStdout>>,
    endpoint: AspForwardingEndpoint,
    providers: Vec<String>,
}

impl Drop for GatewayGuard {
    fn drop(&mut self) {
        // Release stdout pipe, send cooperative POST /shutdown so forwarding_gateway
        // shuts down all 5 child compiler process groups cleanly, then wait up to 3s.
        let _ = self.stdout.take();
        let _ = shutdown_asp_gateway(&self.endpoint);
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn marlin_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("marlin-agent-core root")
        .to_path_buf()
}

fn asp_root() -> PathBuf {
    marlin_root()
        .parent()
        .expect("tao3k parent")
        .join("agent-semantic-protocols")
}

fn spawn_asp_forwarding_gateway(asp_dir: &Path) -> Option<GatewayGuard> {
    let bin = asp_dir.join("target/debug/examples/forwarding_gateway");
    if !bin.exists() {
        return None;
    }
    let mut child = Command::new(&bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn ASP forwarding_gateway");
    let stdout = child.stdout.take().expect("gateway stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let n = reader.read_line(&mut line).expect("read gateway bootstrap");
    assert!(n > 0, "forwarding_gateway exited before bootstrap");
    let boot: serde_json::Value =
        serde_json::from_str(line.trim()).expect("decode gateway bootstrap JSON");
    let endpoint = AspForwardingEndpoint::new(
        boot["endpoint"]
            .as_str()
            .expect("gateway bootstrap endpoint"),
    )
    .expect("valid loopback endpoint");
    let providers = boot["providers"]
        .as_array()
        .expect("providers array")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    Some(GatewayGuard {
        child,
        stdout: Some(reader),
        endpoint,
        providers,
    })
}

fn copy_real_file(src: &Path, dst_root: &Path, rel: &str) {
    let dst = dst_root.join(rel);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).expect("create workspace subdir");
    }
    let body = fs::read_to_string(src).unwrap_or_else(|e| panic!("read {}: {e}", src.display()));
    fs::write(&dst, body).unwrap_or_else(|e| panic!("write {}: {e}", dst.display()));
}

fn benchmark_hot_c_abi_query(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    index: &WorkspaceSearchIndex,
    query: &str,
    iterations: usize,
) -> (u128, u128) {
    let module = runtime
        .intern_identifier(
            IdentifierKind::Symbol,
            ":marlin-agent-core/c-abi/search-dsl",
        )
        .expect("module sym");
    let compile_sym = runtime
        .intern_identifier(
            IdentifierKind::Symbol,
            "marlin-search-dsl-compile-prefilter",
        )
        .expect("compile sym");
    let eval_sym = runtime
        .intern_identifier(IdentifierKind::Symbol, "marlin-search-dsl-eval-edb")
        .expect("eval sym");
    let compile_fn = runtime
        .resolve_package_export(scheme_dir, &module, &compile_sym)
        .expect("resolve compile export");
    let eval_fn = runtime
        .resolve_package_export(scheme_dir, &module, &eval_sym)
        .expect("resolve eval export");

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let t0 = Instant::now();
        let q_obj = runtime
            .string_from_utf8(query)
            .expect("query string")
            .into_object();
        let pf_obj = runtime.call_rooted(&compile_fn, &q_obj).expect("compile");
        let pf = decode_edb_prefilter(pf_obj).expect("decode prefilter");
        let snap = index.project_prefiltered_edb(&pf.as_prefilter());
        let edb_obj = encode_edb_snapshot(runtime, &snap).expect("encode edb");
        let eval_in = runtime
            .collection_from_objects(gerbil_scheme::CollectionKind::Vector, &[&q_obj, &edb_obj])
            .expect("eval input vec")
            .into_object();
        let res_obj = runtime.call_rooted(&eval_fn, &eval_in).expect("eval");
        let decoded =
            decode_rooted_search_dsl_result(res_obj, &snap, 512).expect("decode structured");
        assert!(!decoded.rows.is_empty());
        samples.push(t0.elapsed().as_micros());
    }
    samples.sort_unstable();
    (
        samples[samples.len() / 2],
        samples[(samples.len() * 95) / 100],
    )
}

#[test]
fn asp_http_forwarding_to_marlin_search_and_dynamic_o1_leverage() {
    let scheme_dir = default_marlin_scheme_dir()
        .expect("compiled Marlin scheme dir")
        .to_str()
        .expect("UTF-8 scheme dir")
        .to_owned();
    let runtime = GerbilRuntime::initialize().expect("initialize GerbilRuntime");

    // Always-on CI coverage: self-contained loopback mock ASP HTTP gateway.
    asp_http_e2e_mock::run_self_contained_asp_http_forwarding_and_dynamic_o1_leverage(
        &runtime,
        &scheme_dir,
    );

    let asp_dir = asp_root();
    let marlin_dir = marlin_root();
    let Some(gateway) = spawn_asp_forwarding_gateway(&asp_dir) else {
        if std::env::var_os("MARLIN_REQUIRE_LIVE_ASP_GATEWAY").is_some() {
            panic!(
                "missing ASP forwarding_gateway binary at {}; build with `cargo build --offline -p agent-semantic-ast --example forwarding_gateway`",
                asp_dir
                    .join("target/debug/examples/forwarding_gateway")
                    .display()
            );
        }
        eprintln!(
            "skipping live 5-compiler ASP gateway phase (sibling binary absent at {}; self-contained loopback ASP HTTP phase passed)",
            asp_dir
                .join("target/debug/examples/forwarding_gateway")
                .display()
        );
        return;
    };

    assert_eq!(
        gateway.providers.len(),
        5,
        "expected all 5 native language providers live, got: {:?}",
        gateway.providers
    );

    // Build a real polyglot workspace containing ACTUAL production source files
    // from all 5 languages (Rust, Python, TypeScript, Julia, Gerbil Scheme).
    let tmp = TempDir::new().expect("temp workspace");
    let ws = tmp.path();
    copy_real_file(
        &asp_dir.join("crates/agent-semantic-ast/src/registry.rs"),
        ws,
        "src/rust/registry.rs",
    );
    copy_real_file(
        &asp_dir.join("crates/agent-semantic-ast/src/span_overlay.rs"),
        ws,
        "src/rust/span_overlay.rs",
    );
    copy_real_file(
        &asp_dir.join("languages/asp-python/src/asp_python/_ast_ops.py"),
        ws,
        "src/python/ast_ops.py",
    );
    copy_real_file(
        &asp_dir.join("languages/asp-typescript/src/parser/native_syntax/ast-ops.ts"),
        ws,
        "src/ts/ast_ops.ts",
    );
    copy_real_file(
        &asp_dir.join("languages/AspJulia.jl/src/asp_client_server/ast_ops.jl"),
        ws,
        "src/julia/ast_ops.jl",
    );
    copy_real_file(
        &marlin_dir.join("scheme/search-dsl/objects.ss"),
        ws,
        "src/scheme/objects.ss",
    );
    copy_real_file(
        &marlin_dir.join("scheme/search-dsl/funcs.ss"),
        ws,
        "src/scheme/funcs.ss",
    );
    copy_real_file(
        &marlin_dir.join("scheme/ast-style/objects.ss"),
        ws,
        "src/scheme/ast_style_objects.ss",
    );

    let snap_path = ws.join(".marlin-test-cache/index.snap");
    let edb_cache_path = ws.join(".marlin-test-cache/open-edb.cache.json");
    let mut index =
        WorkspaceSearchIndex::open_or_build_cached(ws, &snap_path, WorkspaceIndexConfig::default());

    // -------------------------------------------------------------------------
    // Phase 1: Cold ASP HTTP Hydration across all 5 real native compilers
    // -------------------------------------------------------------------------
    let cold_receipt = hydrate_workspace_from_asp_http(
        &runtime,
        &mut index,
        &AspHttpHydrationRequest {
            scheme_dir: &scheme_dir,
            endpoint: &gateway.endpoint,
            cache_path: &edb_cache_path,
            sync_new_descriptors: true,
        },
    )
    .expect("cold ASP HTTP hydration");

    assert_eq!(cold_receipt.providers_discovered, 5);
    assert_eq!(cold_receipt.http_inspected_files, 8);
    assert!(
        cold_receipt.http_ingested_entities >= 40,
        "expected >=40 real AST entities across 5 languages, got {}",
        cold_receipt.http_ingested_entities
    );
    assert_eq!(
        cold_receipt.schema_validated_entities, cold_receipt.http_ingested_entities,
        "all live 5-language AST entities must pass forwarded targetedSchema validation"
    );
    assert_eq!(cold_receipt.schema_drift_entities, 0);
    assert_eq!(cold_receipt.failed_files, 0);
    assert!(cold_receipt.cache_saved, "BLAKE3 Open EDB cache must save");

    // Verify ASP forwarded targeted JSON Schemas for all 5 languages.
    let catalog = fetch_asp_provider_catalog(&gateway.endpoint).expect("fetch catalog");
    for env in &catalog.providers {
        assert_eq!(env.targeted_schema.schema_type(), Some("object"));
        assert!(
            !env.targeted_schema.language_kind_enum().is_empty(),
            "provider {} must forward non-empty languageKind enum in targetedSchema",
            env.manifest.language_id.as_str()
        );
    }

    // -------------------------------------------------------------------------
    // Phase 2: Warm BLAKE3 Cache Restore (0 HTTP inspect calls!)
    // -------------------------------------------------------------------------
    let mut warm_index =
        WorkspaceSearchIndex::open_or_build_cached(ws, &snap_path, WorkspaceIndexConfig::default());
    let warm_receipt = hydrate_workspace_from_asp_http(
        &runtime,
        &mut warm_index,
        &AspHttpHydrationRequest {
            scheme_dir: &scheme_dir,
            endpoint: &gateway.endpoint,
            cache_path: &edb_cache_path,
            sync_new_descriptors: false,
        },
    )
    .expect("warm BLAKE3 cache hydration");

    assert_eq!(
        warm_receipt.http_inspected_files, 0,
        "warm BLAKE3 cache must avoid redundant HTTP inspections"
    );
    assert_eq!(warm_receipt.restored_from_cache_files, 8);
    assert_eq!(
        warm_receipt.restored_from_cache_entities,
        cold_receipt.http_ingested_entities
    );

    // -------------------------------------------------------------------------
    // Phase 3: Real Production Codebase Polyglot S-Expression Search + Latency
    // -------------------------------------------------------------------------
    let general_callable_query = "(ascent-fragment
      (relation answer ((lang string?) (name string?)))
      ((answer lang name) <-- (callable _ _ lang name _)))";
    let res_callable = execute_package_search_dsl_structured(
        &runtime,
        &scheme_dir,
        &SearchDslExecutionRequest {
            index: &warm_index,
            query: general_callable_query,
            output_limit: 512,
        },
    )
    .expect("polyglot General callable query");

    let langs_hit: BTreeSet<&str> = res_callable
        .rows
        .iter()
        .filter_map(|r| r.cells.first().map(String::as_str))
        .collect();
    for expected_lang in ["rust", "python", "typescript", "julia", "scheme"] {
        assert!(
            langs_hit.contains(expected_lang),
            "General Ontology `callable` must hit real `{expected_lang}` code; got langs={langs_hit:?}"
        );
    }

    // Special Rust query (`rust_impl`) on real `registry.rs`
    let res_rust_impl = execute_package_search_dsl_structured(
        &runtime,
        &scheme_dir,
        &SearchDslExecutionRequest {
            index: &warm_index,
            query: "(ascent-fragment ((answer name line) <-- (rust_impl _ _ name line)))",
            output_limit: 128,
        },
    )
    .expect("Rust special rust_impl query");
    assert!(
        res_rust_impl.rows.iter().any(|r| r
            .cells
            .first()
            .is_some_and(|n| n.contains("AstProviderRegistry"))),
        "expected AstProviderRegistry in rust_impl rows, got {:?}",
        res_rust_impl.rows
    );

    // Special Scheme POO query (`scheme_poo_def`) on real `objects.ss`
    let res_scheme_poo = execute_package_search_dsl_structured(
        &runtime,
        &scheme_dir,
        &SearchDslExecutionRequest {
            index: &warm_index,
            query: "(ascent-fragment ((answer name) <-- (scheme_poo_def _ _ name _)))",
            output_limit: 128,
        },
    )
    .expect("Scheme special scheme_poo_def query");
    let poo_names: BTreeSet<&str> = res_scheme_poo
        .rows
        .iter()
        .filter_map(|r| r.cells.first().map(String::as_str))
        .collect();
    assert!(
        poo_names.contains("GeneralLanguageOntologyFragment.")
            && poo_names.contains("BaseCodingStyleFragment."),
        "expected real POO prototypes in scheme_poo_def, got {poo_names:?}"
    );

    // Special Python, TypeScript, and Julia queries on real provider code
    let res_py = execute_package_search_dsl_structured(
        &runtime,
        &scheme_dir,
        &SearchDslExecutionRequest {
            index: &warm_index,
            query: "(ascent-fragment ((answer name) <-- (py_fn _ _ name _)))",
            output_limit: 128,
        },
    )
    .expect("Python special py_fn query");
    assert!(!res_py.rows.is_empty());

    let res_ts = execute_package_search_dsl_structured(
        &runtime,
        &scheme_dir,
        &SearchDslExecutionRequest {
            index: &warm_index,
            query: "(ascent-fragment ((answer name) <-- (ts_fn _ _ name _)))",
            output_limit: 128,
        },
    )
    .expect("TypeScript special ts_fn query");
    assert!(!res_ts.rows.is_empty());

    let res_julia = execute_package_search_dsl_structured(
        &runtime,
        &scheme_dir,
        &SearchDslExecutionRequest {
            index: &warm_index,
            query: "(ascent-fragment ((answer name) <-- (julia_fn _ _ name _)))",
            output_limit: 128,
        },
    )
    .expect("Julia special julia_fn query");
    assert!(
        res_julia.rows.iter().any(|r| r
            .cells
            .first()
            .is_some_and(|n| n == "handle_julia_ast_inspect")),
        "expected handle_julia_ast_inspect in Julia rows, got {:?}",
        res_julia.rows
    );

    // Latency benchmark 1: Hot Zero-JSON C-ABI path (pre-resolved exports, simulating linked-ui)
    let (hot_p50_us, hot_p95_us) = benchmark_hot_c_abi_query(
        &runtime,
        &scheme_dir,
        &warm_index,
        general_callable_query,
        30,
    );

    // Latency benchmark 2: Cold-lookup package runner path (20 iterations)
    let mut pkg_samples_us = Vec::with_capacity(20);
    for _ in 0..20 {
        let t0 = Instant::now();
        let r = execute_package_search_dsl_structured(
            &runtime,
            &scheme_dir,
            &SearchDslExecutionRequest {
                index: &warm_index,
                query: general_callable_query,
                output_limit: 512,
            },
        )
        .expect("bench query");
        assert!(!r.rows.is_empty());
        pkg_samples_us.push(t0.elapsed().as_micros());
    }
    pkg_samples_us.sort_unstable();
    let pkg_p50_us = pkg_samples_us[pkg_samples_us.len() / 2];
    let pkg_p95_us = pkg_samples_us[(pkg_samples_us.len() * 95) / 100];

    // -------------------------------------------------------------------------
    // Phase 4: O(1) Leverage Proof — Hot-Register 6th Language (`zig`) over ASP
    //          HTTP -> Marlin Zero-JSON C-ABI with ZERO Recompilation!
    // -------------------------------------------------------------------------
    let zig_lang = AspLanguageId::new("zig").expect("zig lang id");
    let zig_provider = AspProviderId::new("asp-zig").expect("zig provider id");
    let zig_registration = DynamicProviderRegistrationRequest {
        manifest: DynamicProviderRegistrationManifest {
            schema_id: "agent.semantic-protocols.provider-core-manifest".to_owned(),
            schema_version: "1".to_owned(),
            language_id: zig_lang.clone(),
            provider_id: zig_provider,
            display_name: "Zig".to_owned(),
            extensions: vec![".zig".to_owned()],
            project_markers: vec!["build.zig".to_owned()],
            syntax_surface: DynamicProviderSyntaxSurface {
                line_comment: Some("//".to_owned()),
                block_comment: None,
                delimiter_pairs: vec![
                    ["(".to_owned(), ")".to_owned()],
                    ["[".to_owned(), "]".to_owned()],
                    ["{".to_owned(), "}".to_owned()],
                ],
                statement_terminator: Some(";".to_owned()),
                string_delimiters: vec!["\"".to_owned()],
            },
            transport: DynamicProviderTransport {
                transport: ProviderTransportProtocol::HttpJson,
                command: vec!["serve".to_owned()],
                health_path: HttpEndpointPath::new("/health").expect("health path"),
                request_path: HttpEndpointPath::new("/v1/provider-runtime").expect("request path"),
                shutdown_path: HttpEndpointPath::new("/shutdown").expect("shutdown path"),
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
            language_id: zig_lang.clone(),
            provider_language_id: zig_lang,
            extensions: vec![".zig".to_owned()],
            project_markers: vec!["build.zig".to_owned()],
            kind_mappings: vec![
                ForwardedKindMapping {
                    native_language_kind: NativeAstKindName::new("comptime_fn").expect("kind"),
                    general_kind: GeneralOntologyKindName::new("function").expect("kind"),
                    description: "Zig compile-time generic function".to_owned(),
                },
                ForwardedKindMapping {
                    native_language_kind: NativeAstKindName::new("struct_decl").expect("kind"),
                    general_kind: GeneralOntologyKindName::new("struct").expect("kind"),
                    description: "Zig struct declaration".to_owned(),
                },
            ],
            special_relations: vec!["zig_comptime_fn".to_owned(), "zig_struct_decl".to_owned()],
            probes: Vec::new(),
        }),
    };

    let reg_ack = register_dynamic_provider_on_asp(&gateway.endpoint, &zig_registration)
        .expect("dynamic POST /v1/providers/register for zig");
    assert_eq!(reg_ack.status, ProviderRegistrationStatus::Registered);

    // Verify ASP automatically synthesized Zig's targetedSchema and exposes 6 providers!
    let updated_catalog = fetch_asp_provider_catalog(&gateway.endpoint).expect("updated catalog");
    assert_eq!(updated_catalog.providers.len(), 6);
    let zig_env = updated_catalog
        .providers
        .iter()
        .find(|p| p.manifest.language_id.as_str() == "zig")
        .expect("zig in catalog");
    assert_eq!(
        zig_env.targeted_schema.language_kind_enum(),
        vec!["comptime_fn", "struct_decl"]
    );

    // Sync Zig descriptor over HTTP -> Marlin zero-JSON C-ABI!
    let sync_receipt = hydrate_workspace_from_asp_http(
        &runtime,
        &mut warm_index,
        &AspHttpHydrationRequest {
            scheme_dir: &scheme_dir,
            endpoint: &gateway.endpoint,
            cache_path: &edb_cache_path,
            sync_new_descriptors: true,
        },
    )
    .expect("sync dynamic zig descriptor into Marlin");
    assert_eq!(sync_receipt.providers_discovered, 6);
    assert_eq!(sync_receipt.dynamic_descriptors_synced, 1);

    // Ingest General JSON facts for a `.zig` file into Marlin's WorkspaceSearchIndex!
    fs::write(
        ws.join("src/zig_demo.zig"),
        "pub fn buildTargetMatrix(comptime T: type) type { return T; }\n",
    )
    .expect("write zig source");
    let _ = warm_index.sync_workspace_from_disk();
    warm_index.ingest_open_edb_facts(WorkspaceOpenEdbIngestBatch {
        schema_id: Some(SEMANTIC_OPEN_EDB_SCHEMA_ID.to_owned()),
        schema_version: Some(SEMANTIC_OPEN_EDB_SCHEMA_VERSION.to_owned()),
        language_id: Some("zig".to_owned()),
        provider_id: Some("asp-zig".to_owned()),
        entities: vec![WorkspaceEdbEntityFact {
            entity_id: "zig1".to_owned(),
            file_id: 0,
            path: "src/zig_demo.zig".to_owned(),
            line: 1,
            name: "buildTargetMatrix".to_owned(),
            language: "zig".to_owned(),
            kind: "function".to_owned(),
            language_kind: "comptime_fn".to_owned(),
            source: "native-parser".to_owned(),
            attrs: Vec::new(),
        }],
        edges: Vec::new(),
    });

    // Query BOTH dynamic Special `(zig_comptime_fn ...)` AND General `(callable ... "zig" ...)`
    // AND verify Helm TAB completion immediately advertises `zig_comptime_fn`!
    let res_zig_special = execute_package_search_dsl_structured(
        &runtime,
        &scheme_dir,
        &SearchDslExecutionRequest {
            index: &warm_index,
            query: "(ascent-fragment ((answer name) <-- (zig_comptime_fn _ _ name _)))",
            output_limit: 32,
        },
    )
    .expect("dynamic zig_comptime_fn query");
    assert_eq!(res_zig_special.rows.len(), 1);
    assert_eq!(res_zig_special.rows[0].cells, vec!["buildTargetMatrix"]);

    let res_zig_general = execute_package_search_dsl_structured(
        &runtime,
        &scheme_dir,
        &SearchDslExecutionRequest {
            index: &warm_index,
            query: "(ascent-fragment ((answer name) <-- (callable _ _ \"zig\" name _)))",
            output_limit: 32,
        },
    )
    .expect("dynamic zig general callable query");
    assert_eq!(res_zig_general.rows.len(), 1);
    assert_eq!(res_zig_general.rows[0].cells, vec!["buildTargetMatrix"]);

    let completions = project_package_search_dsl_completions(
        &runtime,
        &scheme_dir,
        "(ascent-fragment ((a n) <-- (zig_",
    )
    .expect("dynamic zig completions");
    assert!(
        completions.iter().any(|c| c.form == "zig_comptime_fn"),
        "Helm TAB completions must dynamically expose `zig_comptime_fn`, got {completions:?}"
    );

    println!(
        "\n=== ASP HTTP -> MARLIN 5+1 LANGUAGE E2E BENCHMARK SUMMARY ===\n\
         Live providers booted         : {:?} (+ dynamic hot-registered `zig`)\n\
         Cold 5-lang HTTP hydration    : {} files, {} AST entities, ASP inspect={}us, total={}us\n\
         Warm BLAKE3 cache restore     : {} files, {} entities, HTTP calls=0, total={}us\n\
         5-lang General callable hits  : {} rows across {:?}\n\
         Hot zero-JSON C-ABI query     : p50={}us, p95={}us (30 iterations, pre-bound exports)\n\
         Package-lookup C-ABI query    : p50={}us, p95={}us (20 iterations, per-call export lookup)\n\
         Dynamic `zig` O(1) leverage   : targetedSchema + zig_comptime_fn + General callable + Helm TAB OK\n\
         ===============================================================",
        gateway.providers,
        cold_receipt.http_inspected_files,
        cold_receipt.http_ingested_entities,
        cold_receipt.asp_inspect_micros,
        cold_receipt.total_elapsed_micros,
        warm_receipt.restored_from_cache_files,
        warm_receipt.restored_from_cache_entities,
        warm_receipt.total_elapsed_micros,
        res_callable.rows.len(),
        langs_hit,
        hot_p50_us,
        hot_p95_us,
        pkg_p50_us,
        pkg_p95_us,
    );
}

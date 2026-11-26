// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Link products for the Linked-AOT `marlin-scheme-host-aot` daemon binary.

use std::{
    env, fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use gerbil_scheme_aot_build::{
    ProgramArchiveContract, ProgramArchiveRequest, build_program_archive_with_contract,
    discover_gambit_gsc_from_env,
};
use serde_json::Value;

use crate::archive::{CargoSourceObserver, SchemeProgram, scheme_link_manifest};

/// Tight upper bound on the linked `programs/graph-engine` closure (currently 564 modules
/// including the 2 Rust SDK carriers and Higher-Order Governance POO/ASCENT/Temporal/Cedar)
/// so kitchen-sink imports fail closed at build time.
pub const GRAPH_ENGINE_MAX_MODULES: usize = 590;

/// Modules that MUST be present in the linked `graph-engine` AOT closure.
pub const GRAPH_ENGINE_REQUIRED_MODULES: &[&str] = &[
    "gerbil-scheme-rust/scheme/runtime",
    "tao3k/yuanxu/module-system/source/autoload-declarations",
    "marlin-agent-core/policy/higher-order-governance",
];

/// Exact modules forbidden from ever re-entering the `graph-engine` AOT closure.
pub const GRAPH_ENGINE_FORBIDDEN_MODULES: &[&str] = &[
    "marlin-agent-core/interface",
    "marlin-agent-core/init",
    "marlin-agent-core/ui/helm-workbench",
];

/// Module prefixes forbidden from ever re-entering the `graph-engine` AOT closure.
pub const GRAPH_ENGINE_FORBIDDEN_MODULE_PREFIXES: &[&str] = &[
    "orgize/",
    "gerbil-parser/",
    "marlin-agent-core/ui-helm/",
    "marlin-agent-core/search-dsl/",
    "marlin-agent-core/search-engine/",
    "marlin-agent-core/gitops/",
    "tao3k/agentic-ai-lab/modules/module-authoring/",
];

/// Validate the linked `graph-engine` program manifest fail-closed against module-count,
/// required-module, exact-forbidden, and forbidden-prefix contracts.
///
/// # Errors
/// Returns a human-readable violation report if the manifest is unreadable, malformed,
/// exceeds [`GRAPH_ENGINE_MAX_MODULES`], omits any module in [`GRAPH_ENGINE_REQUIRED_MODULES`],
/// or pulls in any module matching [`GRAPH_ENGINE_FORBIDDEN_MODULES`] or
/// [`GRAPH_ENGINE_FORBIDDEN_MODULE_PREFIXES`].
pub fn validate_graph_engine_program_manifest(manifest_path: &Path) -> Result<usize, String> {
    let raw = fs::read_to_string(manifest_path).map_err(|err| {
        format!(
            "read graph-engine manifest {}: {err}",
            manifest_path.display()
        )
    })?;
    let plan: Value = serde_json::from_str(&raw).map_err(|err| {
        format!(
            "parse graph-engine manifest {}: {err}",
            manifest_path.display()
        )
    })?;
    let modules = plan
        .get("modules")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "graph-engine manifest {} has no modules array",
                manifest_path.display()
            )
        })?;

    let mut module_ids = Vec::with_capacity(modules.len());
    for (idx, row) in modules.iter().enumerate() {
        let id = row.get("module").and_then(Value::as_str).ok_or_else(|| {
            format!(
                "graph-engine manifest {} module[{idx}] is missing string `module` field",
                manifest_path.display()
            )
        })?;
        module_ids.push(id);
    }

    let mut violations = Vec::new();
    if module_ids.len() > GRAPH_ENGINE_MAX_MODULES {
        violations.push(format!(
            "module count {} exceeds GRAPH_ENGINE_MAX_MODULES ({GRAPH_ENGINE_MAX_MODULES})",
            module_ids.len()
        ));
    }

    for required in GRAPH_ENGINE_REQUIRED_MODULES {
        if !module_ids.contains(required) {
            violations.push(format!("missing required module `{required}`"));
        }
    }

    for &id in &module_ids {
        if GRAPH_ENGINE_FORBIDDEN_MODULES.contains(&id) {
            violations.push(format!("forbidden module `{id}` present in AOT closure"));
        }
        if let Some(prefix) = GRAPH_ENGINE_FORBIDDEN_MODULE_PREFIXES
            .iter()
            .find(|&&prefix| id.starts_with(prefix))
        {
            violations.push(format!(
                "forbidden module prefix `{prefix}` matched by `{id}` in AOT closure"
            ));
        }
    }

    if !violations.is_empty() {
        return Err(format!(
            "graph-engine AOT closure fence rejected {} ({} violation(s)):\n  - {}",
            manifest_path.display(),
            violations.len(),
            violations.join("\n  - ")
        ));
    }

    Ok(module_ids.len())
}

/// Link the GraphEngine Oracle + Fleet Controller program published by `just scheme-build`.
pub fn emit_graph_engine_link_directives() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    let manifest =
        scheme_link_manifest(SchemeProgram::new("graph-engine"), &out_dir.join("program"))
            .expect("read the graph-engine link products; run just scheme-build first");
    println!("cargo:rerun-if-changed={}", manifest.display());
    validate_graph_engine_program_manifest(&manifest)
        .expect("graph-engine AOT closure contract violation");
    let gsc = discover_gambit_gsc_from_env().expect("paired Gambit compiler");
    let receipt = build_program_archive_with_contract(
        ProgramArchiveRequest {
            manifest: &manifest,
            gsc: &gsc,
            archive_name: "marlin_graph_engine",
            linker_name: "marlin_graph_engine",
            out_dir: &out_dir.join("archive"),
        },
        ProgramArchiveContract {
            // Bound the slimmed GraphEngine closure (515 modules with SDK carriers after
            // decoupling `marlin-agent-core/interface` and `ui/helm-workbench`) so
            // kitchen-sink imports cannot silently bloat Linked-AOT binary size or boot latency.
            max_modules: NonZeroUsize::new(GRAPH_ENGINE_MAX_MODULES)
                .expect("nonzero module budget"),
            required_modules: GRAPH_ENGINE_REQUIRED_MODULES,
            forbidden_modules: GRAPH_ENGINE_FORBIDDEN_MODULES,
            linker_main_symbol: "gerbil_scheme_rust_program_main",
            additional_objects: &[],
            native_headers: &[],
        },
        &CargoSourceObserver,
    )
    .expect("link the selected GraphEngine Scheme graph");
    for directive in receipt.cargo_directives {
        println!("{}", directive.line());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write_temp_manifest(tag: &str, module_ids: &[String]) -> PathBuf {
        let path = env::temp_dir().join(format!(
            "marlin-graph-engine-fence-{}-{tag}.json",
            std::process::id()
        ));
        let modules: Vec<Value> = module_ids
            .iter()
            .map(|id| json!({"module": id, "system": false, "scm": "/dev/null"}))
            .collect();
        let payload = json!({
            "schema": "gerbil-scheme-rust.aot-program.v1",
            "modules": modules,
        });
        fs::write(&path, serde_json::to_vec(&payload).expect("serialize"))
            .expect("write temp manifest");
        path
    }

    fn base_valid_modules() -> Vec<String> {
        vec![
            "gerbil/runtime/gambit".to_owned(),
            "gerbil-scheme-rust/scheme/runtime".to_owned(),
            "tao3k/yuanxu/module-system/source/autoload-declarations".to_owned(),
            "marlin-agent-core/policy/higher-order-governance".to_owned(),
            "marlin-agent-core/graph-engine/entry".to_owned(),
        ]
    }

    #[test]
    fn valid_slim_manifest_passes_closure_fence() {
        let path = write_temp_manifest("valid", &base_valid_modules());
        let res = validate_graph_engine_program_manifest(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(res, Ok(5));
    }

    #[test]
    fn forbidden_prefix_modules_fail_closed() {
        for bad_module in [
            "orgize/languages/org/parser",
            "gerbil-parser/src/schema",
            "marlin-agent-core/ui-helm/session",
            "marlin-agent-core/search-dsl/engine",
            "marlin-agent-core/search-engine/index",
            "marlin-agent-core/gitops/catalog",
            "tao3k/agentic-ai-lab/modules/module-authoring/main",
        ] {
            let mut mods = base_valid_modules();
            mods.push(bad_module.to_owned());
            let tag = bad_module.replace('/', "_");
            let path = write_temp_manifest(&tag, &mods);
            let err = validate_graph_engine_program_manifest(&path)
                .expect_err(&format!("{bad_module} must fail closed"));
            let _ = fs::remove_file(&path);
            assert!(
                err.contains("forbidden module prefix") && err.contains(bad_module),
                "unexpected error for {bad_module}: {err}"
            );
        }
    }

    #[test]
    fn exact_forbidden_kitchen_sink_modules_fail_closed() {
        for bad_module in [
            "marlin-agent-core/interface",
            "marlin-agent-core/init",
            "marlin-agent-core/ui/helm-workbench",
        ] {
            let mut mods = base_valid_modules();
            mods.push(bad_module.to_owned());
            let tag = format!("exact_{}", bad_module.replace('/', "_"));
            let path = write_temp_manifest(&tag, &mods);
            let err = validate_graph_engine_program_manifest(&path)
                .expect_err(&format!("{bad_module} must fail closed"));
            let _ = fs::remove_file(&path);
            assert!(
                err.contains("forbidden module") && err.contains(bad_module),
                "unexpected error for {bad_module}: {err}"
            );
        }
    }

    #[test]
    fn missing_required_autoload_declarations_fails_closed() {
        let mods = vec![
            "gerbil/runtime/gambit".to_owned(),
            "gerbil-scheme-rust/scheme/runtime".to_owned(),
        ];
        let path = write_temp_manifest("missing_req", &mods);
        let err = validate_graph_engine_program_manifest(&path)
            .expect_err("missing autoload-declarations must fail closed");
        let _ = fs::remove_file(&path);
        assert!(
            err.contains("missing required module") && err.contains("autoload-declarations"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn over_budget_module_count_fails_closed() {
        let mut mods = base_valid_modules();
        while mods.len() <= GRAPH_ENGINE_MAX_MODULES {
            mods.push(format!(
                "marlin-agent-core/graph-engine/extra-{}",
                mods.len()
            ));
        }
        let path = write_temp_manifest("over_budget", &mods);
        let err = validate_graph_engine_program_manifest(&path)
            .expect_err("over-budget closure must fail closed");
        let _ = fs::remove_file(&path);
        assert!(
            err.contains("exceeds GRAPH_ENGINE_MAX_MODULES"),
            "unexpected error: {err}"
        );
    }
}

// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[test]
fn gerbil_scheme_does_not_depend_on_runtime_or_llm_crates() {
    let manifest: toml::Value =
        toml::from_str(include_str!("../../Cargo.toml")).expect("parse crate manifest");
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("manifest should contain dependency table");

    for forbidden in ["marlin-agent-runtime", "marlin-agent-stream", "litellm-rs"] {
        assert!(
            !dependencies.contains_key(forbidden),
            "marlin-gerbil-scheme must not production-depend on {forbidden}; keep real LLM integration in marlin-agent-stream/runtime"
        );
    }
}

#[test]
fn gerbil_scheme_does_not_depend_on_rust_side_scheme_source_parsers() {
    let manifest: toml::Value =
        toml::from_str(include_str!("../../Cargo.toml")).expect("parse crate manifest");
    let forbidden = ["lexpr", "serde-lexpr", "pest", "nom", "chumsky", "winnow"];

    for table_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(dependencies) = manifest.get(table_name).and_then(toml::Value::as_table) else {
            continue;
        };

        for crate_name in forbidden {
            assert!(
                !dependencies.contains_key(crate_name),
                "marlin-gerbil-scheme must not productionize Rust-side Scheme source parsing via {crate_name}; use Gerbil Scheme types -> native ABI -> Rust types"
            );
        }
    }
}

#[test]
fn graph_engine_scheme_sources_stay_rfc44_poo_and_free_of_kitchen_sink_imports() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scheme/graph-engine");
    let mut ss_files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .expect("read scheme/graph-engine directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("ss"))
        .collect();
    ss_files.sort();
    assert!(
        ss_files.len() >= 8,
        "expected at least 8 Scheme source files under {}, found {}",
        dir.display(),
        ss_files.len()
    );

    let forbidden_tokens = [
        "bind-these: #f",
        ":marlin-agent-core/interface",
        ":marlin-agent-core/init",
        "helm-workbench",
        "ui-helm",
        "orgize",
        "gerbil-parser",
        "search-dsl",
        "search-engine",
        "module-authoring",
    ];

    for file in ss_files {
        let raw = std::fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        // Strip Scheme line comments so explanatory docstrings remain allowed while
        // any executable form or import referencing forbidden modules fails closed.
        let code_only: String = raw
            .lines()
            .map(|line| line.split(';').next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        for token in forbidden_tokens {
            assert!(
                !code_only.contains(token),
                "{} must not reference `{token}` (violates RFC 44 POO style or Linked-AOT slim closure boundary)",
                file.display()
            );
        }
    }
}

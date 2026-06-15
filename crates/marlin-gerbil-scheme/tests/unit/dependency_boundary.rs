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

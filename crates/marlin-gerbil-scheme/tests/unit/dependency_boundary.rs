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

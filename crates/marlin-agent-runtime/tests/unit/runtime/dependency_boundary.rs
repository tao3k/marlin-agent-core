use std::fs;

#[test]
fn runtime_does_not_depend_on_stream_litellm_or_gerbil_native_crates() {
    let manifest_path = format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(&manifest_path).expect("runtime Cargo.toml is readable");
    let manifest: toml::Table = toml::from_str(&manifest).expect("runtime Cargo.toml parses");
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("runtime dependencies table exists");

    for forbidden in ["marlin-agent-stream", "marlin-gerbil-scheme", "litellm-rs"] {
        assert!(
            !dependencies.contains_key(forbidden),
            "runtime normal dependencies must not include {forbidden}"
        );
    }
}

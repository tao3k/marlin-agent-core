fn main() {
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    marlin_rust_project_harness_policy::complete_marlin_rust_project_harness_gate_from_env(
        project_root,
    );
    marlin_deck_runtime_native_build::emit_deck_runtime_native_link_directives();
}

fn main() {
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let policy =
        marlin_rust_project_harness_policy::rust_project_harness_policy_for_project(project_root);
    let harness_report =
        rust_lang_project_harness::assert_rust_project_harness_downstream_policy_from_env(&policy);
    marlin_rust_project_harness_policy::complete_build_gate(
        project_root,
        policy.config(),
        harness_report,
    );
    marlin_deck_runtime_native_build::emit_deck_runtime_native_link_directives();
}

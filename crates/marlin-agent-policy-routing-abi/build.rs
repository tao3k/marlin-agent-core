fn main() {
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    marlin_rust_project_harness_policy::assert_marlin_rust_project_harness_build_check_from_env(
        project_root,
    );
    marlin_gerbil_aot_link::emit_agent_policy_routing_abi_link_directives();
}

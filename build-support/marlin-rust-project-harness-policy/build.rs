fn main() {
    let config = rust_lang_project_harness::rust_harness_config_for_project(std::path::Path::new(
        env!("CARGO_MANIFEST_DIR"),
    ));
    rust_lang_project_harness::assert_rust_project_harness_cargo_check_clean_from_env_with_config(
        &config,
    );
}

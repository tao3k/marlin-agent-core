use std::path::Path;

fn main() {
    marlin_rust_project_harness_policy::assert_marlin_rust_project_harness_build_check_from_env(
        Path::new(env!("CARGO_MANIFEST_DIR")),
    );
}

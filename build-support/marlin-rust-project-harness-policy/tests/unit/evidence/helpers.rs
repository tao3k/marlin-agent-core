use std::{fs, path::PathBuf};

use marlin_rust_project_harness_policy::{
    build_verification_policy_receipt, rust_project_harness_config_for_project,
};
use rust_lang_project_harness::plan_rust_project_verification_with_config;

pub(crate) use crate::workspace::workspace_root;

pub(super) fn workspace_crates() -> Vec<PathBuf> {
    let crates_dir = workspace_root().join("crates");
    let mut crates = fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("read workspace crates dir {crates_dir:?}: {error}"))
        .map(|entry| entry.expect("workspace crate entry").path())
        .filter(|path| path.join("Cargo.toml").is_file() && path.join("src/lib.rs").is_file())
        .collect::<Vec<_>>();

    crates.sort();
    crates
}

pub(super) fn runtime_verification_policy_receipt()
-> marlin_rust_project_harness_policy::RustProjectHarnessVerificationPolicyReceipt {
    let project_root = workspace_root().join("crates/marlin-agent-runtime");
    let config = rust_project_harness_config_for_project(&project_root);
    let plan = plan_rust_project_verification_with_config(&project_root, &config)
        .expect("runtime crate should plan rust harness verification");

    build_verification_policy_receipt("marlin-agent-runtime", &project_root, &config, &plan)
}

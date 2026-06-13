use std::path::{Path, PathBuf};

use marlin_agent_test_support::assert_three_layer_testing_system_for_workspace;

#[tokio::test]
async fn test_support_three_layer_testing_system_covers_workspace_packages_without_live_llm() {
    let report = assert_three_layer_testing_system_for_workspace(workspace_root()).await;

    assert!(report.test_run.is_non_live_success());
    assert_eq!(report.test_run.non_live_failed_count(), 0);
    assert_eq!(report.test_run.ignored_live_external_count(), 1);
    assert!(report.agent_runtime.is_success());
    assert_eq!(
        report.agent_runtime.sub_agent_litellm_model_id,
        "anthropic/claude-opus-4-8"
    );
    assert_eq!(report.agent_runtime.stream_chunk_count, 1);
    assert!(report.agent_runtime.memory_visibility_granted);
    assert!(report.covers_package("marlin-agent-test-support"));
    let receipt = report
        .package_receipt("marlin-agent-test-support")
        .expect("test-support package receipt should exist");
    assert!(receipt.is_success());
    assert!(receipt.agent_core_package);
    assert!(receipt.manifest_present);
    assert!(receipt.library_root_present);
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("test-support crate should live under workspace/crates")
        .to_path_buf()
}

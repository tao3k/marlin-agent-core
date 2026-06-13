use std::path::Path;

use marlin_agent_test_support::assert_package_three_layer_coverage;

#[test]
fn harness_consumes_test_support_three_layer_package_coverage() {
    let receipt = assert_package_three_layer_coverage(Path::new(env!("CARGO_MANIFEST_DIR")));

    assert_eq!(receipt.package_name, "marlin-agent-harness");
    assert!(receipt.is_success());
    assert!(receipt.agent_core_package);
    assert!(receipt.manifest_present);
    assert!(receipt.library_root_present);
}

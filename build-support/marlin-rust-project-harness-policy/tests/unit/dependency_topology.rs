use std::{fs, path::Path};

use marlin_rust_project_harness_policy::{
    ExternalDependencyTopologyReceiptStatus, consume_external_dependency_topology_receipt,
};

#[test]
fn dependency_topology_receipt_is_optional_for_hermetic_builds() {
    let receipt = consume_external_dependency_topology_receipt(
        workspace_root(),
        "rust-lang-project-harness",
        None,
    );

    assert!(receipt.is_success(), "{}", receipt.agent_message());
    assert_eq!(
        receipt.status(),
        &ExternalDependencyTopologyReceiptStatus::NotConfigured
    );
    assert!(
        receipt
            .agent_next_action()
            .expect("missing receipt should guide the agent")
            .contains("no external topology receipt is required")
    );
    assert!(!receipt.blocks_build(false));
}

#[test]
fn dependency_topology_receipt_accepts_external_verified_receipt() {
    let tempdir = tempfile::tempdir().expect("create topology receipt tempdir");
    let receipt_path = tempdir.path().join("dependency-topology.json");
    fs::write(
        &receipt_path,
        r#"{
  "schema_id": "agent.semantic-protocols.dependency-topology-receipt",
  "schema_version": "1",
  "package_name": "rust-lang-project-harness",
  "producer": "asp.graph-turbo",
  "status": "verified",
  "evidence_sources": ["graph-turbo:dependency-topology"],
  "agent_next_action": "continue",
  "verification_command": "asp rust search pipe 'dependency topology rust-lang-project-harness' --workspace . --view seeds"
}"#,
    )
    .expect("write topology receipt fixture");

    let receipt = consume_external_dependency_topology_receipt(
        workspace_root(),
        "rust-lang-project-harness",
        Some(&receipt_path),
    );

    assert!(receipt.is_success(), "{}", receipt.agent_message());
    assert_eq!(
        receipt.status(),
        &ExternalDependencyTopologyReceiptStatus::Verified
    );
    assert_eq!(receipt.producer(), Some("asp.graph-turbo"));
    assert_eq!(
        receipt.evidence_sources(),
        &["graph-turbo:dependency-topology".to_owned()]
    );
}

#[test]
fn dependency_topology_receipt_rejects_mismatched_package() {
    let tempdir = tempfile::tempdir().expect("create topology receipt tempdir");
    let receipt_path = tempdir.path().join("dependency-topology.json");
    fs::write(
        &receipt_path,
        r#"{
  "schema_id": "agent.semantic-protocols.dependency-topology-receipt",
  "schema_version": "1",
  "package_name": "other-package",
  "producer": "asp.graph-turbo",
  "status": "verified"
}"#,
    )
    .expect("write mismatched topology receipt fixture");

    let receipt = consume_external_dependency_topology_receipt(
        workspace_root(),
        "rust-lang-project-harness",
        Some(&receipt_path),
    );

    assert!(!receipt.is_success());
    assert!(!receipt.blocks_build(false));
    assert!(receipt.blocks_build(true));
    assert_eq!(
        receipt.status(),
        &ExternalDependencyTopologyReceiptStatus::PackageMismatch {
            expected: "rust-lang-project-harness".to_owned(),
            actual: "other-package".to_owned(),
        }
    );
    assert!(
        receipt
            .agent_message()
            .contains("[MARLIN-DEPTOPO-IFACE-001]")
    );
}

#[test]
fn dependency_topology_receipt_rejects_invalid_schema() {
    let tempdir = tempfile::tempdir().expect("create topology receipt tempdir");
    let receipt_path = tempdir.path().join("dependency-topology.json");
    fs::write(
        &receipt_path,
        r#"{
  "schema_id": "example.invalid",
  "schema_version": "1",
  "package_name": "rust-lang-project-harness",
  "status": "verified"
}"#,
    )
    .expect("write invalid schema topology receipt fixture");

    let receipt = consume_external_dependency_topology_receipt(
        workspace_root(),
        "rust-lang-project-harness",
        Some(&receipt_path),
    );

    assert!(!receipt.is_success());
    assert!(matches!(
        receipt.status(),
        ExternalDependencyTopologyReceiptStatus::InvalidReceipt(_)
    ));
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("policy crate should live under workspace/build-support")
}

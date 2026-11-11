// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::fs;

use crate::policy::{
    ExternalDependencyTopologyReceiptStatus, consume_external_dependency_topology_receipt,
};

use crate::workspace::workspace_root;

#[test]
fn dependency_topology_receipt_is_optional_for_hermetic_builds() {
    let receipt = consume_external_dependency_topology_receipt(workspace_root(), "asp-rust", None);

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
  "package_name": "asp-rust",
  "producer": "asp.graph-turbo",
  "status": "verified",
  "evidence_sources": ["graph-turbo:dependency-topology"],
  "agent_next_action": "continue",
  "verification_command": "asp rust search pipe 'dependency topology asp-rust' --workspace . --view seeds"
}"#,
    )
    .expect("write topology receipt fixture");

    let receipt = consume_external_dependency_topology_receipt(
        workspace_root(),
        "asp-rust",
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
        "asp-rust",
        Some(&receipt_path),
    );

    assert!(!receipt.is_success());
    assert!(!receipt.blocks_build(false));
    assert!(receipt.blocks_build(true));
    assert_eq!(
        receipt.status(),
        &ExternalDependencyTopologyReceiptStatus::PackageMismatch {
            expected: "asp-rust".to_owned(),
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
  "package_name": "asp-rust",
  "status": "verified"
}"#,
    )
    .expect("write invalid schema topology receipt fixture");

    let receipt = consume_external_dependency_topology_receipt(
        workspace_root(),
        "asp-rust",
        Some(&receipt_path),
    );

    assert!(!receipt.is_success());
    assert!(matches!(
        receipt.status(),
        ExternalDependencyTopologyReceiptStatus::InvalidReceipt(_)
    ));
}

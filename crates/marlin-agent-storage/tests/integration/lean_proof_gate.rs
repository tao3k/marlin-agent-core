use crate::paths::workspace_root;
use std::process::Command;

#[test]
fn storage_artifact_pointer_cas_lean_proof_builds() {
    let proof_dir = workspace_root().join("proof-support/lean");
    let scenario = proof_dir.join("Scenarios/storage-artifact-pointer-cas.org");
    let receipt = proof_dir.join("Scenarios/storage-artifact-pointer-cas.toml");

    assert!(
        proof_dir.join("lakefile.lean").is_file(),
        "missing Lean proof package at {}",
        proof_dir.display()
    );
    assert!(
        scenario.is_file(),
        "missing storage CAS proof scenario fixture at {}",
        scenario.display()
    );
    assert!(
        receipt.is_file(),
        "missing storage CAS proof receipt fixture at {}",
        receipt.display()
    );

    let receipt_text =
        std::fs::read_to_string(&receipt).expect("proof receipt fixture should be readable");
    let receipt_table = receipt_text
        .parse::<toml::Table>()
        .expect("proof receipt fixture should be valid TOML");
    let proof = receipt_table
        .get("proof")
        .and_then(toml::Value::as_table)
        .expect("proof receipt fixture should contain [proof]");

    assert_eq!(
        proof.get("engine").and_then(toml::Value::as_str),
        Some("lean")
    );
    assert_eq!(
        proof.get("module").and_then(toml::Value::as_str),
        Some("MarlinProof.Storage.ArtifactPointer")
    );
    assert!(
        proof
            .get("theorems")
            .and_then(toml::Value::as_array)
            .is_some_and(|theorems| theorems.len() >= 6),
        "proof receipt should list the storage CAS theorem names"
    );

    let output = Command::new("lake")
        .arg("build")
        .current_dir(&proof_dir)
        .output()
        .expect("lake must be installed to run storage proof gates");

    assert!(
        output.status.success(),
        "storage Lean proof gate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

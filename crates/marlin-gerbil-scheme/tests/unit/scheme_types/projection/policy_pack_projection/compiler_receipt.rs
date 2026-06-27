use super::{
    GerbilPooLoopProgramCompilerBoundary, GerbilPooLoopProgramCompilerOwner,
    GerbilPooLoopProgramCompilerSerializationBoundary,
    decode_gerbil_poo_loop_program_compiler_receipt, poo_loop_program_compiler_envelope,
    poo_loop_program_compiler_fixture, poo_loop_program_compiler_fixture_with_resolved_policy_pack,
    poo_loop_program_compiler_registry, resolved_loop_policy_audit_fixture_without_merge_receipts,
    resolved_loop_policy_pack_fixture_with_audit,
};
use marlin_agent_protocol::{LoopPolicyMixinId, SlotMergeAlgebra};
use std::time::{Duration, Instant};

#[test]
fn poo_loop_program_compiler_receipt_decodes_program_bound_to_resolved_pack() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(poo_loop_program_compiler_fixture([7; 32]));

    let receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("POO loop program compiler receipt decodes");

    assert!(receipt.has_current_schema());
    assert_eq!(receipt.profile_id.as_str(), "runtime-reactive-tool-loop");
    assert_eq!(
        receipt.compiler_owner,
        GerbilPooLoopProgramCompilerOwner::GerbilPooFlow
    );
    assert_eq!(
        receipt.scheme_boundary,
        GerbilPooLoopProgramCompilerBoundary::SchemeTypesToRustTypes
    );
    assert_eq!(
        receipt.serialization_boundary,
        GerbilPooLoopProgramCompilerSerializationBoundary::RustOwnedCliTraceCrossProcess
    );
    assert_eq!(
        receipt.loop_program.policy_epoch,
        receipt.resolved_policy_pack.policy_epoch
    );
    assert_eq!(
        receipt.loop_program.policy_digest,
        receipt.resolved_policy_pack.policy_digest
    );
    assert_eq!(receipt.loop_program.mechanism_policies.len(), 3);
    assert_eq!(receipt.loop_program.transitions.len(), 6);
    assert!(
        !format!("{:?}", receipt.scheme_boundary)
            .to_ascii_lowercase()
            .contains("json")
    );
    assert!(receipt.resolved_policy_pack.has_current_schema());
    assert!(receipt.loop_program.has_current_schema());
    assert!(!receipt.resolved_policy_pack.hot.graph_nodes.is_empty());
    assert!(!receipt.resolved_policy_pack.audit.provenance.is_empty());
    assert!(!receipt.resolved_policy_pack.audit.linearization.is_empty());
    assert!(!receipt.resolved_policy_pack.audit.forced_slots.is_empty());
    assert!(!receipt.resolved_policy_pack.audit.merge_receipts.is_empty());
    assert!(
        receipt
            .resolved_policy_pack
            .audit
            .uses_mixin(&LoopPolicyMixinId::new("artifact-policy"))
    );
    assert!(
        receipt
            .resolved_policy_pack
            .audit
            .covers_slot_merge_algebras([
                SlotMergeAlgebra::Union,
                SlotMergeAlgebra::Intersection,
                SlotMergeAlgebra::Min,
                SlotMergeAlgebra::OrderedAppend,
                SlotMergeAlgebra::ConflictError,
            ])
    );
}

#[test]
fn poo_loop_program_compiler_receipt_projection_stays_in_process() {
    let registry = poo_loop_program_compiler_registry();
    let envelopes = (0..256)
        .map(|_| poo_loop_program_compiler_envelope(poo_loop_program_compiler_fixture([7; 32])))
        .collect::<Vec<_>>();

    let started = Instant::now();
    for envelope in &envelopes {
        let receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, envelope)
            .expect("POO loop program compiler receipt decodes through the Rust ABI surface");
        assert_eq!(
            receipt.compiler_owner,
            GerbilPooLoopProgramCompilerOwner::GerbilPooFlow
        );
        assert_eq!(
            receipt.scheme_boundary,
            GerbilPooLoopProgramCompilerBoundary::SchemeTypesToRustTypes
        );
        assert_eq!(
            receipt.serialization_boundary,
            GerbilPooLoopProgramCompilerSerializationBoundary::RustOwnedCliTraceCrossProcess
        );
    }
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_millis(250),
        "POO loop compiler receipt projection exceeded in-process ABI budget: {elapsed:?}"
    );
}

#[test]
fn poo_loop_program_compiler_receipt_rejects_digest_drift() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(poo_loop_program_compiler_fixture([8; 32]));

    let error = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect_err("digest drift should be rejected by Rust projection");
    super::assert_rust_projection_decode_error(error, "policy digest");
}

#[test]
fn poo_loop_program_compiler_receipt_rejects_missing_merge_receipts() {
    let registry = poo_loop_program_compiler_registry();
    let payload = poo_loop_program_compiler_fixture_with_resolved_policy_pack(
        [7; 32],
        resolved_loop_policy_pack_fixture_with_audit(
            1,
            resolved_loop_policy_audit_fixture_without_merge_receipts(),
        ),
    );
    let envelope = poo_loop_program_compiler_envelope(payload);

    let error = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect_err("missing merge receipts should be rejected by Rust projection");
    super::assert_rust_projection_decode_error(error, "merge receipts");
}

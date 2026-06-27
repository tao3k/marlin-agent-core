use super::{
    decode_gerbil_resolved_loop_policy_pack, resolved_loop_policy_pack_envelope,
    resolved_loop_policy_pack_fixture, resolved_loop_policy_pack_registry,
};
use marlin_agent_protocol::{LoopPolicyMixinId, SlotMergeAlgebra};

#[test]
fn resolved_loop_policy_pack_decodes_hot_and_audit_ir_without_json_boundary() {
    let registry = resolved_loop_policy_pack_registry();
    let envelope = resolved_loop_policy_pack_envelope(resolved_loop_policy_pack_fixture(1));

    let pack = decode_gerbil_resolved_loop_policy_pack(&registry, &envelope)
        .expect("resolved loop policy pack decodes");

    assert!(pack.has_current_schema());
    assert_eq!(pack.policy_epoch.get(), 42);
    assert_eq!(pack.policy_digest.as_bytes(), &[7_u8; 32]);
    assert_eq!(pack.hot.capability_mask, 0b101);
    assert_eq!(pack.hot.graph_nodes.len(), 1);
    assert_eq!(pack.hot.graph_nodes[0].node_id.get(), 1);
    assert_eq!(pack.hot.continuation_table.len(), 1);
    assert_eq!(pack.audit.provenance.len(), 1);
    assert_eq!(pack.audit.provenance[0].winner_role.as_str(), "planner");
    assert!(
        pack.audit
            .uses_mixin(&LoopPolicyMixinId::new("reactive-tool-loop-base"))
    );
    assert!(
        pack.audit
            .uses_mixin(&LoopPolicyMixinId::new("artifact-policy"))
    );
    assert!(pack.audit.covers_slot_merge_algebras([
        SlotMergeAlgebra::Union,
        SlotMergeAlgebra::Intersection,
        SlotMergeAlgebra::Min,
        SlotMergeAlgebra::OrderedAppend,
        SlotMergeAlgebra::ConflictError,
    ]));
    assert_eq!(pack.audit.forced_slots.len(), 1);
    assert_eq!(pack.audit.merge_receipts.len(), 5);
}

#[test]
fn resolved_loop_policy_pack_rejects_schema_version_drift() {
    let registry = resolved_loop_policy_pack_registry();
    let envelope = resolved_loop_policy_pack_envelope(resolved_loop_policy_pack_fixture(99));

    let error = decode_gerbil_resolved_loop_policy_pack(&registry, &envelope)
        .expect_err("schema drift should be rejected by Rust projection");
    super::assert_rust_projection_decode_error(error, "schema version 99");
}

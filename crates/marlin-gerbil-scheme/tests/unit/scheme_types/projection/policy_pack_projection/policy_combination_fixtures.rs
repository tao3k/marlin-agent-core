use super::{
    GERBIL_POLICY_MIXIN_DEFINITION_SCHEMA_ID, GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID,
    GERBIL_POLICY_SLOT_MERGE_RECEIPT_SCHEMA_ID, GerbilSchemeValue,
    poo_loop_program_compiler_fixture_with_profile_pack_and_program,
};

pub(crate) fn policy_combination_mixin_stack_compiler_fixture() -> GerbilSchemeValue {
    policy_combination_mixin_stack_compiler_fixture_with_mixin_count(7)
}

pub(crate) fn policy_combination_mixin_stack_compiler_fixture_with_mixin_count(
    mixin_count: usize,
) -> GerbilSchemeValue {
    let resolved_policy_pack = policy_combination_resolved_loop_policy_pack_fixture();
    let loop_program = policy_combination_loop_program_fixture([7; 32]);
    let compiler_receipt = poo_loop_program_compiler_fixture_with_profile_pack_and_program(
        "policy-combination/memory-rewrite-checker",
        resolved_policy_pack.clone(),
        loop_program.clone(),
    );

    GerbilSchemeValue::record([
        ("kind", GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID.into()),
        ("owner", "poo-flow.scheme".into()),
        ("compiler-owner", "gerbil-poo-flow".into()),
        (
            "profile-id",
            "policy-combination/memory-rewrite-checker".into(),
        ),
        ("policy-epoch", 15_i64.into()),
        ("policy-mixins", policy_combination_policy_mixins()),
        (
            "mixin-definitions",
            policy_combination_mixin_definitions_fixture(),
        ),
        ("mixin-count", i64::try_from(mixin_count).unwrap().into()),
        (
            "linearization",
            GerbilSchemeValue::vector([
                "memory".into(),
                "maker".into(),
                "rewrite".into(),
                "tool".into(),
                "checker".into(),
            ]),
        ),
        ("linearization-owner", "poo-flow.c3-c4".into()),
        (
            "slot-merge-receipts",
            policy_combination_slot_merge_receipts_fixture(),
        ),
        ("slot-merge-audit", policy_combination_merge_audit_fixture()),
        (
            "slot-merge-laws",
            GerbilSchemeValue::vector([
                "route_rules=ordered_append".into(),
                "observability=union".into(),
                "budget.max_attempts=min".into(),
                "capability=intersection".into(),
                "human_gates=union".into(),
                "exclusive_resource=conflict_error".into(),
            ]),
        ),
        ("slot-merge-owner", "poo-flow.slot-merge-algebra".into()),
        ("profile-spec", policy_combination_profile_spec_fixture()),
        ("resolved-policy-pack", resolved_policy_pack),
        ("loop-program", loop_program),
        ("compiler-receipt", compiler_receipt),
        ("scheme-boundary", "scheme-types-to-rust-types".into()),
        (
            "serialization-boundary",
            "rust-owned-cli-trace-cross-process".into(),
        ),
        ("rust-handler-manufactured", false.into()),
    ])
}

fn policy_combination_policy_mixins() -> GerbilSchemeValue {
    GerbilSchemeValue::vector([
        "memory-policy".into(),
        "maker-policy".into(),
        "dynamic-rewrite-policy".into(),
        "tool-policy".into(),
        "checker-policy".into(),
        "artifact-policy".into(),
        "trace-policy".into(),
    ])
}

fn policy_combination_mixin_definitions_fixture() -> GerbilSchemeValue {
    GerbilSchemeValue::vector([
        policy_mixin_definition_fixture("memory-policy", "memory", [31]),
        policy_mixin_definition_fixture("maker-policy", "maker", [32]),
        policy_mixin_definition_fixture("dynamic-rewrite-policy", "rewrite", [35]),
        policy_mixin_definition_fixture("tool-policy", "tool", [36]),
        policy_mixin_definition_fixture("checker-policy", "checker", [33, 34]),
        policy_mixin_definition_fixture("artifact-policy", "artifact", [35]),
        policy_mixin_definition_fixture("trace-policy", "trace", [32]),
    ])
}

fn policy_mixin_definition_fixture<const N: usize>(
    mixin_id: &str,
    role: &str,
    slot_ids: [i64; N],
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("kind", GERBIL_POLICY_MIXIN_DEFINITION_SCHEMA_ID.into()),
        ("mixin-id", mixin_id.into()),
        ("role", role.into()),
        (
            "slot-ids",
            GerbilSchemeValue::vector(slot_ids.into_iter().map(GerbilSchemeValue::from)),
        ),
        ("owner", "poo-flow.scheme".into()),
        ("scheme-boundary", "scheme-types-to-rust-types".into()),
        (
            "serialization-boundary",
            "rust-owned-cli-trace-cross-process".into(),
        ),
        ("rust-handler-manufactured", false.into()),
    ])
}

fn policy_combination_profile_spec_fixture() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        (
            "profile-id",
            "policy-combination/memory-rewrite-checker".into(),
        ),
        ("policy-epoch", 15_i64.into()),
        ("policy-mixins", policy_combination_policy_mixins()),
        (
            "linearization",
            GerbilSchemeValue::vector([
                "memory".into(),
                "maker".into(),
                "rewrite".into(),
                "tool".into(),
                "checker".into(),
            ]),
        ),
    ])
}

fn policy_combination_resolved_loop_policy_pack_fixture() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_version", 1_i64.into()),
        ("policy_epoch", 15_i64.into()),
        (
            "policy_digest",
            GerbilSchemeValue::vector((0..32).map(|_| GerbilSchemeValue::from(7_i64))),
        ),
        ("hot", policy_combination_hot_fixture()),
        ("audit", policy_combination_audit_fixture()),
    ])
}

fn policy_combination_hot_fixture() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("capability_mask", 7_i64.into()),
        ("human_gate_mask", 1_i64.into()),
        (
            "budget_caps",
            GerbilSchemeValue::record([
                ("max_attempts", 3_i64.into()),
                ("max_cost_units", 1000_i64.into()),
                ("max_wall_time_ms", 30000_i64.into()),
            ]),
        ),
        (
            "graph_nodes",
            GerbilSchemeValue::vector([
                GerbilSchemeValue::record([
                    ("node_id", 1_i64.into()),
                    ("executor_id", 21_i64.into()),
                    ("capability_mask", 1_i64.into()),
                    ("resource_class_id", 4_i64.into()),
                ]),
                GerbilSchemeValue::record([
                    ("node_id", 2_i64.into()),
                    ("executor_id", 22_i64.into()),
                    ("capability_mask", 2_i64.into()),
                    ("resource_class_id", 4_i64.into()),
                ]),
                GerbilSchemeValue::record([
                    ("node_id", 3_i64.into()),
                    ("executor_id", 23_i64.into()),
                    ("capability_mask", 4_i64.into()),
                    ("resource_class_id", 4_i64.into()),
                ]),
            ]),
        ),
        (
            "graph_edges",
            GerbilSchemeValue::vector([
                GerbilSchemeValue::record([("from", 1_i64.into()), ("to", 2_i64.into())]),
                GerbilSchemeValue::record([("from", 2_i64.into()), ("to", 3_i64.into())]),
            ]),
        ),
        (
            "route_index",
            GerbilSchemeValue::record([(
                "buckets",
                GerbilSchemeValue::vector([GerbilSchemeValue::record([
                    ("bucket_id", 1_i64.into()),
                    ("scope_mask", 255_i64.into()),
                    ("target_id", 3_i64.into()),
                ])]),
            )]),
        ),
        (
            "resource_classes",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("resource_class_id", 4_i64.into()),
                ("exclusive", true.into()),
            ])]),
        ),
        (
            "continuation_table",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([(
                "op",
                "stop_completed".into(),
            )])]),
        ),
        (
            "maker_profiles",
            GerbilSchemeValue::vector([GerbilSchemeValue::from(21_i64)]),
        ),
        (
            "checker_profiles",
            GerbilSchemeValue::vector([GerbilSchemeValue::from(22_i64)]),
        ),
    ])
}

fn policy_combination_audit_fixture() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("policy_mixins", policy_combination_policy_mixins()),
        (
            "provenance",
            GerbilSchemeValue::vector([
                slot_provenance_fixture(31, "memory", ["memory", "maker", "checker"], "ordered_append"),
                slot_provenance_fixture(32, "maker", ["memory", "maker", "checker"], "union"),
                slot_provenance_fixture(33, "checker", ["memory", "maker", "checker"], "min"),
                slot_provenance_fixture(34, "checker", ["maker", "rewrite", "tool", "checker"], "intersection"),
                slot_provenance_fixture(35, "checker", ["rewrite", "checker"], "union"),
                slot_provenance_fixture(36, "tool", ["tool", "checker"], "conflict_error"),
            ]),
        ),
        (
            "linearization",
            GerbilSchemeValue::vector([
                "memory".into(),
                "maker".into(),
                "rewrite".into(),
                "tool".into(),
                "checker".into(),
            ]),
        ),
        (
            "diagnostics",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("code", "policy-combination-matrix-ok".into()),
                ("severity", "info".into()),
            ])]),
        ),
        (
            "source_locations",
            GerbilSchemeValue::vector([GerbilSchemeValue::record([
                ("source_location_id", 2_i64.into()),
                (
                    "path",
                    "gerbil/src/config-interface/modules/policy-pack.ss".into(),
                ),
                ("line", 1_i64.into()),
                ("column", 1_i64.into()),
            ])]),
        ),
        (
            "explanation_strings",
            GerbilSchemeValue::vector([
                "policy combination matrix projects memory, maker, rewrite, tool, checker into typed loop program".into(),
            ]),
        ),
        (
            "forced_slots",
            GerbilSchemeValue::vector([
                forced_slot_fixture(31, "hot"),
                forced_slot_fixture(32, "hot"),
                forced_slot_fixture(33, "hot"),
                forced_slot_fixture(34, "hot"),
                forced_slot_fixture(35, "audit_only"),
                forced_slot_fixture(36, "hot"),
            ]),
        ),
        ("merge_receipts", policy_combination_merge_audit_fixture()),
    ])
}

fn slot_provenance_fixture<const N: usize>(
    slot_id: i64,
    winner_role: &str,
    source_roles: [&str; N],
    merge: &str,
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("slot_id", slot_id.into()),
        ("winner_role", winner_role.into()),
        (
            "source_role_order",
            GerbilSchemeValue::vector(source_roles.into_iter().map(GerbilSchemeValue::from)),
        ),
        ("merge", merge.into()),
    ])
}

fn forced_slot_fixture(slot_id: i64, hotness: &str) -> GerbilSchemeValue {
    GerbilSchemeValue::record([("slot_id", slot_id.into()), ("hotness", hotness.into())])
}

fn policy_combination_merge_audit_fixture() -> GerbilSchemeValue {
    GerbilSchemeValue::vector([
        slot_merge_audit_fixture(31, "ordered_append", "applied"),
        slot_merge_audit_fixture(32, "union", "applied"),
        slot_merge_audit_fixture(33, "min", "applied"),
        slot_merge_audit_fixture(34, "intersection", "applied"),
        slot_merge_audit_fixture(35, "union", "applied"),
        slot_merge_audit_fixture(36, "conflict_error", "conflict"),
    ])
}

fn slot_merge_audit_fixture(slot_id: i64, merge: &str, status: &str) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("slot_id", slot_id.into()),
        ("merge", merge.into()),
        ("status", status.into()),
    ])
}

fn policy_combination_slot_merge_receipts_fixture() -> GerbilSchemeValue {
    GerbilSchemeValue::vector([
        slot_merge_receipt_fixture(
            31,
            "route_rules",
            "ordered_append",
            "applied",
            GerbilSchemeValue::vector(["read_memory".into(), "invoke_model".into()]),
            GerbilSchemeValue::vector([
                "read_memory".into(),
                "invoke_model".into(),
                "rewrite_graph".into(),
                "dispatch_tools".into(),
                "verify".into(),
                "stop".into(),
            ]),
            GerbilSchemeValue::vector([]),
        ),
        slot_merge_receipt_fixture(
            32,
            "observability",
            "union",
            "applied",
            GerbilSchemeValue::vector(["runtime.memory".into(), "runtime.model".into()]),
            GerbilSchemeValue::vector([
                "runtime.memory".into(),
                "runtime.model".into(),
                "runtime.tool".into(),
                "harness.execution".into(),
            ]),
            GerbilSchemeValue::vector([]),
        ),
        slot_merge_receipt_fixture(
            33,
            "budget.max_attempts",
            "min",
            "applied",
            GerbilSchemeValue::vector([5_i64.into(), 3_i64.into()]),
            3_i64.into(),
            GerbilSchemeValue::vector([]),
        ),
        slot_merge_receipt_fixture(
            34,
            "capability",
            "intersection",
            "applied",
            GerbilSchemeValue::vector([
                "+memory".into(),
                "+model".into(),
                "+rewrite".into(),
                "+tool".into(),
                "+verify".into(),
            ]),
            GerbilSchemeValue::vector([
                "+memory".into(),
                "+rewrite".into(),
                "+tool".into(),
                "+verify".into(),
            ]),
            GerbilSchemeValue::vector([]),
        ),
        slot_merge_receipt_fixture(
            35,
            "human_gates",
            "union",
            "applied",
            GerbilSchemeValue::vector(["checker-review".into(), "rewrite-review".into()]),
            GerbilSchemeValue::vector(["checker-review".into(), "rewrite-review".into()]),
            GerbilSchemeValue::vector([]),
        ),
        slot_merge_receipt_fixture(
            36,
            "exclusive_resource",
            "conflict_error",
            "conflict",
            GerbilSchemeValue::vector(["workspace-write".into(), "workspace-write".into()]),
            "workspace-write".into(),
            GerbilSchemeValue::vector(["duplicate-exclusive-resource".into()]),
        ),
    ])
}

fn slot_merge_receipt_fixture(
    slot_id: i64,
    slot: &str,
    merge: &str,
    status: &str,
    inputs: GerbilSchemeValue,
    result: GerbilSchemeValue,
    conflict_reasons: GerbilSchemeValue,
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("kind", GERBIL_POLICY_SLOT_MERGE_RECEIPT_SCHEMA_ID.into()),
        ("slot_id", slot_id.into()),
        ("slot", slot.into()),
        ("merge", merge.into()),
        ("status", status.into()),
        ("inputs", inputs),
        ("result", result),
        ("conflict-reasons", conflict_reasons),
        ("owner", "poo-flow.scheme".into()),
        ("scheme-boundary", "scheme-types-to-rust-types".into()),
        (
            "serialization-boundary",
            "rust-owned-cli-trace-cross-process".into(),
        ),
        ("rust-handler-manufactured", false.into()),
    ])
}

fn policy_combination_loop_program_fixture(policy_digest: [u8; 32]) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema_version", 1_i64.into()),
        (
            "program_id",
            "policy-combination-memory-rewrite-checker".into(),
        ),
        ("policy_epoch", 15_i64.into()),
        (
            "policy_digest",
            GerbilSchemeValue::vector(
                policy_digest
                    .iter()
                    .copied()
                    .map(|byte| GerbilSchemeValue::from(i64::from(byte))),
            ),
        ),
        (
            "mechanism_policies",
            GerbilSchemeValue::vector([
                "memory-policy".into(),
                "maker-policy".into(),
                "dynamic-rewrite-policy".into(),
                "tool-policy".into(),
                "checker-policy".into(),
                "artifact-policy".into(),
                "trace-policy".into(),
            ]),
        ),
        ("initial_state", "start".into()),
        (
            "transitions",
            GerbilSchemeValue::vector([
                loop_program_transition_fixture(
                    "start-memory",
                    "start",
                    "start",
                    "read_memory",
                    "memory-ready",
                ),
                loop_program_transition_fixture(
                    "memory-maker",
                    "memory-ready",
                    "runtime_receipt",
                    "invoke_model",
                    "await-maker",
                ),
                loop_program_transition_fixture(
                    "maker-rewrite",
                    "await-maker",
                    "model_event",
                    "rewrite_graph",
                    "rewritten",
                ),
                loop_program_transition_fixture(
                    "rewrite-tool",
                    "rewritten",
                    "runtime_receipt",
                    "dispatch_tools",
                    "await-tool",
                ),
                loop_program_transition_fixture(
                    "tool-checker",
                    "await-tool",
                    "tool_receipt",
                    "verify",
                    "await-checker",
                ),
                loop_program_transition_fixture(
                    "checker-stop",
                    "await-checker",
                    "verification_receipt",
                    "stop",
                    "stopped",
                ),
            ]),
        ),
    ])
}

fn loop_program_transition_fixture(
    transition_id: &str,
    from: &str,
    event: &str,
    action: &str,
    to: &str,
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("transition_id", transition_id.into()),
        ("from", from.into()),
        ("event", event.into()),
        ("action", action.into()),
        ("to", to.into()),
    ])
}

use marlin_agent_protocol::{
    AuditLoopPolicyPack, BudgetCaps, CompiledLoopEdge, CompiledLoopNode, CompiledRouteBucket,
    CompiledRouteIndex, ContinuationOp, ForcedSlot, HotLoopPolicyPack, LOOP_PROGRAM_SCHEMA_VERSION,
    LoopMechanismPolicyId, LoopPolicyAgentProfileId, LoopPolicyConditionId, LoopPolicyDiagnostic,
    LoopPolicyDiagnosticCode, LoopPolicyDiagnosticSeverity, LoopPolicyDigest, LoopPolicyEpoch,
    LoopPolicyExecutorId, LoopPolicyExplanation, LoopPolicyGateId, LoopPolicyGraphTemplateId,
    LoopPolicyNodeId, LoopPolicyReasonCode, LoopPolicyResourceClassId, LoopPolicyRoleId,
    LoopPolicyRouteBucketId, LoopPolicyRouteTargetId, LoopPolicySlotId, LoopPolicySourceLocationId,
    LoopPolicySourcePath, LoopProgram, LoopProgramActionKind, LoopProgramEventKind, LoopProgramId,
    LoopProgramInput, LoopProgramStateId, LoopProgramTransition, LoopProgramTransitionId,
    RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION, ResolvedLoopPolicyPack, ResourceClass, SlotHotness,
    SlotMergeAlgebra, SlotMergeReceipt, SlotMergeStatus, SlotProvenance, SourceLocation,
};

#[test]
fn resolved_loop_policy_pack_splits_hot_runtime_data_from_audit_data() {
    let pack = sample_policy_pack();

    assert!(pack.has_current_schema());
    assert_eq!(
        pack.schema_version,
        RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION
    );
    assert_eq!(pack.policy_epoch, LoopPolicyEpoch::new(42));
    assert_eq!(pack.policy_digest.as_bytes(), &[7_u8; 32]);
    assert_eq!(pack.hot.capability_mask, 0b0010_1101);
    assert_eq!(pack.hot.human_gate_mask, 0b0001_0110);
    assert_eq!(pack.hot.graph_nodes.len(), 2);
    assert_eq!(pack.hot.graph_edges.len(), 1);
    assert_eq!(pack.hot.continuation_table.len(), 4);
    assert_eq!(pack.audit.linearization[0].as_str(), "incident-freeze");
    assert_eq!(pack.audit.provenance[0].merge, SlotMergeAlgebra::Min);

    let encoded = serde_json::to_value(&pack).expect("policy pack serializes");
    assert_eq!(encoded["schema_version"], 1);
    assert_eq!(encoded["policy_epoch"], 42);
    assert_eq!(encoded["hot"]["graph_nodes"][0]["node_id"], 1);
    assert_eq!(encoded["hot"]["continuation_table"][1]["op"], "retry");
    assert!(encoded["hot"].get("provenance").is_none());
    assert!(encoded["hot"].get("source_locations").is_none());
    assert!(encoded["hot"].get("explanation_strings").is_none());
    assert_eq!(encoded["audit"]["forced_slots"][0]["hotness"], "hot");
    assert_eq!(encoded["audit"]["merge_receipts"][0]["status"], "applied");
}

#[test]
fn continuation_ops_are_defunctionalized_data() {
    let pack = sample_policy_pack();

    assert_eq!(
        pack.hot.continuation_table[0],
        ContinuationOp::StopCompleted
    );
    assert_eq!(
        pack.hot.continuation_table[1],
        ContinuationOp::Retry {
            graph_template: LoopPolicyGraphTemplateId::new(5),
            max_attempts: 2,
        }
    );
    assert_eq!(
        pack.hot.continuation_table[2],
        ContinuationOp::Defer {
            gate_id: LoopPolicyGateId::new(3),
        }
    );
    assert_eq!(
        pack.hot.continuation_table[3],
        ContinuationOp::Escalate {
            reason_code: LoopPolicyReasonCode::new(9),
        }
    );
}

#[test]
fn loop_program_composes_mechanism_policies_without_provider_turn_drivers() {
    let program = sample_loop_program();

    assert!(program.has_current_schema());
    assert_eq!(program.schema_version, LOOP_PROGRAM_SCHEMA_VERSION);
    assert_eq!(program.program_id.as_str(), "repo-build-reactive-turn");
    assert!(program.uses_policy(&LoopMechanismPolicyId::new("reactive-tool-loop-base")));
    assert!(program.uses_policy(&LoopMechanismPolicyId::new(
        "codex-style-pending-input-drain"
    )));
    assert!(program.uses_policy(&LoopMechanismPolicyId::new(
        "openrath-style-resource-key-dispatch"
    )));
    assert!(program.uses_policy(&LoopMechanismPolicyId::new(
        "claude-style-dynamic-graph-rewrite"
    )));
    assert_eq!(program.initial_state.as_str(), "start");
    assert_eq!(program.transitions.len(), 6);

    let encoded = serde_json::to_value(&program).expect("loop program serializes");
    assert_eq!(encoded["schema_version"], 1);
    assert_eq!(encoded["mechanism_policies"][0], "reactive-tool-loop-base");
    assert_eq!(encoded["transitions"][0]["event"], "start");
    assert_eq!(encoded["transitions"][0]["action"], "invoke_model");
    assert!(!encoded.to_string().contains("TurnDriver"));
}

#[test]
fn loop_program_actions_cover_generic_runtime_handoffs() {
    let actions: Vec<LoopProgramActionKind> = sample_loop_program()
        .transitions
        .iter()
        .map(|transition| transition.action.clone())
        .collect();

    assert!(actions.contains(&LoopProgramActionKind::InvokeModel));
    assert!(actions.contains(&LoopProgramActionKind::DispatchTools));
    assert!(actions.contains(&LoopProgramActionKind::Continue));
    assert!(actions.contains(&LoopProgramActionKind::RewriteGraph));
    assert!(actions.contains(&LoopProgramActionKind::Verify));
    assert!(actions.contains(&LoopProgramActionKind::Stop));
}

fn sample_policy_pack() -> ResolvedLoopPolicyPack {
    ResolvedLoopPolicyPack::new(
        42,
        LoopPolicyDigest::from_bytes([7_u8; 32]),
        sample_hot_pack(),
        sample_audit_pack(),
    )
}

fn sample_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("repo-build-reactive-turn"),
        policy_epoch: LoopPolicyEpoch::new(42),
        policy_digest: LoopPolicyDigest::from_bytes([9_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("reactive-tool-loop-base"),
            LoopMechanismPolicyId::new("codex-style-pending-input-drain"),
            LoopMechanismPolicyId::new("openrath-style-resource-key-dispatch"),
            LoopMechanismPolicyId::new("claude-style-dynamic-graph-rewrite"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("start-model"),
                from: LoopProgramStateId::new("start"),
                event: LoopProgramEventKind::Start,
                action: LoopProgramActionKind::InvokeModel,
                to: LoopProgramStateId::new("await-model"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("model-tool-frontier"),
                from: LoopProgramStateId::new("await-model"),
                event: LoopProgramEventKind::ToolRequest,
                action: LoopProgramActionKind::DispatchTools,
                to: LoopProgramStateId::new("await-tools"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("tools-continue"),
                from: LoopProgramStateId::new("await-tools"),
                event: LoopProgramEventKind::ToolReceipt,
                action: LoopProgramActionKind::Continue,
                to: LoopProgramStateId::new("await-model"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("dynamic-rewrite"),
                from: LoopProgramStateId::new("await-model"),
                event: LoopProgramEventKind::ModelEvent,
                action: LoopProgramActionKind::RewriteGraph,
                to: LoopProgramStateId::new("rewritten"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("verify-rewrite"),
                from: LoopProgramStateId::new("rewritten"),
                event: LoopProgramEventKind::RuntimeReceipt,
                action: LoopProgramActionKind::Verify,
                to: LoopProgramStateId::new("verifying"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("verification-stop"),
                from: LoopProgramStateId::new("verifying"),
                event: LoopProgramEventKind::VerificationReceipt,
                action: LoopProgramActionKind::Stop,
                to: LoopProgramStateId::new("stopped"),
            },
        ]
        .into_boxed_slice(),
    })
}

fn sample_hot_pack() -> HotLoopPolicyPack {
    HotLoopPolicyPack {
        capability_mask: 0b0010_1101,
        human_gate_mask: 0b0001_0110,
        budget_caps: BudgetCaps {
            max_attempts: 2,
            max_cost_units: 1_500,
            max_wall_time_ms: 30_000,
        },
        graph_nodes: vec![
            CompiledLoopNode {
                node_id: LoopPolicyNodeId::new(1),
                executor_id: LoopPolicyExecutorId::new(10),
                capability_mask: 0b0000_0001,
                resource_class_id: LoopPolicyResourceClassId::new(1),
            },
            CompiledLoopNode {
                node_id: LoopPolicyNodeId::new(2),
                executor_id: LoopPolicyExecutorId::new(11),
                capability_mask: 0b0000_0100,
                resource_class_id: LoopPolicyResourceClassId::new(2),
            },
        ]
        .into_boxed_slice(),
        graph_edges: vec![CompiledLoopEdge {
            from: LoopPolicyNodeId::new(1),
            to: LoopPolicyNodeId::new(2),
            condition_id: Some(LoopPolicyConditionId::new(20)),
        }]
        .into_boxed_slice(),
        route_index: CompiledRouteIndex {
            buckets: vec![CompiledRouteBucket {
                bucket_id: LoopPolicyRouteBucketId::new(1),
                scope_mask: 0b11,
                target_id: LoopPolicyRouteTargetId::new(8),
            }]
            .into_boxed_slice(),
        },
        resource_classes: vec![
            ResourceClass {
                resource_class_id: LoopPolicyResourceClassId::new(1),
                exclusive: false,
            },
            ResourceClass {
                resource_class_id: LoopPolicyResourceClassId::new(2),
                exclusive: true,
            },
        ]
        .into_boxed_slice(),
        continuation_table: vec![
            ContinuationOp::StopCompleted,
            ContinuationOp::Retry {
                graph_template: LoopPolicyGraphTemplateId::new(5),
                max_attempts: 2,
            },
            ContinuationOp::Defer {
                gate_id: LoopPolicyGateId::new(3),
            },
            ContinuationOp::Escalate {
                reason_code: LoopPolicyReasonCode::new(9),
            },
        ]
        .into_boxed_slice(),
        maker_profiles: vec![LoopPolicyAgentProfileId::new(7)].into_boxed_slice(),
        checker_profiles: vec![LoopPolicyAgentProfileId::new(9)].into_boxed_slice(),
    }
}

fn sample_audit_pack() -> AuditLoopPolicyPack {
    AuditLoopPolicyPack {
        provenance: vec![SlotProvenance {
            slot_id: LoopPolicySlotId::new(1),
            winner_role: LoopPolicyRoleId::new("incident-freeze"),
            source_role_order: vec![
                LoopPolicyRoleId::new("incident-freeze"),
                LoopPolicyRoleId::new("repo-policy"),
                LoopPolicyRoleId::new("org-default"),
            ]
            .into_boxed_slice(),
            merge: SlotMergeAlgebra::Min,
        }]
        .into_boxed_slice(),
        linearization: vec![
            LoopPolicyRoleId::new("incident-freeze"),
            LoopPolicyRoleId::new("repo-policy"),
            LoopPolicyRoleId::new("org-default"),
        ]
        .into_boxed_slice(),
        diagnostics: vec![LoopPolicyDiagnostic {
            code: LoopPolicyDiagnosticCode::new("loop-policy.slot-forced"),
            severity: LoopPolicyDiagnosticSeverity::Info,
            source_location_id: Some(LoopPolicySourceLocationId::new(1)),
        }]
        .into_boxed_slice(),
        source_locations: vec![SourceLocation {
            source_location_id: LoopPolicySourceLocationId::new(1),
            path: LoopPolicySourcePath::new("policies/loops/cases/failure-retry-llm.ss"),
            line: 12,
            column: 3,
        }]
        .into_boxed_slice(),
        explanation_strings: vec![LoopPolicyExplanation::new(
            "network_mode denied by incident-freeze",
        )]
        .into_boxed_slice(),
        forced_slots: vec![ForcedSlot {
            slot_id: LoopPolicySlotId::new(1),
            hotness: SlotHotness::Hot,
        }]
        .into_boxed_slice(),
        merge_receipts: vec![SlotMergeReceipt {
            slot_id: LoopPolicySlotId::new(1),
            merge: SlotMergeAlgebra::Min,
            status: SlotMergeStatus::Applied,
        }]
        .into_boxed_slice(),
    }
}

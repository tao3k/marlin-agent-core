use marlin_agent_protocol::{
    AgentFlowIntent, AgentFlowIntentId, AgentFlowMemoryIntent, AgentFlowMemoryOperation,
    AgentFlowMemoryTarget, AgentFlowPlacementIntent, AgentFlowPlacementOperation,
    AgentFlowPlacementTarget, AgentFlowReceiptStatus, AgentFlowSession, AgentFlowSessionTransform,
    AgentFlowToolIntent, AgentFlowToolName, AgentFlowTransformRejection,
    build_agent_flow_runtime_handoff, derive_agent_flow_session,
};

#[test]
fn agent_flow_transform_projects_runtime_handoff_and_derived_session() {
    let session = AgentFlowSession::root("session-1");
    let transform = AgentFlowSessionTransform::new(
        "transform-1",
        session.session_id.clone(),
        vec![
            tool_intent("intent-tool", "tool.shell"),
            memory_intent(
                "intent-memory",
                "org-memory.project",
                AgentFlowMemoryOperation::Recall,
            ),
            placement_intent(
                "intent-placement",
                "workspace.primary",
                AgentFlowPlacementOperation::BindWorkspace,
            ),
        ],
    );

    let handoff = build_agent_flow_runtime_handoff(&session, transform, "handoff-1", 1_000)
        .expect("handoff should be admitted");

    assert_eq!(handoff.handoff_id.as_str(), "handoff-1");
    assert_eq!(handoff.transform_id.as_str(), "transform-1");
    assert_eq!(handoff.source_session_id.as_str(), "session-1");
    assert_eq!(handoff.admitted_at_ms, 1_000);
    assert_eq!(handoff.intents.len(), 3);

    let receipt = derive_agent_flow_session(&session, handoff, "receipt-1");

    assert_eq!(receipt.receipt_id.as_str(), "receipt-1");
    assert_eq!(receipt.status, AgentFlowReceiptStatus::Derived);
    assert_eq!(
        receipt.derived_session.source_session_id.as_str(),
        "session-1"
    );
    assert_eq!(receipt.derived_session.transform_id.as_str(), "transform-1");
    assert_eq!(receipt.derived_session.receipt_id.as_str(), "receipt-1");
    assert_eq!(
        receipt.derived_session.session.session_id.as_str(),
        "session-1"
    );
    assert_eq!(receipt.derived_session.session.generation, 1);

    let value = serde_json::to_value(&receipt).expect("receipt serializes");
    assert_eq!(value["handoff"]["intents"][0]["intent_family"], "tool");
    assert_eq!(value["status"], "derived");
    assert!(value.get("workflow_run").is_none());
    assert!(value.get("controller").is_none());
}

#[test]
fn agent_flow_handoff_rejects_wrong_source_session() {
    let session = AgentFlowSession::root("session-1");
    let transform = AgentFlowSessionTransform::new(
        "transform-mismatch",
        "other-session",
        vec![tool_intent("intent-tool", "tool.shell")],
    );

    let rejection = build_agent_flow_runtime_handoff(&session, transform, "handoff-1", 1_000)
        .expect_err("mismatched transform source should be rejected");

    assert_eq!(
        rejection,
        AgentFlowTransformRejection::SourceSessionMismatch {
            session_id: "session-1".into(),
            transform_source_session_id: "other-session".into(),
        }
    );
}

#[test]
fn agent_flow_handoff_rejects_empty_intent_set() {
    let session = AgentFlowSession::root("session-1");
    let transform = AgentFlowSessionTransform::new(
        "transform-empty",
        session.session_id.clone(),
        Vec::<AgentFlowIntent>::new(),
    );

    let rejection = build_agent_flow_runtime_handoff(&session, transform, "handoff-1", 1_000)
        .expect_err("empty transform should be rejected");

    assert_eq!(
        rejection,
        AgentFlowTransformRejection::EmptyIntentSet {
            transform_id: "transform-empty".into(),
        }
    );
}

fn tool_intent(intent_id: &str, tool_name: &str) -> AgentFlowIntent {
    AgentFlowIntent::Tool(AgentFlowToolIntent {
        intent_id: AgentFlowIntentId::new(intent_id),
        tool_name: AgentFlowToolName::new(tool_name),
    })
}

fn memory_intent(
    intent_id: &str,
    target: &str,
    operation: AgentFlowMemoryOperation,
) -> AgentFlowIntent {
    AgentFlowIntent::Memory(AgentFlowMemoryIntent {
        intent_id: AgentFlowIntentId::new(intent_id),
        target: AgentFlowMemoryTarget::new(target),
        operation,
    })
}

fn placement_intent(
    intent_id: &str,
    target: &str,
    operation: AgentFlowPlacementOperation,
) -> AgentFlowIntent {
    AgentFlowIntent::Placement(AgentFlowPlacementIntent {
        intent_id: AgentFlowIntentId::new(intent_id),
        target: AgentFlowPlacementTarget::new(target),
        operation,
    })
}

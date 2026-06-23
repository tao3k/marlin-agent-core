use marlin_agent_protocol::{
    AgentFlowIntent, AgentFlowIntentId, AgentFlowMemoryIntent, AgentFlowMemoryOperation,
    AgentFlowMemoryTarget, AgentFlowSession, AgentFlowSessionTransform, AgentFlowToolIntent,
    AgentFlowToolName, AgentFlowTransformRejection,
};
use marlin_agent_runtime::{AgentFlowLoopStepRequest, project_agent_flow_loop_step};

#[test]
fn agent_flow_loop_step_derives_session_receipt_without_controller_state() {
    let session = AgentFlowSession::root("session-1");
    let transform = AgentFlowSessionTransform::new(
        "transform-1",
        session.session_id.clone(),
        vec![
            AgentFlowIntent::Tool(AgentFlowToolIntent {
                intent_id: AgentFlowIntentId::new("intent-tool"),
                tool_name: AgentFlowToolName::new("tool.shell"),
            }),
            AgentFlowIntent::Memory(AgentFlowMemoryIntent {
                intent_id: AgentFlowIntentId::new("intent-memory"),
                target: AgentFlowMemoryTarget::new("org-memory.project"),
                operation: AgentFlowMemoryOperation::Recall,
            }),
        ],
    );

    let receipt = project_agent_flow_loop_step(AgentFlowLoopStepRequest {
        session,
        transform,
        handoff_id: "handoff-1".into(),
        receipt_id: "receipt-1".into(),
        admitted_at_ms: 512,
    })
    .expect("loop step should produce receipt");

    assert_eq!(receipt.handoff.handoff_id.as_str(), "handoff-1");
    assert_eq!(receipt.handoff.admitted_at_ms, 512);
    assert_eq!(receipt.handoff.intents.len(), 2);
    assert_eq!(
        receipt.derived_session.session.session_id.as_str(),
        "session-1"
    );
    assert_eq!(receipt.derived_session.session.generation, 1);
    assert_eq!(receipt.derived_session.receipt_id.as_str(), "receipt-1");

    let value = serde_json::to_value(&receipt).expect("receipt serializes");
    assert!(value.get("run_id").is_none());
    assert!(value.get("graph_id").is_none());
    assert!(value.get("controller").is_none());
}

#[test]
fn agent_flow_loop_step_reuses_protocol_handoff_gate() {
    let session = AgentFlowSession::root("session-1");
    let transform = AgentFlowSessionTransform::new(
        "transform-empty",
        session.session_id.clone(),
        Vec::<AgentFlowIntent>::new(),
    );

    let rejection = project_agent_flow_loop_step(AgentFlowLoopStepRequest {
        session,
        transform,
        handoff_id: "handoff-1".into(),
        receipt_id: "receipt-1".into(),
        admitted_at_ms: 512,
    })
    .expect_err("empty transform should be rejected before receipt derivation");

    assert_eq!(
        rejection,
        AgentFlowTransformRejection::EmptyIntentSet {
            transform_id: "transform-empty".into(),
        }
    );
}

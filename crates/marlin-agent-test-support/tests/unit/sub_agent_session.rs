use marlin_agent_sessions::SessionKind;
use marlin_agent_test_support::{
    assert_sub_agent_memory_session_fixture, sub_agent_memory_allowed_fixture,
    sub_agent_memory_denied_fixture, sub_agent_memory_session_replay_evidence,
    sub_agent_memory_session_visibility_evidence,
};

#[test]
fn sub_agent_memory_allowed_fixture_grants_memory_visibility() {
    let fixture = sub_agent_memory_allowed_fixture();
    let (child_session, isolation_receipt) = fixture.parent_session().child_session(
        SessionKind::SubAgent,
        fixture.config().child_session_id(),
        fixture.requested_visibility(),
    );

    assert_sub_agent_memory_session_fixture(
        &fixture,
        &child_session,
        fixture.config(),
        &isolation_receipt,
    );
    let evidence = sub_agent_memory_session_visibility_evidence(&child_session, &isolation_receipt);
    let detail = evidence.detail.as_deref().expect("visibility detail");
    assert!(evidence.present);
    assert_eq!(evidence.subject, "sub-agent-memory-session:reviewer");
    assert!(detail.contains("root_session_id=session/root"));
    assert!(detail.contains("memory_visible=true"));
    assert!(detail.contains("denied_memory=false"));
    assert!(detail.contains("denied_namespace_count=0"));
    assert!(detail.contains("max_history_items=Some(16)"));
    assert!(detail.contains("history_limit_applied=true"));
}

#[test]
fn sub_agent_memory_denied_fixture_records_denied_memory_visibility() {
    let fixture = sub_agent_memory_denied_fixture();
    let (child_session, isolation_receipt) = fixture.parent_session().child_session(
        SessionKind::SubAgent,
        fixture.config().child_session_id(),
        fixture.requested_visibility(),
    );

    assert_sub_agent_memory_session_fixture(
        &fixture,
        &child_session,
        fixture.config(),
        &isolation_receipt,
    );
    let evidence = sub_agent_memory_session_visibility_evidence(&child_session, &isolation_receipt);
    let detail = evidence.detail.as_deref().expect("visibility detail");
    assert!(evidence.present);
    assert_eq!(evidence.subject, "sub-agent-memory-session:auditor");
    assert!(detail.contains("root_session_id=session/root"));
    assert!(detail.contains("memory_visible=false"));
    assert!(detail.contains("denied_memory=true"));
    assert!(detail.contains("denied_namespace_count=1"));
    assert!(detail.contains("max_history_items=Some(32)"));
    assert!(detail.contains("history_limit_applied=false"));
}

#[test]
fn sub_agent_memory_denied_fixture_projects_replay_contraction_evidence() {
    let fixture = sub_agent_memory_denied_fixture();
    let (child_session, isolation_receipt) = fixture.parent_session().child_session(
        SessionKind::SubAgent,
        fixture.config().child_session_id(),
        fixture.requested_visibility(),
    );

    let evidence = sub_agent_memory_session_replay_evidence(&child_session, &isolation_receipt);
    let detail = evidence.detail.as_deref().expect("replay detail");

    assert!(evidence.present);
    assert_eq!(evidence.subject, "sub-agent-session-replay:auditor");
    assert!(detail.contains("requested_namespaces=[System,User,Workspace,Memory]"));
    assert!(detail.contains("granted_namespaces=[System,User,Workspace]"));
    assert!(detail.contains("denied_namespaces=[Memory]"));
    assert!(detail.contains("requested_history_limit=Some(32)"));
    assert!(detail.contains("granted_history_limit=Some(32)"));
    assert!(detail.contains("history_limit_applied=false"));
    assert!(detail.contains("visibility_contracted=true"));
    assert!(detail.contains("live_llm=false"));
}

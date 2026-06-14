use marlin_agent_protocol::{
    HookAgentScope, SubAgentActivity, SubAgentActivityKind, SubAgentSearchReceipt, SubAgentSource,
    SubAgentSpawnProfile,
};

#[test]
fn sub_agent_source_and_activity_keep_thread_spawn_context() {
    let source = SubAgentSource::ThreadSpawn {
        parent_run_id: Some("run-1".to_owned().into()),
        depth: 2,
        agent_path: Some("agents/reviewer.org".to_owned()),
        agent_nickname: Some("reviewer".to_owned()),
        agent_role: Some("code-review".to_owned()),
    };
    let activity = SubAgentActivity::new("reviewer", source.clone(), SubAgentActivityKind::Started)
        .with_spawn_profile(
            SubAgentSpawnProfile::new("reviewer", "reviewer", "code-review")
                .with_nickname("reviewer"),
        )
        .with_status_message("spawned");

    assert_eq!(activity.source, source);
    assert!(matches!(
        &activity.source,
        SubAgentSource::ThreadSpawn {
            agent_path: Some(path),
            ..
        } if path.ends_with(".org")
    ));
    assert_eq!(activity.agent_reference, "reviewer");
    assert_eq!(activity.kind, SubAgentActivityKind::Started);
    assert_eq!(activity.status_message.as_deref(), Some("spawned"));
    assert_eq!(
        activity
            .spawn_profile
            .as_ref()
            .map(|profile| profile.agent_type.as_str()),
        Some("reviewer")
    );
    assert_eq!(
        activity
            .spawn_profile
            .as_ref()
            .map(|profile| &profile.hook_agent_scope),
        Some(&HookAgentScope::SubAgent)
    );
}

#[test]
fn sub_agent_activity_round_trips_search_receipt() {
    let receipt = SubAgentSearchReceipt::new("A2-owner-items", "owner-items")
        .with_evidence([
            "owner:path(crates/marlin-agent-sessions/src/id.rs)!owner",
            "test:path(crates/marlin-agent-sessions/src/id.rs)!tests",
        ])
        .with_missing("owner item match/code selector/concrete test evidence")
        .with_next("follow T.tests or broaden owner query terms around session id API")
        .with_risk("query terms may miss renamed command/receipt/surface concepts");
    let activity = SubAgentActivity::new(
        "019eba14-6f17-7e82-a960-34fdb7852507",
        SubAgentSource::ThreadSpawn {
            parent_run_id: Some("run-2".to_owned().into()),
            depth: 1,
            agent_path: Some("019eba14-6f17-7e82-a960-34fdb7852507".to_owned()),
            agent_nickname: Some("Linnaeus".to_owned()),
            agent_role: Some("explorer".to_owned()),
        },
        SubAgentActivityKind::Stopped,
    )
    .with_search_receipt(receipt.clone());

    let value = serde_json::to_value(&activity).expect("activity serializes");
    assert_eq!(value["search_receipt"]["role"], "A2-owner-items");
    assert_eq!(value["search_receipt"]["action"], "owner-items");

    let decoded: SubAgentActivity = serde_json::from_value(value).expect("activity deserializes");
    assert_eq!(decoded.search_receipt, Some(receipt));
}

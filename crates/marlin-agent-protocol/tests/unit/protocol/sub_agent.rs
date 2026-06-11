use marlin_agent_protocol::{SubAgentActivity, SubAgentActivityKind, SubAgentSource};

#[test]
fn sub_agent_source_and_activity_keep_thread_spawn_context() {
    let source = SubAgentSource::ThreadSpawn {
        parent_run_id: Some("run-1".to_owned().into()),
        depth: 2,
        agent_path: Some("agents/reviewer.md".to_owned()),
        agent_nickname: Some("reviewer".to_owned()),
        agent_role: Some("code-review".to_owned()),
    };
    let activity = SubAgentActivity::new("reviewer", source.clone(), SubAgentActivityKind::Started)
        .with_status_message("spawned");

    assert_eq!(activity.source, source);
    assert_eq!(activity.agent_reference, "reviewer");
    assert_eq!(activity.kind, SubAgentActivityKind::Started);
    assert_eq!(activity.status_message.as_deref(), Some("spawned"));
}

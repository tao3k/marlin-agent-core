use marlin_agent_protocol::{
    SubAgentActivity, SubAgentActivityKind, SubAgentContextNamespace, SubAgentContextPolicy,
    SubAgentPerformanceBudget, SubAgentPermissionSet, SubAgentSearchReceipt, SubAgentSource,
    SubAgentSpawnConfig, SubAgentSpawnPolicy, SubAgentSpawnStrategy,
};

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

#[test]
fn sub_agent_spawn_config_defaults_to_toml_read_only_policy() {
    let config = SubAgentSpawnConfig::toml("asp-explorer", "asp_explorer", "explorer");

    assert_eq!(config.profile_id, "asp-explorer");
    assert_eq!(config.agent_type.as_str(), "asp_explorer");
    assert_eq!(config.child_session_id(), "asp-explorer");
    assert_eq!(
        config.policy.permissions,
        SubAgentPermissionSet::read_only()
    );
    assert!(!config.policy.permissions.workspace_write);
    assert_eq!(
        config.policy.context.namespaces,
        vec![
            SubAgentContextNamespace::System,
            SubAgentContextNamespace::User,
            SubAgentContextNamespace::Workspace,
            SubAgentContextNamespace::Memory,
        ]
    );
    assert_eq!(config.policy.performance.max_concurrency, Some(1));
}

#[test]
fn sub_agent_spawn_config_keeps_scheme_strategy_optional() {
    let policy = SubAgentSpawnPolicy {
        permissions: SubAgentPermissionSet::worker(),
        context: SubAgentContextPolicy::isolated("worker-session"),
        performance: SubAgentPerformanceBudget::interactive()
            .with_max_concurrency(4)
            .with_timeout_ms(30_000),
    };
    let config = SubAgentSpawnConfig::toml("worker", "worker", "implementation")
        .with_strategy(SubAgentSpawnStrategy::Scheme {
            module: "marlin/spawn-strategy".to_owned(),
            procedure: Some("rank-workers".to_owned()),
            aot: true,
        })
        .with_policy(policy);

    let value = serde_json::to_value(&config).expect("config serializes");
    assert_eq!(
        value["strategy"]["Scheme"]["module"],
        "marlin/spawn-strategy"
    );
    assert_eq!(value["strategy"]["Scheme"]["aot"], true);
    assert_eq!(value["policy"]["permissions"]["workspace_write"], true);
    assert_eq!(value["policy"]["performance"]["max_concurrency"], 4);

    let decoded: SubAgentSpawnConfig = serde_json::from_value(value).expect("config deserializes");
    assert_eq!(decoded.child_session_id(), "worker-session");
}

use marlin_agent_protocol::{
    HookAgentScope, SubAgentActivity, SubAgentActivityKind, SubAgentConfigSurface,
    SubAgentContextPolicy, SubAgentContextVisibility, SubAgentPerformanceBudget,
    SubAgentPermissionSet, SubAgentSearchReceipt, SubAgentSource, SubAgentSpawnConfig,
    SubAgentSpawnConfigError, SubAgentSpawnConfigSet, SubAgentSpawnPolicy, SubAgentSpawnProfile,
    SubAgentSpawnProfileId, SubAgentSpawnStrategy,
};

const SUB_AGENT_SPAWN_PROFILE_TOML: &str = r#"
[[profiles]]
profile_id = "asp-explorer"
agent_type = "asp_explorer"
role = "explorer"
nickname = "ASP Explorer"
hook_agent_scope = "SubAgent"

[profiles.strategy.Scheme]
module = "marlin/sub-agent/search"
procedure = "rank-frontier"
aot = true

[[profiles]]
profile_id = "worker"
agent_type = "worker"
role = "implementation"
hook_agent_scope = "CustomAgent"

[profiles.policy.permissions]
read_only = false
workspace_write = true
network_access = false
process_spawn = false
descendant_spawn = false
tool_access = true
hook_access = true
secret_access = false

[profiles.policy.context]
session_id = "worker-session"
visibility = ["System", "Workspace"]
max_history_items = 8

[profiles.policy.performance]
max_concurrency = 4
timeout_ms = 30000
token_budget = 2000
max_depth = 2
"#;

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
        .with_spawn_profile(
            SubAgentSpawnProfile::new("reviewer", "reviewer", "code-review")
                .with_nickname("reviewer"),
        )
        .with_status_message("spawned");

    assert_eq!(activity.source, source);
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

#[test]
fn sub_agent_spawn_config_defaults_to_toml_read_only_policy() {
    let config = SubAgentSpawnConfig::toml("asp-explorer", "asp_explorer", "explorer");

    assert_eq!(config.profile_id.as_str(), "asp-explorer");
    assert_eq!(config.agent_type.as_str(), "asp_explorer");
    assert_eq!(config.hook_agent_scope, HookAgentScope::SubAgent);
    assert_eq!(config.child_session_id(), "asp-explorer");
    assert_eq!(
        config.policy.permissions,
        SubAgentPermissionSet::read_only()
    );
    assert!(!config.policy.permissions.workspace_write);
    assert!(config.policy.permissions.tool_access);
    assert!(!config.policy.permissions.hook_access);
    assert!(!config.policy.permissions.secret_access);
    assert_eq!(
        config.policy.context.visibility,
        vec![
            SubAgentContextVisibility::System,
            SubAgentContextVisibility::User,
            SubAgentContextVisibility::Workspace,
            SubAgentContextVisibility::Memory,
        ]
    );
    assert_eq!(config.policy.performance.max_concurrency, Some(1));
}

#[test]
fn sub_agent_context_policy_rejects_removed_namespaces_key() {
    let error = serde_json::from_value::<SubAgentContextPolicy>(serde_json::json!({
        "session_id": "legacy-session",
        "namespaces": ["System", "Workspace"],
        "max_history_items": 4
    }))
    .expect_err("removed namespaces key must not be accepted");

    assert!(error.to_string().contains("namespaces"));

    let decoded: SubAgentContextPolicy = serde_json::from_value(serde_json::json!({
        "session_id": "current-session",
        "visibility": ["System", "Workspace"],
        "max_history_items": 4
    }))
    .expect("visibility key is accepted");

    assert_eq!(
        decoded.visibility,
        vec![
            SubAgentContextVisibility::System,
            SubAgentContextVisibility::Workspace,
        ]
    );
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
        .with_hook_agent_scope(HookAgentScope::CustomAgent)
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
    assert_eq!(value["hook_agent_scope"], "CustomAgent");
    assert_eq!(value["policy"]["permissions"]["workspace_write"], true);
    assert_eq!(value["policy"]["performance"]["max_concurrency"], 4);

    let decoded: SubAgentSpawnConfig = serde_json::from_value(value).expect("config deserializes");
    assert_eq!(decoded.child_session_id(), "worker-session");
    assert_eq!(decoded.hook_agent_scope, HookAgentScope::CustomAgent);
}

#[test]
fn sub_agent_spawn_config_set_compiles_toml_profiles_to_typed_configs() {
    let config_set =
        SubAgentSpawnConfigSet::from_toml_str(SUB_AGENT_SPAWN_PROFILE_TOML).expect("TOML compiles");

    assert_eq!(config_set.profiles().len(), 2);

    let explorer = config_set
        .profile(&SubAgentSpawnProfileId::from("asp-explorer"))
        .expect("explorer profile exists");
    assert_eq!(explorer.agent_type.as_str(), "asp_explorer");
    assert_eq!(explorer.role, "explorer");
    assert_eq!(explorer.nickname.as_deref(), Some("ASP Explorer"));
    assert_eq!(explorer.surface, SubAgentConfigSurface::Toml);
    assert_eq!(
        explorer.policy.permissions,
        SubAgentPermissionSet::read_only()
    );
    assert_eq!(explorer.child_session_id(), "asp-explorer");
    assert_eq!(
        explorer.strategy,
        SubAgentSpawnStrategy::Scheme {
            module: "marlin/sub-agent/search".to_owned(),
            procedure: Some("rank-frontier".to_owned()),
            aot: true,
        }
    );

    let worker = config_set
        .profile(&SubAgentSpawnProfileId::from("worker"))
        .expect("worker profile exists");
    assert_eq!(worker.hook_agent_scope, HookAgentScope::CustomAgent);
    assert_eq!(worker.child_session_id(), "worker-session");
    assert!(worker.policy.permissions.workspace_write);
    assert!(worker.policy.permissions.hook_access);
    assert_eq!(
        worker.policy.context.visibility,
        vec![
            SubAgentContextVisibility::System,
            SubAgentContextVisibility::Workspace,
        ]
    );
    assert_eq!(worker.policy.performance.max_concurrency, Some(4));
    assert_eq!(worker.policy.performance.token_budget, Some(2000));
}

#[test]
fn sub_agent_spawn_profile_toml_rejects_source_surface_field() {
    let error = SubAgentSpawnConfigSet::from_toml_str(
        r#"
[[profiles]]
profile_id = "ambiguous"
agent_type = "worker"
role = "implementation"
surface = "Scheme"
"#,
    )
    .expect_err("TOML profile source must not override compiled source surface");

    assert!(matches!(error, SubAgentSpawnConfigError::Toml(_)));
    assert!(error.to_string().contains("surface"));
}

#[test]
fn sub_agent_spawn_config_set_rejects_duplicate_profile_ids() {
    let error = SubAgentSpawnConfigSet::from_toml_str(
        r#"
[[profiles]]
profile_id = "worker"
agent_type = "worker"
role = "implementation"

[[profiles]]
profile_id = "worker"
agent_type = "reviewer"
role = "review"
"#,
    )
    .expect_err("duplicate profile ids are rejected");

    assert!(matches!(
        error,
        SubAgentSpawnConfigError::DuplicateProfileId { .. }
    ));
    assert!(error.to_string().contains("worker"));
}

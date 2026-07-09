use std::{
    collections::BTreeMap,
    path::{Component, PathBuf},
};

use marlin_agent_protocol::{
    MARLIN_SESSION_ID_ENV_VAR, RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeEnvironment,
    RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentDelta, RuntimeHome, RuntimeHomeSource,
    RuntimeSandboxPolicy, RuntimeSession, RuntimeSessionIdSource, RuntimeStateDirectoryKind,
    RuntimeStateLayout, RuntimeStateObjectKind, RuntimeStateStorageReceipt,
    RuntimeStateStorageStatus,
};

#[test]
fn runtime_environment_records_custom_home_layers_and_sandbox() {
    let home = RuntimeHome::custom("/tmp/marlin-home").with_profile("fast");
    let sandbox = RuntimeSandboxPolicy {
        writable_roots: vec![PathBuf::from("/tmp/work")],
        network_access: true,
        exclude_tmpdir_env_var: true,
        exclude_slash_tmp: false,
    };

    let environment = RuntimeEnvironment::default()
        .with_home(home.clone())
        .with_cwd("/tmp/workspace")
        .with_sandbox(sandbox.clone())
        .with_config_layer(RuntimeConfigLayer::new(
            RuntimeConfigLayerSource::Project {
                dot_marlin_folder: PathBuf::from("/tmp/workspace/.marlin"),
            },
            40,
        ))
        .with_config_layer(RuntimeConfigLayer::new(
            RuntimeConfigLayerSource::SessionFlags,
            100,
        ));

    assert_eq!(environment.home, Some(home));
    assert_eq!(
        environment
            .state_layout
            .as_ref()
            .expect("state layout should be derived from home")
            .path_for(RuntimeStateDirectoryKind::Sessions),
        Some(&PathBuf::from("/tmp/marlin-home/sessions"))
    );
    assert_eq!(environment.cwd, Some(PathBuf::from("/tmp/workspace")));
    assert_eq!(environment.sandbox, sandbox);
    assert_eq!(environment.config_layers.len(), 2);
    assert_eq!(environment.config_layers[1].precedence, 100);
}

#[test]
fn runtime_environment_records_session_id_and_source() {
    let session =
        RuntimeSession::try_new("marlin-session-1", RuntimeSessionIdSource::MarlinSessionEnv)
            .expect("session id");
    let environment = RuntimeEnvironment::default().with_session(session.clone());
    let value = serde_json::to_value(&environment).expect("environment should serialize");

    assert_eq!(environment.session, Some(session));
    assert_eq!(value["session"]["id"], "marlin-session-1");
    assert_eq!(value["session"]["source"], "MarlinSessionEnv");
    assert_eq!(MARLIN_SESSION_ID_ENV_VAR, "MARLIN_SESSION_ID");
}

#[test]
fn runtime_home_builds_default_state_layout_and_storage_receipt() {
    let home = RuntimeHome::default_for_user_home("/home/alice").with_profile("default");
    let layout = RuntimeStateLayout::standard(home.clone());
    let receipt = RuntimeStateStorageReceipt::planned(&layout);
    let value = serde_json::to_value(&receipt).expect("receipt should serialize");

    assert_eq!(home.path, PathBuf::from("/home/alice/.marlin"));
    assert_eq!(
        layout.path_for(RuntimeStateDirectoryKind::Config),
        Some(&PathBuf::from("/home/alice/.marlin/config"))
    );
    assert_eq!(
        layout.path_for(RuntimeStateDirectoryKind::Sessions),
        Some(&PathBuf::from("/home/alice/.marlin/sessions"))
    );
    assert_eq!(
        layout.path_for(RuntimeStateDirectoryKind::GraphCache),
        Some(&PathBuf::from("/home/alice/.marlin/cache/graph"))
    );
    assert_eq!(
        layout.path_for(RuntimeStateDirectoryKind::Receipts),
        Some(&PathBuf::from("/home/alice/.marlin/receipts"))
    );
    assert_eq!(receipt.status, RuntimeStateStorageStatus::Planned);
    assert_eq!(value["home"]["path"], "/home/alice/.marlin");
    assert_eq!(value["status"], "Planned");
    assert_eq!(value["directories"][0]["kind"], "Config");
    assert_eq!(
        value["directories"][0]["path"],
        "/home/alice/.marlin/config"
    );
    assert!(value.get("state_file_body").is_none());
}

#[test]
fn runtime_state_layout_resolves_object_paths_without_path_traversal() {
    let layout = RuntimeStateLayout::standard(RuntimeHome::default_for_user_home("/home/alice"));

    let session = layout
        .session_path("root/session:1")
        .expect("session object path should resolve");
    let memory = layout
        .memory_shard_path("../memory shard:alpha")
        .expect("memory shard object path should resolve");
    let receipt = layout
        .receipt_path("receipt:memory-query-1")
        .expect("receipt object path should resolve");
    let graph_cache = layout
        .graph_cache_path("project alpha/query")
        .expect("graph cache object path should resolve");
    let serialized_graph_cache =
        serde_json::to_value(&graph_cache).expect("object path should serialize");

    assert_eq!(session.kind, RuntimeStateObjectKind::Session);
    assert_eq!(session.file_stem.as_str(), "root-session-1");
    assert_eq!(
        session.path,
        PathBuf::from("/home/alice/.marlin/sessions/root-session-1.json")
    );
    assert_eq!(memory.file_stem.as_str(), "memory-shard-alpha");
    assert_eq!(
        memory.path,
        PathBuf::from("/home/alice/.marlin/memory/memory-shard-alpha.json")
    );
    assert_eq!(
        receipt.path,
        PathBuf::from("/home/alice/.marlin/receipts/receipt-memory-query-1.json")
    );
    assert_eq!(
        graph_cache.path,
        PathBuf::from("/home/alice/.marlin/cache/graph/project-alpha-query.json")
    );
    assert!(
        !memory
            .path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    );
    assert_eq!(serialized_graph_cache["kind"], "GraphCache");
    assert_eq!(
        serialized_graph_cache["path"],
        "/home/alice/.marlin/cache/graph/project-alpha-query.json"
    );
    assert!(serialized_graph_cache.get("state_file_body").is_none());
}

#[test]
fn runtime_home_can_record_sub_agent_inheritance() {
    let home = RuntimeHome {
        path: PathBuf::from("/tmp/marlin-home/sub/reviewer"),
        source: RuntimeHomeSource::InheritedSubAgent {
            parent_home: PathBuf::from("/tmp/marlin-home"),
        },
        profile: Some("review".to_owned()),
    };

    assert!(matches!(
        home.source,
        RuntimeHomeSource::InheritedSubAgent { .. }
    ));
    assert_eq!(home.profile.as_deref(), Some("review"));
}

#[test]
fn runtime_environment_defaults_missing_activation_when_deserializing() {
    let environment: RuntimeEnvironment = serde_json::from_str(
        r#"{
          "home": null,
          "cwd": "/repo",
          "sandbox": {
            "writable_roots": [],
            "network_access": false,
            "exclude_tmpdir_env_var": false,
            "exclude_slash_tmp": false
          },
          "config_layers": []
        }"#,
    )
    .expect("legacy environment without activation should deserialize");

    assert_eq!(environment.cwd, Some(PathBuf::from("/repo")));
    assert_eq!(environment.state_layout, None);
    assert_eq!(
        environment.activation,
        RuntimeEnvironmentActivationPolicy::disabled()
    );
    assert!(environment.workspace_projects.is_empty());
    assert_eq!(environment.active_workspace_project, None);
}

#[test]
fn runtime_environment_delta_records_names_without_values() {
    let before = BTreeMap::from([
        ("KEEP".to_owned(), "same".to_owned()),
        ("PATH".to_owned(), "/bin".to_owned()),
        ("SECRET_TOKEN".to_owned(), "old-secret".to_owned()),
    ]);
    let after = BTreeMap::from([
        ("KEEP".to_owned(), "same".to_owned()),
        ("NEW_VAR".to_owned(), "new-value".to_owned()),
        ("PATH".to_owned(), "/usr/bin".to_owned()),
    ]);

    let delta = RuntimeEnvironmentDelta::from_snapshots(&before, &after);
    let serialized = serde_json::to_string(&delta).expect("delta should serialize");

    assert_eq!(delta.added, vec!["NEW_VAR"]);
    assert_eq!(delta.changed, vec!["PATH"]);
    assert_eq!(delta.removed, vec!["SECRET_TOKEN"]);
    assert!(!serialized.contains("old-secret"));
    assert!(!serialized.contains("new-value"));
}

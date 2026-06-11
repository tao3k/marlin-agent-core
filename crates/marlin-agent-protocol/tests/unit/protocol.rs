use std::path::PathBuf;

use marlin_agent_protocol::{
    AgentEvent, AgentEventTopic, AgentScenario, AgentScenarioStep, AgentSpanName,
    AgentTraceSpanRecord, HookEventName, HookHandlerType, HookOutputEntry, HookOutputEntryKind,
    HookRunStatus, HookRunSummary, LoopEvidence, LoopEvidenceKind, RuntimeConfigLayer,
    RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeHome, RuntimeHomeSource,
    RuntimeSandboxPolicy, SubAgentActivity, SubAgentActivityKind, SubAgentSource,
};

#[test]
fn scenario_records_expected_evidence_and_steps() {
    let scenario = AgentScenario::new("content-loop")
        .with_description("validates content-backed loop evidence")
        .with_step(
            AgentScenarioStep::new("load-content")
                .with_input("path", "LOOP.org")
                .expecting_event_topic("kernel.execution")
                .expecting_span_name("harness.execution"),
        )
        .expecting_evidence(LoopEvidenceKind::Content);

    assert_eq!(scenario.id, "content-loop");
    assert_eq!(scenario.steps[0].input["path"], "LOOP.org");
    assert_eq!(
        scenario.steps[0].expected_span_names,
        vec![AgentSpanName::new("harness.execution")]
    );
    assert_eq!(scenario.expected_evidence, vec![LoopEvidenceKind::Content]);
}

#[test]
fn agent_event_topic_round_trips_through_events() {
    let topic = AgentEventTopic::new("kernel.execution");
    let event = AgentEvent::new(topic.clone(), "started");

    assert_eq!(topic.as_str(), "kernel.execution");
    assert_eq!(event.topic, "kernel.execution");
    assert_eq!(event.topic_id(), topic);
    assert_eq!(
        AgentEvent::new("kernel.node", "started").topic_id(),
        AgentEventTopic::new("kernel.node")
    );
}

#[test]
fn agent_trace_span_record_keeps_name_and_fields() {
    let record = AgentTraceSpanRecord::new("agent.provider").with_field("node_id", "plan");

    assert_eq!(record.name, AgentSpanName::new("agent.provider"));
    assert_eq!(
        record.fields.get("node_id").map(String::as_str),
        Some("plan")
    );
}

#[test]
fn evidence_distinguishes_present_and_missing_facts() {
    let present =
        LoopEvidence::present(LoopEvidenceKind::Safety, "safety-doc").with_detail("parser-owned");
    let missing = LoopEvidence::missing(LoopEvidenceKind::Budget, "loop-budget");

    assert!(present.present);
    assert_eq!(present.detail.as_deref(), Some("parser-owned"));
    assert!(!missing.present);
}

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
    assert_eq!(environment.cwd, Some(PathBuf::from("/tmp/workspace")));
    assert_eq!(environment.sandbox, sandbox);
    assert_eq!(environment.config_layers.len(), 2);
    assert_eq!(environment.config_layers[1].precedence, 100);
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
fn hook_run_summary_tracks_status_and_output_entries() {
    let summary = HookRunSummary::running(
        "hook-1",
        HookEventName::SubAgentStart,
        HookHandlerType::Agent,
    )
    .with_entry(HookOutputEntry::new(
        HookOutputEntryKind::Context,
        "spawn reviewer",
    ))
    .completed();

    assert_eq!(summary.id.as_str(), "hook-1");
    assert_eq!(summary.event_name, HookEventName::SubAgentStart);
    assert_eq!(summary.handler_type, HookHandlerType::Agent);
    assert_eq!(summary.status, HookRunStatus::Completed);
    assert_eq!(summary.entries.len(), 1);
    assert_eq!(summary.entries[0].kind, HookOutputEntryKind::Context);
}

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

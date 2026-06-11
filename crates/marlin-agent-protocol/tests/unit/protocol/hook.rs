use marlin_agent_protocol::{
    HookEventName, HookHandlerType, HookOutputEntry, HookOutputEntryKind, HookRunStatus,
    HookRunSummary,
};

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

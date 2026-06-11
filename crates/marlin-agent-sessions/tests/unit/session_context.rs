use marlin_agent_sessions::{
    AgentSessionContext, ContextExpansionPolicy, ContextNamespace, ContextVisibility, SessionId,
    SessionIsolationPolicy, SessionKind,
};

#[test]
fn session_id_rejects_empty_values() {
    assert!(SessionId::try_new("").is_err());
    assert!(SessionId::try_new("   ").is_err());
    assert_eq!(SessionId::new("session/a").as_str(), "session/a");
}

#[test]
fn child_session_visibility_is_narrowed_by_parent_context() {
    let parent = AgentSessionContext::root(
        "root",
        ContextVisibility::from_namespaces([ContextNamespace::Workspace, ContextNamespace::Memory])
            .with_max_history_items(Some(12)),
    );
    let requested = ContextVisibility::from_namespaces([
        ContextNamespace::Workspace,
        ContextNamespace::Secrets,
    ])
    .with_max_history_items(Some(64));

    let (child, receipt) =
        parent.child_session(SessionKind::SubAgent, "sub/reviewer", requested.clone());

    assert_eq!(child.session_id().as_str(), "sub/reviewer");
    assert_eq!(
        child
            .parent_session_id()
            .expect("child should carry parent")
            .as_str(),
        "root"
    );
    assert_eq!(child.root_session_id().as_str(), "root");
    assert_eq!(child.kind(), &SessionKind::SubAgent);
    assert!(child.visibility().contains(&ContextNamespace::Workspace));
    assert!(!child.visibility().contains(&ContextNamespace::Secrets));
    assert_eq!(child.visibility().max_history_items(), Some(12));

    assert_eq!(receipt.requested_visibility(), &requested);
    assert_eq!(receipt.denied_namespaces(), &[ContextNamespace::Secrets]);
    assert!(receipt.history_limit_applied());
}

#[test]
fn strict_child_session_cannot_expand_unlimited_parent_history() {
    let parent = AgentSessionContext::root(
        "root",
        ContextVisibility::from_namespaces([ContextNamespace::Workspace]),
    );
    let requested = ContextVisibility::from_namespaces([ContextNamespace::Workspace])
        .with_max_history_items(Some(8));

    let (child, receipt) = parent.child_session(SessionKind::Tool, "tool/run", requested);

    assert_eq!(child.visibility().max_history_items(), Some(8));
    assert!(!receipt.history_limit_applied());
}

#[test]
fn explicit_expansion_policy_can_grant_requested_context() {
    let parent = AgentSessionContext::root_with_policy(
        "root",
        ContextVisibility::from_namespaces([ContextNamespace::Workspace]),
        SessionIsolationPolicy::allow_context_expansion(),
    );
    let requested = ContextVisibility::from_namespaces([
        ContextNamespace::Workspace,
        ContextNamespace::Secrets,
    ])
    .with_max_history_items(Some(128));

    let (child, receipt) = parent.child_session(SessionKind::Hook, "hook/pre-tool", requested);

    assert_eq!(
        parent.isolation().context_expansion(),
        &ContextExpansionPolicy::Allow
    );
    assert!(child.visibility().contains(&ContextNamespace::Secrets));
    assert!(receipt.denied_namespaces().is_empty());
    assert!(!receipt.history_limit_applied());
}

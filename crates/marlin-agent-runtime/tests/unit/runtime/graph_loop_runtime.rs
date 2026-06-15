use marlin_agent_runtime::{
    ContextVisibility, GraphLoopRunRegistryHandle, RuntimeEnvironment, SessionKind,
    TokioAgentRuntime,
};

fn inspect_run(
    graph_loop_runs: GraphLoopRunRegistryHandle,
    run_id: &str,
    now_ms: u64,
) -> (String, String) {
    graph_loop_runs.read_registry(|registry| {
        let snapshot = registry.snapshot(now_ms);
        let observation = snapshot
            .runs
            .iter()
            .find(|observation| observation.run_id.as_str() == run_id)
            .expect("graph loop run should be visible");
        (
            observation.run_id.as_str().to_owned(),
            observation.graph_id.as_str().to_owned(),
        )
    })
}

#[test]
fn runtime_context_shares_graph_loop_registry_with_children() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let graph_loop_runs = runtime.graph_loop_runs();
    graph_loop_runs.with_registry(|registry| {
        registry
            .start_run("run-1", "graph-1", 0)
            .expect("start graph loop run");
    });

    let child_context = runtime.context().child_context();
    let child_runs = child_context.graph_loop_runs();
    let (child_run_id, child_graph_id) = inspect_run(child_runs, "run-1", 1);

    assert_eq!(child_run_id, "run-1");
    assert_eq!(child_graph_id, "graph-1");

    let environment_child_runs = runtime
        .context()
        .child_context_with_environment(RuntimeEnvironment::default().with_cwd("/tmp/child"))
        .graph_loop_runs();
    assert_eq!(inspect_run(environment_child_runs, "run-1", 2).0, "run-1");

    let (session_child_context, _receipt) = runtime.context().child_context_for_session(
        SessionKind::SubAgent,
        "session-child",
        ContextVisibility::default_runtime(),
    );
    assert_eq!(
        inspect_run(session_child_context.graph_loop_runs(), "run-1", 3).1,
        "graph-1"
    );
}

#[test]
fn runtime_children_share_graph_loop_registry() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    runtime.graph_loop_runs().with_registry(|registry| {
        registry
            .start_run("run-2", "graph-2", 0)
            .expect("start graph loop run");
    });

    assert_eq!(
        inspect_run(runtime.child_runtime().graph_loop_runs(), "run-2", 1).1,
        "graph-2"
    );

    let (capture_child, _capture_events) = runtime.child_runtime_with_event_capture(4);
    assert_eq!(
        inspect_run(capture_child.graph_loop_runs(), "run-2", 2).0,
        "run-2"
    );

    assert_eq!(
        inspect_run(
            runtime
                .child_runtime_with_environment(
                    RuntimeEnvironment::default().with_cwd("/tmp/child")
                )
                .graph_loop_runs(),
            "run-2",
            3,
        )
        .1,
        "graph-2"
    );

    let (session_child, _receipt) = runtime.child_runtime_for_session(
        SessionKind::SubAgent,
        "runtime-session-child",
        ContextVisibility::default_runtime(),
    );
    assert_eq!(
        inspect_run(session_child.graph_loop_runs(), "run-2", 4).0,
        "run-2"
    );
}

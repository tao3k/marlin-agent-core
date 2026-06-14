use std::sync::Arc;

use marlin_agent_runtime::{
    ContextNamespace, RuntimeContext, RuntimeFuture, SubAgentContextPolicy,
    SubAgentContextVisibility, SubAgentPerformanceBudget, SubAgentPermissionSet, SubAgentRuntime,
    SubAgentSpawnConfig, SubAgentSpawnPolicy, TokioAgentRuntime,
};

#[tokio::test]
async fn sub_agent_config_compiles_to_child_session_visibility() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let config = SubAgentSpawnConfig::toml("asp-explorer", "asp_explorer", "explorer")
        .with_nickname("Galileo")
        .with_policy(SubAgentSpawnPolicy {
            permissions: SubAgentPermissionSet::read_only(),
            context: SubAgentContextPolicy {
                session_id: Some("session:asp-explorer".to_owned()),
                visibility: vec![
                    SubAgentContextVisibility::System,
                    SubAgentContextVisibility::Workspace,
                ],
                max_history_items: Some(2),
            },
            performance: SubAgentPerformanceBudget::interactive(),
        });

    let (task, receipt) =
        runtime.spawn_sub_agent_with_config(Arc::new(SessionPolicyEchoSubAgent), (), config);

    assert_eq!(receipt.child_session_id().as_str(), "session:asp-explorer");
    assert_eq!(receipt.profile_id(), "asp-explorer");
    assert_eq!(receipt.agent_type(), "asp_explorer");
    assert_eq!(receipt.role(), "explorer");
    assert_eq!(receipt.nickname(), Some("Galileo"));
    let activity_profile = receipt.activity_profile();
    assert_eq!(activity_profile.profile_id.as_str(), "asp-explorer");
    assert_eq!(activity_profile.agent_type.as_str(), "asp_explorer");
    assert_eq!(activity_profile.role, "explorer");
    assert_eq!(activity_profile.nickname.as_deref(), Some("Galileo"));
    assert_eq!(
        receipt.isolation_receipt().child_session_id().as_str(),
        "session:asp-explorer"
    );
    assert!(
        receipt
            .config()
            .policy
            .context
            .visibility
            .contains(&SubAgentContextVisibility::System)
    );
    let output = task.join().await.expect("sub-agent task should finish");

    assert_eq!(output.session_id, "session:asp-explorer");
    assert!(output.workspace_visible);
    assert!(!output.memory_visible);
    assert_eq!(output.max_history_items, Some(2));
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SessionPolicyOutput {
    session_id: String,
    workspace_visible: bool,
    memory_visible: bool,
    max_history_items: Option<usize>,
}

#[derive(Clone, Debug)]
struct SessionPolicyEchoSubAgent;

impl SubAgentRuntime for SessionPolicyEchoSubAgent {
    type Input = ();
    type Output = SessionPolicyOutput;

    fn run_sub_agent(
        &self,
        _input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let session = context.session().clone();
        Box::pin(async move {
            SessionPolicyOutput {
                session_id: session.session_id().as_str().to_owned(),
                workspace_visible: session.visibility().contains(&ContextNamespace::Workspace),
                memory_visible: session.visibility().contains(&ContextNamespace::Memory),
                max_history_items: session.visibility().max_history_items(),
            }
        })
    }
}

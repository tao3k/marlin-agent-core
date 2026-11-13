// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::sync::Arc;

use marlin_agent_protocol::{ModelToolCall, ModelToolCallId, ModelToolName};
use marlin_agent_runtime::{
    ModelToolCatalog, ModelToolCatalogError, ModelToolDeclaration, ModelToolGovernance,
    ModelToolOutcome, RuntimeContext, RuntimeFuture, ShellToolDeclarationRequest, ShellToolHandler,
    TokioAgentRuntime, ToolRuntime,
};

struct Handler;

impl ToolRuntime for Handler {
    type Invocation = ModelToolCall;
    type Output = ModelToolOutcome;

    fn run_tool(
        &self,
        _call: ModelToolCall,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async { ModelToolOutcome::Completed("ok".into()) })
    }
}

fn declaration(name: &str) -> ModelToolDeclaration {
    ModelToolDeclaration::shell(ShellToolDeclarationRequest {
        name: ModelToolName::new(name).unwrap(),
        description: "Read a bounded workspace fact".into(),
        argument: "command".into(),
        timeout_seconds: 5,
        output_limit: 1024,
    })
    .unwrap()
}

fn call(id: &str, name: &str) -> ModelToolCall {
    ModelToolCall {
        id: ModelToolCallId::new(id).unwrap(),
        name: ModelToolName::new(name).unwrap(),
        arguments: serde_json::json!({"command":"printf ok"})
            .as_object()
            .unwrap()
            .clone(),
    }
}

#[test]
fn only_registered_handlers_are_offered_and_admitted() {
    let mut catalog = ModelToolCatalog::new();
    let empty = catalog.offer();
    assert!(empty.definitions().is_empty());
    assert!(empty.completion_options().tools.is_none());
    assert_eq!(
        empty.admit_calls(&[call("one", "read")]),
        Err(ModelToolCatalogError::UnregisteredTool)
    );
    catalog
        .register(declaration("read"), Arc::new(Handler))
        .unwrap();
    let offered = catalog.offer();
    assert_eq!(
        offered.definitions(),
        vec![declaration("read").definition()]
    );
    assert_eq!(
        offered.completion_options().tools,
        Some(offered.definitions())
    );
    assert!(offered.matches_definitions(Some(&offered.definitions())));
    assert!(!offered.matches_definitions(None));
    assert_eq!(offered.admit_calls(&[call("one", "read")]), Ok(()));
    assert_eq!(
        offered.admit_calls(&[call("one", "read"), call("two", "write")]),
        Err(ModelToolCatalogError::UnregisteredTool)
    );
    assert_eq!(
        offered.admit_calls(&[call("one", "read"), call("one", "read")]),
        Err(ModelToolCatalogError::InvalidCallSet)
    );
    catalog
        .register(declaration("write"), Arc::new(Handler))
        .unwrap();
    assert_eq!(
        offered.admit_calls(&[call("two", "write")]),
        Err(ModelToolCatalogError::UnregisteredTool),
        "a later registration cannot change the request's offer"
    );
    assert_eq!(
        catalog.register(declaration("read"), Arc::new(Handler)),
        Err(ModelToolCatalogError::DuplicateName)
    );
    assert_eq!(offered.definitions().len(), 1);
    assert_eq!(catalog.offer().definitions().len(), 2);
}

#[tokio::test]
async fn selected_shell_declaration_uses_one_generic_handler_and_ask_gate() {
    let declaration = ModelToolDeclaration::shell(ShellToolDeclarationRequest {
        name: ModelToolName::new("bash").unwrap(),
        description: "Run a bounded command".into(),
        argument: "command".into(),
        timeout_seconds: 5,
        output_limit: 1024,
    })
    .unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let mut catalog = ModelToolCatalog::new();
    catalog
        .register_declaration(declaration.clone(), workspace.path().into())
        .unwrap();
    let second = ModelToolDeclaration::shell(ShellToolDeclarationRequest {
        name: ModelToolName::new("workspace_shell").unwrap(),
        description: "Another Scheme declared shell tool".into(),
        argument: "script".into(),
        timeout_seconds: 5,
        output_limit: 1024,
    })
    .unwrap();
    catalog
        .register_declaration(second, workspace.path().into())
        .unwrap();
    let offer = catalog.offer();
    assert_eq!(
        offer.governance(declaration.name()),
        Some(ModelToolGovernance::Ask)
    );
    assert_eq!(
        offer.definitions()[0].parameters["required"],
        serde_json::json!(["command"])
    );
    assert_eq!(
        offer.definitions()[1].parameters["required"],
        serde_json::json!(["script"])
    );
    let mut valid = call("one", "bash");
    valid
        .arguments
        .insert("command".into(), "printf hello".into());
    assert_eq!(offer.admit_calls(&[valid.clone()]), Ok(()));
    let mut invalid = valid.clone();
    invalid.arguments.insert("extra".into(), true.into());
    assert_eq!(
        offer.admit_calls(&[invalid]),
        Err(ModelToolCatalogError::InvalidCallSet)
    );
    let (owner, _) = TokioAgentRuntime::new(4);
    let outcome = ShellToolHandler::new(workspace.path().into(), declaration)
        .run_tool(valid, owner.context())
        .await;
    assert_eq!(
        outcome,
        ModelToolOutcome::Completed("exit code: 0\nhello".into())
    );
}

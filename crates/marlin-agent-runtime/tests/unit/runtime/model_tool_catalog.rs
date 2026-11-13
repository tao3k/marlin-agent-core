// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::sync::Arc;

use marlin_agent_protocol::{ModelToolCall, ModelToolCallId, ModelToolDefinition, ModelToolName};
use marlin_agent_runtime::{
    ModelToolCatalog, ModelToolCatalogError, ModelToolOutcome, RuntimeContext, RuntimeFuture,
    ToolRuntime,
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

fn definition(name: &str) -> ModelToolDefinition {
    ModelToolDefinition {
        name: ModelToolName::new(name).unwrap(),
        description: "Read a bounded workspace fact".into(),
        parameters: serde_json::json!({"type": "object"}),
    }
}

fn call(id: &str, name: &str) -> ModelToolCall {
    ModelToolCall {
        id: ModelToolCallId::new(id).unwrap(),
        name: ModelToolName::new(name).unwrap(),
        arguments: serde_json::Map::new(),
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
    let mut invalid = definition("invalid");
    invalid.parameters = serde_json::Value::Null;
    assert_eq!(
        catalog.register(invalid, Arc::new(Handler)),
        Err(ModelToolCatalogError::InvalidDefinition)
    );
    catalog
        .register(definition("read"), Arc::new(Handler))
        .unwrap();
    let offered = catalog.offer();
    assert_eq!(offered.definitions(), vec![definition("read")]);
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
        .register(definition("write"), Arc::new(Handler))
        .unwrap();
    assert_eq!(
        offered.admit_calls(&[call("two", "write")]),
        Err(ModelToolCatalogError::UnregisteredTool),
        "a later registration cannot change the request's offer"
    );
    assert_eq!(
        catalog.register(definition("read"), Arc::new(Handler)),
        Err(ModelToolCatalogError::DuplicateName)
    );
    assert_eq!(offered.definitions().len(), 1);
    assert_eq!(catalog.offer().definitions().len(), 2);
}

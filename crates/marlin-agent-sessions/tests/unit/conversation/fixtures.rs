// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::{
    ModelEndpoint, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse, ModelToolCall,
    ModelToolCallId, ModelToolName, assistant_gateway_message,
};
use marlin_agent_sessions::{
    ConversationTerminalObservation, ConversationTurnOutcome, ConversationTurnStart,
};
use marlin_agent_storage::{AgentId, ProjectId, SessionId};

pub(super) fn ids() -> (ProjectId, SessionId, AgentId) {
    (
        ProjectId::new("project:fixture").unwrap(),
        SessionId::new("session:fixture").unwrap(),
        AgentId::new("agent:fixture").unwrap(),
    )
}

pub(super) fn start(text: &str) -> ConversationTurnStart {
    ConversationTurnStart {
        endpoint: ModelEndpoint::new("openai", "gpt-fixture"),
        user_text: text.into(),
    }
}

pub(super) fn completion(content: &str) -> ConversationTerminalObservation {
    ConversationTerminalObservation::Completed {
        response: ModelGatewayCompletionResponse::new(
            "response-fixture",
            "gpt-fixture",
            vec![ModelGatewayCompletionChoice::new(
                0,
                assistant_gateway_message(content),
                Some("stop".into()),
            )],
        ),
    }
}

pub(super) fn recorded_completion(content: &str) -> ConversationTurnOutcome {
    let ConversationTerminalObservation::Completed { response } = completion(content) else {
        unreachable!("fixture always creates a completion")
    };
    ConversationTurnOutcome::Completed { response }
}

pub(super) fn tool_request() -> ModelGatewayCompletionResponse {
    let mut message = assistant_gateway_message("I need the requested marker.");
    message.tool_calls.push(ModelToolCall {
        id: ModelToolCallId::new("call_1").unwrap(),
        name: ModelToolName::new("read_marker").unwrap(),
        arguments: serde_json::json!({"path":"src/lib.rs"})
            .as_object()
            .unwrap()
            .clone(),
    });
    ModelGatewayCompletionResponse::new(
        "response-tool",
        "gpt-fixture",
        vec![ModelGatewayCompletionChoice::new(
            0,
            message,
            Some("tool_calls".into()),
        )],
    )
}

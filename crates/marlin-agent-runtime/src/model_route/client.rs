//! Runtime-facing model gateway protocol re-exports.

pub use marlin_agent_protocol::{
    ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionOptions,
    ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    assistant_gateway_message, system_gateway_message, user_gateway_message,
};

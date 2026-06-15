//! Runtime model routing and provider-neutral gateway contracts.

mod client;
mod config;
mod resolver;
mod selection;
mod session;

pub use client::{
    ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionOptions,
    ModelGatewayCompletionResponse, ModelGatewayError, ModelGatewayFuture, ModelGatewayMessage,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelGatewayResult, ModelGatewayTransport,
    RuntimeEdgeModelGateway, assistant_gateway_message, system_gateway_message,
    user_gateway_message,
};
pub use config::{ModelRouteConfig, ModelRouteConfigError};
pub use resolver::{CompiledModelRouteResolver, ModelRouteCompileError};
pub use selection::{
    ModelRouteSelectionProjectionError, ModelRouteSelectionProjectionReceipt,
    ModelRouteSelectionProjectionSource, ProjectedModelRouteDecision,
};
pub use session::{
    ActivatedModelRouteProfileSpawnRequest, ModelRouteSessionBinding, RoutedSubAgentSpawn,
};

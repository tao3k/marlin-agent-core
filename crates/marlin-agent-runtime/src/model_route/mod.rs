//! Runtime model routing and LiteLLM adapter.

mod client;
mod config;
mod resolver;
mod session;

pub use client::{
    ChatMessage, CompletionOptions, CompletionResponse, LiteLlmModelClient,
    LiteLlmModelClientError, LiteLlmModelClientResult, assistant_message, system_message,
    user_message,
};
pub use config::{ModelRouteConfig, ModelRouteConfigError};
pub use resolver::{CompiledModelRouteResolver, ModelRouteCompileError};
pub use session::{ModelRouteSessionBinding, RoutedSubAgentSpawn};

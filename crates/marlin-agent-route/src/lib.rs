//! Axum adapters for model route admission.

mod http;

pub use http::{
    ChatModelRouteRequest, MODEL_ROUTE_CHAT_PATH, ModelRouteHttpError, ModelRouteHttpErrorBody,
    ModelRouteHttpState, admit_chat_route, model_route_router, model_route_router_from_config,
    model_route_router_from_toml_path, model_route_router_from_toml_str,
};

//! `Axum` HTTP adapter for typed model-route admission.

use std::{fmt, path::Path, sync::Arc};

use axum::{
    Json, Router,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use marlin_agent_protocol::{
    ModelRouteAdmissionRequest, ModelRouteAdmissionResponse, ModelRouteArtifactRef,
    ModelRouteEvidenceProfile, ModelRoutePrecisionTier, ModelRoutePrivacyTier, ModelRouteRequest,
};
use marlin_agent_runtime::{
    CompiledModelRouteResolver, ModelRouteAdmissionError, ModelRouteConfig, ModelRouteConfigError,
    admit_model_route_with_resolver,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "deck-actor")]
use marlin_deck_runtime::{DeckRouteAdmission, DeckRouteAdmissionRejection};

/// Stable chat model-route admission path.
pub const MODEL_ROUTE_CHAT_PATH: &str = "/api/model-route/chat";

/// Shared state for model-route HTTP adapters.
#[derive(Clone)]
pub struct ModelRouteHttpState {
    admission: RouteAdmission,
}

#[derive(Clone)]
enum RouteAdmission {
    Rust(Arc<CompiledModelRouteResolver>),
    #[cfg(feature = "deck-actor")]
    Deck(DeckRouteAdmission),
}

impl fmt::Debug for ModelRouteHttpState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModelRouteHttpState")
            .finish_non_exhaustive()
    }
}

impl ModelRouteHttpState {
    pub fn new(resolver: CompiledModelRouteResolver) -> Self {
        Self {
            admission: RouteAdmission::Rust(Arc::new(resolver)),
        }
    }

    /// Serve the same chat route through the resident Scheme Deck actors.
    #[cfg(feature = "deck-actor")]
    pub fn with_deck_admission(deck: DeckRouteAdmission) -> Self {
        Self {
            admission: RouteAdmission::Deck(deck),
        }
    }

    pub fn from_config(config: ModelRouteConfig) -> Result<Self, ModelRouteConfigError> {
        config.into_resolver().map(Self::new)
    }

    pub fn from_toml_str(source: &str) -> Result<Self, ModelRouteConfigError> {
        Self::from_config(ModelRouteConfig::from_toml_str(source)?)
    }

    pub fn from_toml_path(path: impl AsRef<Path>) -> Result<Self, ModelRouteConfigError> {
        Self::from_config(ModelRouteConfig::from_toml_path(path)?)
    }

    pub fn resolver(&self) -> &CompiledModelRouteResolver {
        match &self.admission {
            RouteAdmission::Rust(resolver) => resolver,
            #[cfg(feature = "deck-actor")]
            RouteAdmission::Deck(deck) => deck.resolver(),
        }
    }
}

/// Builds the model-route Axum router.
pub fn model_route_router(state: ModelRouteHttpState) -> Router {
    Router::new()
        .route(MODEL_ROUTE_CHAT_PATH, post(admit_chat_route))
        .with_state(state)
}

/// Builds the model-route Axum router from typed model route configuration.
pub fn model_route_router_from_config(
    config: ModelRouteConfig,
) -> Result<Router, ModelRouteConfigError> {
    ModelRouteHttpState::from_config(config).map(model_route_router)
}

/// Builds the model-route Axum router from TOML model route configuration.
pub fn model_route_router_from_toml_str(source: &str) -> Result<Router, ModelRouteConfigError> {
    ModelRouteHttpState::from_toml_str(source).map(model_route_router)
}

/// Builds the model-route Axum router from a TOML model route configuration file.
pub fn model_route_router_from_toml_path(
    path: impl AsRef<Path>,
) -> Result<Router, ModelRouteConfigError> {
    ModelRouteHttpState::from_toml_path(path).map(model_route_router)
}

/// Chat admission request supplied by UI or HTTP API consumers.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ChatModelRouteRequest {
    pub route_request: Option<ModelRouteRequest>,
    pub precision_tier: String,
    pub privacy_tier: String,
    pub latency_budget_ms: u64,
    pub evidence_profile: String,
    pub artifact_refs: Vec<String>,
}

impl Default for ChatModelRouteRequest {
    fn default() -> Self {
        Self {
            route_request: None,
            precision_tier: "high".to_owned(),
            privacy_tier: "private".to_owned(),
            latency_budget_ms: 60_000,
            evidence_profile: "local-knowledge-chat".to_owned(),
            artifact_refs: Vec::new(),
        }
    }
}

impl ChatModelRouteRequest {
    pub fn into_admission_request(self) -> ModelRouteAdmissionRequest {
        let mut request = ModelRouteAdmissionRequest::chat(
            self.route_request
                .unwrap_or_else(default_chat_route_request),
        )
        .with_precision_tier(ModelRoutePrecisionTier::new(self.precision_tier))
        .with_privacy_tier(ModelRoutePrivacyTier::new(self.privacy_tier))
        .with_latency_budget_ms(self.latency_budget_ms)
        .with_evidence_profile(ModelRouteEvidenceProfile::new(self.evidence_profile));

        for artifact_ref in self.artifact_refs {
            request = request.with_artifact_ref(ModelRouteArtifactRef::new(artifact_ref));
        }

        request
    }
}

/// Axum handler that admits a chat model route through the runtime resolver.
pub async fn admit_chat_route(
    State(state): State<ModelRouteHttpState>,
    payload: Result<Json<ChatModelRouteRequest>, JsonRejection>,
) -> Result<Json<ModelRouteAdmissionResponse>, ModelRouteHttpError> {
    let Json(payload) = payload.map_err(ModelRouteHttpError::from)?;
    let request = payload.into_admission_request();
    #[cfg(feature = "deck-actor")]
    if let RouteAdmission::Deck(deck) = &state.admission {
        return deck
            .admit(request, tokio_util::sync::CancellationToken::new())
            .await
            .map(Json)
            .map_err(ModelRouteHttpError::from);
    }

    let response = admit_model_route_with_resolver(state.resolver(), request)?;

    Ok(Json(response))
}

/// HTTP error envelope for model-route admission adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRouteHttpError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ModelRouteHttpError {
    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

impl From<ModelRouteAdmissionError> for ModelRouteHttpError {
    fn from(error: ModelRouteAdmissionError) -> Self {
        match error {
            ModelRouteAdmissionError::NoMatchingRoute { .. } => Self {
                status: StatusCode::NOT_FOUND,
                code: "MODEL_ROUTE_NOT_FOUND",
                message: error.to_string(),
            },
        }
    }
}

impl From<JsonRejection> for ModelRouteHttpError {
    fn from(error: JsonRejection) -> Self {
        Self {
            status: error.status(),
            code: "MODEL_ROUTE_INVALID_REQUEST",
            message: error.body_text(),
        }
    }
}

#[cfg(feature = "deck-actor")]
impl From<DeckRouteAdmissionRejection> for ModelRouteHttpError {
    fn from(error: DeckRouteAdmissionRejection) -> Self {
        use DeckRouteAdmissionRejection as Rejection;
        let (status, code) = match error {
            Rejection::Backpressure => (StatusCode::SERVICE_UNAVAILABLE, "DECK_ROUTE_BACKPRESSURE"),
            Rejection::Cancelled => (StatusCode::REQUEST_TIMEOUT, "DECK_ROUTE_CANCELLED"),
            Rejection::Deadline => (StatusCode::GATEWAY_TIMEOUT, "DECK_ROUTE_DEADLINE"),
            Rejection::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "DECK_ROUTE_UNAVAILABLE"),
            Rejection::NoMatchingRoute => (StatusCode::NOT_FOUND, "MODEL_ROUTE_NOT_FOUND"),
            Rejection::Scheme(_) | Rejection::InvalidProjection | Rejection::Projection(_) => {
                return Self {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    code: "DECK_ROUTE_PROJECTION_FAILED",
                    message: "Deck route projection failed".into(),
                };
            }
        };
        Self {
            status,
            code,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ModelRouteHttpError {
    fn into_response(self) -> Response {
        let status = self.status;
        (
            status,
            Json(ModelRouteHttpErrorBody {
                code: self.code.to_owned(),
                message: self.message,
            }),
        )
            .into_response()
    }
}

/// Serialized `HTTP` error body returned by model-route adapters.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRouteHttpErrorBody {
    /// Stable machine-readable error code.
    pub code: String,
    /// Human-readable diagnostic message from the adapter boundary.
    pub message: String,
}

fn default_chat_route_request() -> ModelRouteRequest {
    ModelRouteRequest::command(["marlin", "chat"]).with_command_kind("chat")
}

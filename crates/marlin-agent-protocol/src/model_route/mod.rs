//! Model routing protocol for provider/model identity and sub-agent session policy.

mod admission;
mod artifact;
mod endpoint;
mod identity;
mod matcher;
mod receipt;
mod request;
mod rule;
mod scope;
mod session;

pub use admission::{
    MODEL_ROUTE_ADMISSION_SCHEMA_ID, ModelRouteAdmissionMode, ModelRouteAdmissionRequest,
    ModelRouteAdmissionResponse, ModelRouteIntent,
};
pub use artifact::ModelRouteArtifactProjection;
pub use endpoint::{ModelEndpoint, ModelEndpointContractError};
pub use identity::{
    LiteLlmModelId, ModelAlias, ModelCommandKind, ModelName, ModelProviderId,
    ModelRouteArtifactRef, ModelRouteEvidenceProfile, ModelRouteModality, ModelRoutePrecisionTier,
    ModelRoutePrivacyTier, ModelRouteRuleId, ModelRouteSessionId, ModelRouteSourceKind,
    ModelRouteTaskKind, ModelSessionPersistenceKey, ModelSessionPoolId,
};
pub use matcher::ModelCommandMatcher;
pub use receipt::{ModelRouteDecision, ModelRouteReceipt};
pub use request::ModelRouteRequest;
pub use rule::ModelRouteRule;
pub use scope::ModelRouteAgentScope;
pub use session::{ModelContextForkMode, ModelSessionLifecycle, ModelSessionPolicy};

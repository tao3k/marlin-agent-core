//! Provider and model identity shared by the gateway and frontends.
mod endpoint;
mod identity;
pub use endpoint::{ModelEndpoint, ModelEndpointContractError};
pub use identity::{LiteLlmModelId, ModelAlias, ModelName, ModelProviderId};

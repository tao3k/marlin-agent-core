//! Semantic identifier newtypes shared across `model_route` contracts.

use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

macro_rules! semantic_string {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

semantic_string!(
    /// Real model provider identifier, for example `openai`, `anthropic`, or `local`.
    ModelProviderId
);

semantic_string!(
    /// Provider-owned model name, for example `gpt-5` or `claude-opus-4-8`.
    ModelName
);

semantic_string!(
    /// Optional local alias used for configuration ergonomics.
    ModelAlias
);

semantic_string!(
    /// Stable identifier for a route rule.
    ModelRouteRuleId
);

semantic_string!(
    /// String passed to LiteLLM as the concrete model selector.
    LiteLlmModelId
);

semantic_string!(
    /// Typed command kind used by routing requests.
    ModelCommandKind
);

semantic_string!(
    /// Model route task class admitted by the runtime routing plane.
    ModelRouteTaskKind
);

semantic_string!(
    /// Input/output modality admitted by the runtime routing plane.
    ModelRouteModality
);

semantic_string!(
    /// Source class admitted by artifact-oriented model routing, for example `attachment`.
    ModelRouteSourceKind
);

semantic_string!(
    /// Precision tier requested by a routed model call.
    ModelRoutePrecisionTier
);

semantic_string!(
    /// Privacy tier requested by a routed model call.
    ModelRoutePrivacyTier
);

semantic_string!(
    /// Evidence profile requested by a routed model call.
    ModelRouteEvidenceProfile
);

semantic_string!(
    /// Artifact reference already admitted by the runtime before model routing.
    ModelRouteArtifactRef
);

semantic_string!(
    /// Persistent session reuse key.
    ModelSessionPersistenceKey
);

semantic_string!(
    /// Session pool identifier.
    ModelSessionPoolId
);

semantic_string!(
    /// Requested routed session id.
    ModelRouteSessionId
);

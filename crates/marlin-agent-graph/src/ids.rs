//! Semantic identifiers used by the `AgentGraph` protocol.

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

macro_rules! semantic_id {
    ($name:ident) => {
        #[doc = concat!("Typed semantic identifier for `", stringify!($name), "`.")]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Builds a non-empty semantic identifier.
            pub fn new(value: impl Into<String>) -> Result<Self, AgentGraphValidationError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(AgentGraphValidationError::empty(stringify!($name)));
                }
                Ok(Self(value))
            }

            /// Returns the identifier as a stable string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = AgentGraphValidationError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

semantic_id!(AgentGraphId);
semantic_id!(AgentNodeId);
semantic_id!(AgentEdgeId);
semantic_id!(AgentRole);
semantic_id!(AgentCapability);
semantic_id!(AgentTopologyPolicyId);
semantic_id!(GraphLoopGraphRef);
semantic_id!(GraphLoopNodeRef);
semantic_id!(OrgMemoryScopeRef);
semantic_id!(GerbilPolicyScopeRef);
semantic_id!(AgentPolicyDecisionRef);
semantic_id!(AgentEvidenceId);
semantic_id!(AgentDelegationReason);
semantic_id!(AgentElectionReason);
semantic_id!(AgentRoutingReason);

/// Validation error for typed `AgentGraph` semantic identifiers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentGraphValidationError {
    type_name: &'static str,
}

impl AgentGraphValidationError {
    fn empty(type_name: &'static str) -> Self {
        Self { type_name }
    }
}

impl fmt::Display for AgentGraphValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} cannot be empty", self.type_name)
    }
}

impl Error for AgentGraphValidationError {}

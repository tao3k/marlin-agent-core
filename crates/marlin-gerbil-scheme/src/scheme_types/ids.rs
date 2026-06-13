//! Stable newtype identifiers for the `Scheme` type bridge.

use serde::{Deserialize, Serialize};

/// Stable identifier for a Scheme-side value type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilSchemeTypeId(String);

impl GerbilSchemeTypeId {
    pub fn new(type_id: impl Into<String>) -> Self {
        Self(type_id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable identifier for a JSON schema emitted by Scheme.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilSchemeSchemaId(String);

impl GerbilSchemeSchemaId {
    pub fn new(schema_id: impl Into<String>) -> Self {
        Self(schema_id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable field name used inside a Scheme-side type descriptor.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilSchemeFieldName(String);

impl GerbilSchemeFieldName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

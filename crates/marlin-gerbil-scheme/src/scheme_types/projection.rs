//! Optional Rust-side contracts for known `Scheme` typed value projections.

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::ids::{GerbilSchemeSchemaId, GerbilSchemeTypeId};

/// Type and optional schema identity expected by a Rust projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilSchemeProjectionContract {
    pub type_id: GerbilSchemeTypeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<GerbilSchemeSchemaId>,
}

impl GerbilSchemeProjectionContract {
    pub fn new(type_id: GerbilSchemeTypeId) -> Self {
        Self {
            type_id,
            schema_id: None,
        }
    }

    pub fn with_schema_id(mut self, schema_id: GerbilSchemeSchemaId) -> Self {
        self.schema_id = Some(schema_id);
        self
    }

    pub fn type_id(&self) -> &GerbilSchemeTypeId {
        &self.type_id
    }

    pub fn schema_id(&self) -> Option<&GerbilSchemeSchemaId> {
        self.schema_id.as_ref()
    }
}

/// Static contract for Rust projections that intentionally bind to a Scheme type.
///
/// Dynamic Scheme capabilities can continue to use `GerbilSchemeTypedValue::decode_value`
/// without implementing this trait. Implement it only when a Rust projection is meant
/// to reject the wrong Scheme type or schema before serde decodes the payload.
pub trait GerbilSchemeTypedProjection: DeserializeOwned {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract;
}

//! JSON kind tags used by `Scheme` type validation errors.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON value kind observed while validating a Scheme typed value payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GerbilSchemeJsonTypeKind {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

impl GerbilSchemeJsonTypeKind {
    pub(super) fn from_value(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(_) => Self::Boolean,
            Value::Number(_) => Self::Number,
            Value::String(_) => Self::String,
            Value::Array(_) => Self::Array,
            Value::Object(_) => Self::Object,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
        }
    }
}

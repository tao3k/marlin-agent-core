// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Validated domain newtypes and protocol enums for ASP HTTP forwarding.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::SearchPlanProjectionError;

/// Canonical ASP language identifier (for example `"rust"`, `"gerbil-scheme"`, `"zig"`).
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct AspLanguageId(String);

impl AspLanguageId {
    /// Construct a validated non-empty language identifier.
    pub fn new(raw: impl Into<String>) -> Result<Self, SearchPlanProjectionError> {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() {
            return Err(SearchPlanProjectionError::Invalid(
                "ASP language identifier must not be empty",
            ));
        }
        Ok(Self(trimmed))
    }

    /// Borrow the underlying canonical identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Canonical ASP provider identifier (for example `"asp-rust"`, `"asp-zig"`).
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct AspProviderId(String);

impl AspProviderId {
    /// Construct a validated non-empty provider identifier.
    pub fn new(raw: impl Into<String>) -> Result<Self, SearchPlanProjectionError> {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() {
            return Err(SearchPlanProjectionError::Invalid(
                "ASP provider identifier must not be empty",
            ));
        }
        Ok(Self(trimmed))
    }

    /// Borrow the underlying provider identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Native compiler AST node kind name (for example `"impl_item"`, `"comptime_fn"`).
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct NativeAstKindName(String);

impl NativeAstKindName {
    /// Construct a validated non-empty native AST kind name.
    pub fn new(raw: impl Into<String>) -> Result<Self, SearchPlanProjectionError> {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() {
            return Err(SearchPlanProjectionError::Invalid(
                "native AST kind name must not be empty",
            ));
        }
        Ok(Self(trimmed))
    }

    /// Borrow the underlying native AST kind name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// General Language Ontology kind name (for example `"function"`, `"struct"`).
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct GeneralOntologyKindName(String);

impl GeneralOntologyKindName {
    /// Construct a validated non-empty general ontology kind name.
    pub fn new(raw: impl Into<String>) -> Result<Self, SearchPlanProjectionError> {
        let trimmed = raw.into().trim().to_owned();
        if trimmed.is_empty() {
            return Err(SearchPlanProjectionError::Invalid(
                "general ontology kind name must not be empty",
            ));
        }
        Ok(Self(trimmed))
    }

    /// Borrow the underlying general ontology kind name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated HTTP path starting with `'/'` (for example `"/health"`, `"/v1/provider-runtime"`).
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct HttpEndpointPath(String);

impl HttpEndpointPath {
    /// Construct a validated non-empty HTTP route path starting with `'/'`.
    pub fn new(raw: impl Into<String>) -> Result<Self, SearchPlanProjectionError> {
        let trimmed = raw.into().trim().to_owned();
        if !trimmed.starts_with('/') {
            return Err(SearchPlanProjectionError::Invalid(
                "HTTP endpoint path must start with '/'",
            ));
        }
        Ok(Self(trimmed))
    }

    /// Borrow the underlying path string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Targeted JSON Schema Draft 2020-12 document forwarded by ASP per language.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TargetedJsonSchemaDocument(Value);

impl TargetedJsonSchemaDocument {
    /// Inspect the schema's top-level `"type"` constraint, if present.
    #[must_use]
    pub fn schema_type(&self) -> Option<&str> {
        self.0.get("type").and_then(Value::as_str)
    }

    /// Extract the allowed native `languageKind` enum entries from this schema.
    #[must_use]
    pub fn language_kind_enum(&self) -> Vec<&str> {
        self.0
            .pointer("/properties/languageKind/enum")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// Extract the allowed General Ontology `kind` enum entries from this schema.
    #[must_use]
    pub fn kind_enum(&self) -> Vec<&str> {
        self.property_enum("kind")
    }

    /// Resolve a schema node, following a local `#/$defs/...` `$ref` pointer and
    /// unwrapping nullable `anyOf` / `oneOf` wrappers (e.g. `schemars` `Option<T>`).
    fn resolve_schema_node<'a>(&'a self, node: &'a Value) -> &'a Value {
        if let Some(ref_str) = node.get("$ref").and_then(Value::as_str)
            && let Some(pointer) = ref_str.strip_prefix('#')
            && let Some(target) = self.0.pointer(pointer)
        {
            return self.resolve_schema_node(target);
        }
        for key in ["anyOf", "oneOf"] {
            if let Some(branches) = node.get(key).and_then(Value::as_array) {
                let non_null: Vec<&Value> = branches
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) != Some("null"))
                    .collect();
                if non_null.len() == 1 {
                    return self.resolve_schema_node(non_null[0]);
                }
            }
        }
        node
    }

    /// Resolve a property sub-schema, following local `#/$defs/...` `$ref` and
    /// nullable `anyOf`/`oneOf` wrappers.
    fn resolved_property_schema(&self, property: &str) -> Option<&Value> {
        let raw = self.0.get("properties")?.get(property)?;
        Some(self.resolve_schema_node(raw))
    }

    /// Return all declared JSON Schema property names under `/properties` in deterministic order.
    #[must_use]
    pub fn property_names(&self) -> Vec<&str> {
        let Some(props) = self.0.get("properties").and_then(Value::as_object) else {
            return Vec::new();
        };
        let mut names: Vec<&str> = props.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// Extract the string `enum` values for `/properties/<property>/enum` (following local `$ref` and `anyOf`).
    #[must_use]
    pub fn property_enum(&self, property: &str) -> Vec<&str> {
        self.resolved_property_schema(property)
            .and_then(|s| s.get("enum"))
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// Extract a string `const` value for `/properties/<property>/const` (following local `$ref` and `anyOf`).
    #[must_use]
    pub fn property_const(&self, property: &str) -> Option<&str> {
        self.resolved_property_schema(property)
            .and_then(|s| s.get("const"))
            .and_then(Value::as_str)
    }

    /// Extract the primary non-null JSON Schema `type` string for `/properties/<property>/type`
    /// (supports both `"type": "boolean"` and `schemars` `"type": ["boolean", "null"]`).
    #[must_use]
    pub fn property_type(&self, property: &str) -> Option<&str> {
        let resolved = self.resolved_property_schema(property)?;
        let ty = resolved.get("type")?;
        if let Some(s) = ty.as_str() {
            return Some(s);
        }
        ty.as_array().and_then(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .find(|t| *t != "null")
        })
    }

    /// Extract the optional `description` for `/properties/<property>/description`.
    #[must_use]
    pub fn property_description(&self, property: &str) -> Option<&str> {
        self.0
            .get("properties")
            .and_then(|p| p.get(property))
            .and_then(|s| s.get("description"))
            .and_then(Value::as_str)
            .or_else(|| {
                self.resolved_property_schema(property)
                    .and_then(|s| s.get("description"))
                    .and_then(Value::as_str)
            })
    }

    /// Validate an entity's `(kind, language_kind)` pair against any enum
    /// constraints declared in this targeted JSON Schema.
    #[must_use]
    pub fn admits_entity_kinds(&self, kind: &str, language_kind: &str) -> bool {
        let allowed_lk = self.language_kind_enum();
        let lk_ok = allowed_lk.is_empty() || allowed_lk.contains(&language_kind);
        let allowed_kind = self.kind_enum();
        let kind_ok = allowed_kind.is_empty() || allowed_kind.contains(&kind);
        lk_ok && kind_ok
    }
}

/// Environment variable naming an optional loopback ASP HTTP forwarding gateway
/// (for example `MARLIN_ASP_FORWARDING_URL=http://127.0.0.1:41234`).
pub const ASP_FORWARDING_URL_ENV: &str = "MARLIN_ASP_FORWARDING_URL";

/// Environment variable naming an optional loopback Bearer control-plane token
/// for `POST /v1/providers/register` and `POST /shutdown`.
pub const ASP_FORWARDING_TOKEN_ENV: &str = "MARLIN_ASP_FORWARDING_TOKEN";

/// Validated loopback HTTP endpoint for an ASP forwarding gateway, optionally
/// carrying its per-session Bearer control-plane token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AspForwardingEndpoint {
    authority: String,
    raw_url: String,
    control_token: Option<String>,
}

impl AspForwardingEndpoint {
    /// Validate and wrap a loopback `http://127.0.0.1:<port>` or `http://localhost:<port>` URL.
    pub fn new(raw: impl Into<String>) -> Result<Self, SearchPlanProjectionError> {
        let trimmed = raw.into().trim().trim_end_matches('/').to_owned();
        let Some(rest) = trimmed.strip_prefix("http://") else {
            return Err(SearchPlanProjectionError::Invalid(
                "ASP forwarding endpoint must use http:// loopback URL",
            ));
        };
        let authority = rest.split('/').next().unwrap_or("");
        let host = authority.split(':').next().unwrap_or("");
        if !matches!(host, "127.0.0.1" | "localhost") || !authority.contains(':') {
            return Err(SearchPlanProjectionError::Invalid(
                "ASP forwarding endpoint requires loopback host:port",
            ));
        }
        Ok(Self {
            authority: authority.to_owned(),
            raw_url: format!("http://{authority}"),
            control_token: None,
        })
    }

    /// Attach a loopback Bearer control-plane token to this endpoint.
    #[must_use]
    pub fn with_control_token(mut self, token: impl Into<String>) -> Self {
        let trimmed = token.into().trim().to_owned();
        if !trimmed.is_empty() {
            self.control_token = Some(trimmed);
        }
        self
    }

    /// Borrow the optional Bearer control-plane token.
    #[must_use]
    pub fn control_token(&self) -> Option<&str> {
        self.control_token.as_deref()
    }

    /// Resolve a validated loopback endpoint from [`ASP_FORWARDING_URL_ENV`]
    /// (and optional [`ASP_FORWARDING_TOKEN_ENV`]), returning `None` when unset
    /// or invalid (fail-closed).
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var(ASP_FORWARDING_URL_ENV).ok()?;
        let endpoint = Self::new(raw).ok()?;
        match std::env::var(ASP_FORWARDING_TOKEN_ENV) {
            Ok(token) => Some(endpoint.with_control_token(token)),
            Err(_) => Some(endpoint),
        }
    }

    /// Borrow the canonical `http://host:port` string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw_url
    }

    /// Borrow the `host:port` authority.
    #[must_use]
    pub fn authority(&self) -> &str {
        &self.authority
    }
}

/// Transport protocol advertised by a provider manifest.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProviderTransportProtocol {
    #[serde(rename = "http-json")]
    HttpJson,
}

/// Status returned by ASP `POST /v1/providers/register`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderRegistrationStatus {
    Registered,
}

/// Registration mode acknowledged by ASP `POST /v1/providers/register`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderRegistrationMode {
    LiveHttp,
    DescriptorOnly,
}

//! Rust binding for the Deck runtime Scheme model-route policy selector.

use crate::policy_pack_projection::{
    GerbilLoopPolicyProjectionModule, decode_gerbil_loop_policy_projection_module,
    gerbil_loop_policy_projection_module_contract,
    gerbil_loop_policy_projection_module_type_manifest, gerbil_resolved_loop_policy_pack_contract,
    gerbil_resolved_loop_policy_pack_type_manifest,
};
use crate::scheme_types::{
    GerbilSchemeFieldName, GerbilSchemeNativeAbiContract, GerbilSchemeNativeAbiId,
    GerbilSchemeNativeAbiReadinessPlan, GerbilSchemeNativeProjectionReceipt,
    GerbilSchemeNativeProjectionRequest, GerbilSchemeNativeSymbol, GerbilSchemePackageId,
    GerbilSchemePackageManifest, GerbilSchemeProjectionContract, GerbilSchemeSchemaId,
    GerbilSchemeTypeDecodeError, GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId,
    GerbilSchemeTypeManifest, GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec,
    GerbilSchemeTypedProjection, GerbilSchemeTypedValue, decode_gerbil_scheme_native_projection,
    validate_gerbil_scheme_native_projection,
};
use marlin_agent_protocol::{
    ModelRouteAgentScope, ModelRouteRequest, RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION,
    ResolvedLoopPolicyPack,
};
use serde::{Deserialize, Serialize};

/// Native ABI id for Deck runtime typed projections.
pub const GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID: &str =
    "marlin.deck-runtime.native-projection";
/// Package id for the Deck runtime typed projection ABI manifest.
pub const GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_PACKAGE_ID: &str =
    "marlin.deck-runtime.native-projection";
/// Native ABI version for Deck runtime typed projections.
pub const GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION: u32 = 1;
/// Native ABI symbol that projects a Gerbil POO policy object into a typed Rust envelope.
pub const GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL: &str =
    "marlin_deck_runtime_project_poo_policy";
/// Native ABI symbol that projects a Gerbil-resolved loop policy pack into Rust IR.
pub const GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL: &str =
    "marlin_deck_runtime_project_resolved_loop_policy_pack";
/// Native ABI symbol that projects a Gerbil config-interface module into Rust IR.
pub const GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL: &str =
    "marlin_deck_runtime_project_loop_policy_projection_module";
/// Type id returned by the Gerbil POO policy projection.
pub const GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID: &str =
    "marlin.deck-runtime.poo-policy-projection";
/// Schema id returned by the Gerbil POO policy projection.
pub const GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID: &str =
    "marlin.deck-runtime.poo-policy-projection.v1";
/// Schema id returned by the Deck runtime native model-route selector.
pub const GERBIL_DECK_RUNTIME_MODEL_ROUTE_SELECTION_SCHEMA_ID: &str =
    "marlin-deck-runtime.model-route-selection.v1";
/// Schema id for the Rust-owned native bridge receipt.
pub const GERBIL_DECK_RUNTIME_NATIVE_TYPED_BRIDGE_RECEIPT_SCHEMA_ID: &str =
    "marlin-deck-runtime.native-typed-bridge.v1";

/// Context policy mode returned by the Scheme Deck runtime selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilDeckRuntimeContextMode(String);

impl GerbilDeckRuntimeContextMode {
    pub fn new(mode: impl Into<String>) -> Self {
        Self(mode.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for GerbilDeckRuntimeContextMode {
    fn default() -> Self {
        Self::new("forked-context")
    }
}

impl From<&str> for GerbilDeckRuntimeContextMode {
    fn from(mode: &str) -> Self {
        Self::new(mode)
    }
}

impl From<String> for GerbilDeckRuntimeContextMode {
    fn from(mode: String) -> Self {
        Self::new(mode)
    }
}

/// Isolation policy mode returned by the Scheme Deck runtime selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilDeckRuntimeIsolationMode(String);

impl GerbilDeckRuntimeIsolationMode {
    pub fn new(mode: impl Into<String>) -> Self {
        Self(mode.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for GerbilDeckRuntimeIsolationMode {
    fn default() -> Self {
        Self::new("workspace-isolated")
    }
}

impl From<&str> for GerbilDeckRuntimeIsolationMode {
    fn from(mode: &str) -> Self {
        Self::new(mode)
    }
}

impl From<String> for GerbilDeckRuntimeIsolationMode {
    fn from(mode: String) -> Self {
        Self::new(mode)
    }
}

/// Selected policy kind returned by the Scheme Deck runtime selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GerbilDeckRuntimeSelectedPolicyKind(String);

impl GerbilDeckRuntimeSelectedPolicyKind {
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for GerbilDeckRuntimeSelectedPolicyKind {
    fn from(kind: &str) -> Self {
        Self::new(kind)
    }
}

impl From<String> for GerbilDeckRuntimeSelectedPolicyKind {
    fn from(kind: String) -> Self {
        Self::new(kind)
    }
}

/// Scheme-side model route policy input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeModelRoutePolicy {
    pub name: String,
    pub provider: String,
    pub model: String,
    pub command_prefixes: Vec<String>,
    pub agent_scopes: Vec<String>,
    pub context_mode: GerbilDeckRuntimeContextMode,
    pub isolation_mode: GerbilDeckRuntimeIsolationMode,
}

impl GerbilDeckRuntimeModelRoutePolicy {
    pub fn new(
        name: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            provider: provider.into(),
            model: model.into(),
            command_prefixes: Vec::new(),
            agent_scopes: Vec::new(),
            context_mode: GerbilDeckRuntimeContextMode::default(),
            isolation_mode: GerbilDeckRuntimeIsolationMode::default(),
        }
    }

    pub fn with_command_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.command_prefixes.push(prefix.into());
        self
    }

    pub fn with_agent_scope(mut self, scope: impl Into<String>) -> Self {
        self.agent_scopes.push(scope.into());
        self
    }

    pub fn with_context_mode(mut self, mode: impl Into<GerbilDeckRuntimeContextMode>) -> Self {
        self.context_mode = mode.into();
        self
    }

    pub fn with_isolation_mode(mut self, mode: impl Into<GerbilDeckRuntimeIsolationMode>) -> Self {
        self.isolation_mode = mode.into();
        self
    }
}

/// Request sent to the Scheme Deck runtime policy selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeModelRoutePolicyRequest {
    pub policies: Vec<GerbilDeckRuntimeModelRoutePolicy>,
    pub command: String,
    pub agent_scope: String,
}

impl GerbilDeckRuntimeModelRoutePolicyRequest {
    pub fn new(command: impl Into<String>, agent_scope: impl Into<String>) -> Self {
        Self {
            policies: Vec::new(),
            command: command.into(),
            agent_scope: agent_scope.into(),
        }
    }

    pub fn from_model_route_request(
        policies: impl IntoIterator<Item = GerbilDeckRuntimeModelRoutePolicy>,
        request: &ModelRouteRequest,
    ) -> Self {
        Self {
            policies: policies.into_iter().collect(),
            command: request.command_line(),
            agent_scope: request
                .agent_scope
                .as_ref()
                .map(model_route_agent_scope_label)
                .or_else(|| request.sub_agent_role.clone())
                .unwrap_or_else(|| "any".to_string()),
        }
    }

    pub fn with_policy(mut self, policy: GerbilDeckRuntimeModelRoutePolicy) -> Self {
        self.policies.push(policy);
        self
    }
}

/// Selected policy fields returned by the Scheme selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeModelRouteSelectedPolicy {
    pub kind: GerbilDeckRuntimeSelectedPolicyKind,
    pub name: String,
    pub provider: String,
    pub model: String,
    pub command_prefixes: Vec<String>,
    pub agent_scopes: Vec<String>,
    pub context_mode: GerbilDeckRuntimeContextMode,
    pub isolation_mode: GerbilDeckRuntimeIsolationMode,
}

/// Internal native bridge boundary used by the Deck runtime selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilDeckRuntimeNativeBridgeBoundary {
    SchemeTypesToRustTypes,
}

/// Serialization boundary owned by Rust for outer traces and CLI surfaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilDeckRuntimeSerializationBoundary {
    RustOwnedCliTraceCrossProcess,
}

/// Native typed bridge status.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GerbilDeckRuntimeNativeBridgeStatus {
    Ready,
}

/// Rust-owned receipt that proves the Scheme call crossed a typed native bridge.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeNativeBridgeReceipt {
    pub schema_id: String,
    pub abi_id: String,
    pub abi_version: u32,
    pub status: GerbilDeckRuntimeNativeBridgeStatus,
    pub bridge_boundary: GerbilDeckRuntimeNativeBridgeBoundary,
    pub serialization_boundary: GerbilDeckRuntimeSerializationBoundary,
}

impl GerbilDeckRuntimeNativeBridgeReceipt {
    pub fn ready(abi_id: impl Into<String>, abi_version: u32) -> Self {
        Self {
            schema_id: GERBIL_DECK_RUNTIME_NATIVE_TYPED_BRIDGE_RECEIPT_SCHEMA_ID.to_owned(),
            abi_id: abi_id.into(),
            abi_version,
            status: GerbilDeckRuntimeNativeBridgeStatus::Ready,
            bridge_boundary: GerbilDeckRuntimeNativeBridgeBoundary::SchemeTypesToRustTypes,
            serialization_boundary:
                GerbilDeckRuntimeSerializationBoundary::RustOwnedCliTraceCrossProcess,
        }
    }
}

/// Receipt returned by the Scheme Deck runtime policy selector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimeModelRouteSelectionReceipt {
    pub schema_id: String,
    pub command: String,
    pub agent_scope: String,
    pub matched: bool,
    pub native_bridge: GerbilDeckRuntimeNativeBridgeReceipt,
    pub policy: Option<GerbilDeckRuntimeModelRouteSelectedPolicy>,
}

impl GerbilDeckRuntimeModelRouteSelectionReceipt {
    pub fn selected_policy(&self) -> Option<&GerbilDeckRuntimeModelRouteSelectedPolicy> {
        self.policy.as_ref()
    }
}

/// Rust projection for a Gerbil-built POO policy object.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilDeckRuntimePooPolicyProjection {
    pub schema_id: String,
    pub policy_id: String,
    pub object_system: String,
    pub package: String,
    pub module: String,
    pub action: String,
}

impl GerbilSchemeTypedProjection for GerbilDeckRuntimePooPolicyProjection {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_deck_runtime_poo_policy_projection_contract()
    }
}

/// Contract expected for the Gerbil POO policy projection.
pub fn gerbil_deck_runtime_poo_policy_projection_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
    ))
}

/// Scheme type manifest for Deck runtime POO policy projections.
pub fn gerbil_deck_runtime_poo_policy_projection_type_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_DECK_RUNTIME_POO_POLICY_PROJECTION_SCHEMA_ID,
            )),
            fields: [
                "schema_id",
                "policy_id",
                "object_system",
                "package",
                "module",
                "action",
            ]
            .into_iter()
            .map(required_projection_string_field)
            .collect(),
        }],
    }
}

/// Scheme type manifest for all Deck runtime native projections.
pub fn gerbil_deck_runtime_native_projection_type_manifest() -> GerbilSchemeTypeManifest {
    let mut manifest = gerbil_deck_runtime_poo_policy_projection_type_manifest();
    manifest
        .types
        .extend(gerbil_resolved_loop_policy_pack_type_manifest().types);
    manifest
        .types
        .extend(gerbil_loop_policy_projection_module_type_manifest().types);
    manifest
}

/// Native ABI projection request for the Gerbil POO policy projection.
pub fn gerbil_deck_runtime_poo_policy_projection_request() -> GerbilSchemeNativeProjectionRequest {
    GerbilSchemeNativeProjectionRequest::new(
        GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID),
        GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION,
        GerbilSchemeNativeSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL),
        gerbil_deck_runtime_poo_policy_projection_contract(),
    )
}

/// Native ABI projection request for a Gerbil-resolved loop policy pack.
pub fn gerbil_deck_runtime_resolved_loop_policy_pack_projection_request()
-> GerbilSchemeNativeProjectionRequest {
    GerbilSchemeNativeProjectionRequest::new(
        GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID),
        GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION,
        GerbilSchemeNativeSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL),
        gerbil_resolved_loop_policy_pack_contract(),
    )
}

/// Native ABI projection request for a Gerbil config-interface loop policy module.
pub fn gerbil_deck_runtime_loop_policy_projection_module_request()
-> GerbilSchemeNativeProjectionRequest {
    GerbilSchemeNativeProjectionRequest::new(
        GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID),
        GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION,
        GerbilSchemeNativeSymbol::new(
            GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
        ),
        gerbil_loop_policy_projection_module_contract(),
    )
}

/// Native ABI contract declared by the Deck runtime projection package.
pub fn gerbil_deck_runtime_native_projection_abi_contract() -> GerbilSchemeNativeAbiContract {
    GerbilSchemeNativeAbiContract::new(
        GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID),
        GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION,
    )
    .with_exported_symbols([
        GerbilSchemeNativeSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL),
        GerbilSchemeNativeSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL),
        GerbilSchemeNativeSymbol::new(
            GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
        ),
    ])
}

/// Readiness plan expected before calling the Deck runtime native projection ABI.
pub fn gerbil_deck_runtime_native_projection_readiness_plan() -> GerbilSchemeNativeAbiReadinessPlan
{
    GerbilSchemeNativeAbiReadinessPlan::new(
        GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_ID),
        GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_ABI_VERSION,
    )
    .with_exported_symbols([
        GerbilSchemeNativeSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_POO_POLICY_SYMBOL),
        GerbilSchemeNativeSymbol::new(GERBIL_DECK_RUNTIME_PROJECT_RESOLVED_LOOP_POLICY_PACK_SYMBOL),
        GerbilSchemeNativeSymbol::new(
            GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
        ),
    ])
}

/// Package manifest for the Deck runtime typed projection ABI.
pub fn gerbil_deck_runtime_native_projection_package_manifest() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new(GERBIL_DECK_RUNTIME_NATIVE_PROJECTION_PACKAGE_ID),
        gerbil_deck_runtime_native_projection_type_manifest(),
    )
    .with_projection_contracts([
        gerbil_deck_runtime_poo_policy_projection_contract(),
        gerbil_resolved_loop_policy_pack_contract(),
        gerbil_loop_policy_projection_module_contract(),
    ])
    .with_native_abi(gerbil_deck_runtime_native_projection_abi_contract())
}

/// Decode a Gerbil POO policy projection returned by the native ABI.
pub fn decode_gerbil_deck_runtime_poo_policy_projection(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<
    (
        GerbilSchemeNativeProjectionReceipt,
        GerbilDeckRuntimePooPolicyProjection,
    ),
    GerbilSchemeTypeDecodeError,
> {
    decode_gerbil_scheme_native_projection(
        registry,
        &gerbil_deck_runtime_native_projection_readiness_plan(),
        &gerbil_deck_runtime_poo_policy_projection_request(),
        typed_value,
    )
}

/// Decode a Gerbil-resolved loop policy pack returned by the native ABI.
pub fn decode_gerbil_deck_runtime_resolved_loop_policy_pack_projection(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<
    (GerbilSchemeNativeProjectionReceipt, ResolvedLoopPolicyPack),
    GerbilSchemeTypeDecodeError,
> {
    let (receipt, projection): (GerbilSchemeNativeProjectionReceipt, ResolvedLoopPolicyPack) =
        decode_gerbil_scheme_native_projection(
            registry,
            &gerbil_deck_runtime_native_projection_readiness_plan(),
            &gerbil_deck_runtime_resolved_loop_policy_pack_projection_request(),
            typed_value,
        )?;

    if projection.has_current_schema() {
        Ok((receipt, projection))
    } else {
        Err(GerbilSchemeTypeDecodeError::RustProjection {
            message: format!(
                "resolved loop policy pack schema version {} does not match {}",
                projection.schema_version, RESOLVED_LOOP_POLICY_PACK_SCHEMA_VERSION
            ),
        })
    }
}

/// Decode a Gerbil config-interface loop policy projection module returned by the native ABI.
pub fn decode_gerbil_deck_runtime_loop_policy_projection_module(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<
    (
        GerbilSchemeNativeProjectionReceipt,
        GerbilLoopPolicyProjectionModule,
    ),
    GerbilSchemeTypeDecodeError,
> {
    let receipt = validate_gerbil_scheme_native_projection(
        &gerbil_deck_runtime_native_projection_readiness_plan(),
        &gerbil_deck_runtime_loop_policy_projection_module_request(),
        typed_value,
    )?;
    let projection = decode_gerbil_loop_policy_projection_module(registry, typed_value)?;
    Ok((receipt, projection))
}

fn required_projection_string_field(name: &str) -> GerbilSchemeTypeFieldSpec {
    GerbilSchemeTypeFieldSpec {
        name: GerbilSchemeFieldName::new(name),
        type_id: GerbilSchemeTypeId::new("string"),
        element_type_id: None,
        required: true,
        description: None,
    }
}

fn model_route_agent_scope_label(scope: &ModelRouteAgentScope) -> String {
    match scope {
        ModelRouteAgentScope::Any => "any",
        ModelRouteAgentScope::RootAgent => "root-agent",
        ModelRouteAgentScope::SubAgent => "sub-agent",
        ModelRouteAgentScope::CustomAgent => "custom-agent",
        ModelRouteAgentScope::CustomerAgent => "customer-agent",
        ModelRouteAgentScope::ForkedAgent => "forked-agent",
        ModelRouteAgentScope::IsolatedAgent => "isolated-agent",
        ModelRouteAgentScope::PersistentAgent => "persistent-agent",
    }
    .to_string()
}

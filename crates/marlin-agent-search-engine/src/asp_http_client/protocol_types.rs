// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed HTTP forwarding catalog, dynamic registration, and hydration request/receipt models.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::domain_types::{
    AspForwardingEndpoint, AspLanguageId, AspProviderId, GeneralOntologyKindName, HttpEndpointPath,
    NativeAstKindName, ProviderRegistrationMode, ProviderRegistrationStatus,
    ProviderTransportProtocol, TargetedJsonSchemaDocument,
};

/// Provider manifest summary forwarded over HTTP by ASP `GET /v1/providers`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardedProviderManifest {
    pub language_id: AspLanguageId,
    pub provider_id: AspProviderId,
    #[serde(default)]
    pub extensions: Vec<String>,
}

/// One native-kind -> general-kind mapping entry forwarded in a DSL pack.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardedKindMapping {
    pub native_language_kind: NativeAstKindName,
    pub general_kind: GeneralOntologyKindName,
    #[serde(default)]
    pub description: String,
}

/// Optional Scheme-aligned DSL pack forwarded alongside a provider manifest.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardedDslPack {
    pub language_id: AspLanguageId,
    pub provider_language_id: AspLanguageId,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub kind_mappings: Vec<ForwardedKindMapping>,
    #[serde(default)]
    pub special_relations: Vec<String>,
}

/// One provider forwarding envelope returned by ASP `GET /v1/providers`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardedProviderEnvelope {
    pub manifest: ForwardedProviderManifest,
    pub targeted_schema: TargetedJsonSchemaDocument,
    #[serde(default)]
    pub dsl_pack: Option<ForwardedDslPack>,
}

/// Complete catalog payload returned by ASP `GET /v1/providers`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardedProviderCatalog {
    pub schema_id: String,
    pub schema_version: String,
    pub generation: u64,
    pub registry_digest: String,
    pub providers: Vec<ForwardedProviderEnvelope>,
}

/// Lexical syntax surface declared by a dynamically registered language provider.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicProviderSyntaxSurface {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_comment: Option<[String; 2]>,
    pub delimiter_pairs: Vec<[String; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_terminator: Option<String>,
    pub string_delimiters: Vec<String>,
}

/// HTTP transport descriptor for a dynamically registered language provider.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicProviderTransport {
    pub transport: ProviderTransportProtocol,
    pub command: Vec<String>,
    pub health_path: HttpEndpointPath,
    pub request_path: HttpEndpointPath,
    pub shutdown_path: HttpEndpointPath,
}

/// Capabilities advertised by a dynamically registered language provider.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicProviderCapabilities {
    pub ast_inspect: bool,
    pub ast_verify: bool,
    pub open_edb: bool,
    pub project_resolution: bool,
}

/// Full provider-core manifest sent to ASP `POST /v1/providers/register`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicProviderRegistrationManifest {
    pub schema_id: String,
    pub schema_version: String,
    pub language_id: AspLanguageId,
    pub provider_id: AspProviderId,
    pub display_name: String,
    pub extensions: Vec<String>,
    pub project_markers: Vec<String>,
    pub syntax_surface: DynamicProviderSyntaxSurface,
    pub transport: DynamicProviderTransport,
    pub capabilities: DynamicProviderCapabilities,
}

/// Language conformance / DSL pack sent to ASP `POST /v1/providers/register`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicProviderDslPackRegistration {
    pub schema_id: String,
    pub schema_version: String,
    pub language_id: AspLanguageId,
    pub provider_language_id: AspLanguageId,
    pub extensions: Vec<String>,
    pub project_markers: Vec<String>,
    pub kind_mappings: Vec<ForwardedKindMapping>,
    pub special_relations: Vec<String>,
    pub probes: Vec<Value>,
}

/// Typed registration request for ASP `POST /v1/providers/register`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicProviderRegistrationRequest {
    pub manifest: DynamicProviderRegistrationManifest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsl_pack: Option<DynamicProviderDslPackRegistration>,
}

/// Typed acknowledgement returned by ASP `POST /v1/providers/register`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicProviderRegistrationAck {
    pub status: ProviderRegistrationStatus,
    pub mode: ProviderRegistrationMode,
    #[serde(default)]
    pub language_id: Option<AspLanguageId>,
    #[serde(default)]
    pub provider_id: Option<AspProviderId>,
    #[serde(default)]
    pub generation: Option<u64>,
    #[serde(default)]
    pub registered_count: Option<usize>,
}

/// Explicit special `:gerbil-ascent` relation spec sent across the zero-JSON
/// C-ABI when dynamically registering a language provider in Marlin Scheme.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicSpecialRelationSpec {
    pub relation_name: String,
    pub kind_pattern: Option<GeneralOntologyKindName>,
    pub language_kind_pattern: Option<NativeAstKindName>,
    pub description: String,
}

/// Typed request for zero-JSON dynamic language descriptor registration in Marlin Scheme.
#[derive(Clone, Copy, Debug)]
pub struct DynamicLanguageDescriptorRequest<'a> {
    pub scheme_dir: &'a str,
    pub language_id: &'a AspLanguageId,
    pub provider_language_id: &'a AspLanguageId,
    pub extensions: &'a [String],
    pub kind_mappings: &'a [ForwardedKindMapping],
    pub special_specs: &'a [DynamicSpecialRelationSpec],
}

/// Typed request for syncing ASP provider descriptors and hydrating Open EDB facts.
#[derive(Clone, Copy, Debug)]
pub struct AspHttpHydrationRequest<'a> {
    pub scheme_dir: &'a str,
    pub endpoint: &'a AspForwardingEndpoint,
    pub cache_path: &'a Path,
    pub sync_new_descriptors: bool,
}

/// Receipt returned after syncing provider descriptors and hydrating workspace
/// Open EDB facts over ASP HTTP.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AspHttpHydrationReceipt {
    pub providers_discovered: usize,
    pub dynamic_descriptors_synced: usize,
    pub restored_from_cache_files: usize,
    pub restored_from_cache_entities: usize,
    pub http_inspected_files: usize,
    pub http_ingested_entities: usize,
    pub asp_inspect_micros: u64,
    pub total_elapsed_micros: u64,
    pub cache_saved: bool,
}

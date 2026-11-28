// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! One native Scheme value projection for any declared tool capability.

use std::{collections::HashSet, fmt::Display, sync::Arc, sync::RwLock};
#[cfg(feature = "grpc")]
use std::path::PathBuf;

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    RootedSchemeObject, SchemeScalar,
};
use marlin_agent_protocol::ModelToolName;
#[cfg(feature = "grpc")]
use marlin_agent_runtime::{ModelToolCatalog, ModelToolCatalogError, ModelToolHandler};
use marlin_agent_runtime::{
    ModelToolCapability, ModelToolDeclaration, ToolDeclarationRequest, ToolParameterName,
    ToolParameterSpec, ToolParameterSpecRequest, ToolParameterType,
};
use marlin_agent_search_engine::WorkspaceSearchIndex;

/// Shared resident workspace index handle reused across TUI Helm and gRPC search.
pub type SharedWorkspaceSearchIndex = Arc<RwLock<WorkspaceSearchIndex>>;

/// Rejection while converting selected Scheme tool values into Rust types.
#[derive(Debug)]
pub enum ToolingProjectionError {
    Native(String),
    Invalid(&'static str),
}

impl std::fmt::Display for ToolingProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native(message) => write!(formatter, "tooling native projection: {message}"),
            Self::Invalid(message) => write!(formatter, "invalid tooling declaration: {message}"),
        }
    }
}

impl std::error::Error for ToolingProjectionError {}

fn native<T>(result: Result<T, impl Display>) -> Result<T, ToolingProjectionError> {
    result.map_err(|error| ToolingProjectionError::Native(error.to_string()))
}

fn symbol(value: RootedSchemeObject<'_>) -> Result<String, ToolingProjectionError> {
    if native(value.as_ref().kind())? != ObjectKind::Symbol {
        return Err(ToolingProjectionError::Invalid("expected a symbol"));
    }
    let mut buffer = IdentifierBuffer::default();
    native(value.as_ref().copy_identifier_names(&mut buffer))?;
    let Some((IdentifierKind::Symbol, name)) = buffer.get(0) else {
        return Err(ToolingProjectionError::Invalid("invalid symbol"));
    };
    if buffer.len() != 1 || name.is_empty() || name.len() > 64 {
        return Err(ToolingProjectionError::Invalid("symbol length"));
    }
    Ok(name.to_owned())
}

fn string(value: RootedSchemeObject<'_>) -> Result<String, ToolingProjectionError> {
    let value = native(value.into_string())?;
    if native(value.len().into_result())? > 4096 {
        return Err(ToolingProjectionError::Invalid("description length"));
    }
    native(value.to_string().into_result())
}

fn positive_integer(value: RootedSchemeObject<'_>) -> Result<u64, ToolingProjectionError> {
    match native(value.as_ref().scalar())? {
        SchemeScalar::Fixnum(value) if value > 0 => Ok(value as u64),
        _ => Err(ToolingProjectionError::Invalid("expected positive integer")),
    }
}

fn boolean(value: RootedSchemeObject<'_>) -> Result<bool, ToolingProjectionError> {
    match native(value.as_ref().scalar())? {
        SchemeScalar::Boolean(flag) => Ok(flag),
        _ => Err(ToolingProjectionError::Invalid("expected boolean")),
    }
}

fn decode_parameter_type(value: RootedSchemeObject<'_>) -> Result<ToolParameterType, ToolingProjectionError> {
    ToolParameterType::parse_str(&symbol(value)?).map_err(ToolingProjectionError::Invalid)
}

fn decode_enum_values(value: RootedSchemeObject<'_>) -> Result<Vec<String>, ToolingProjectionError> {
    if matches!(native(value.as_ref().kind())?, ObjectKind::Boolean) {
        if !boolean(value)? {
            return Ok(Vec::new());
        }
        return Err(ToolingProjectionError::Invalid(
            "enum slot must be #f or string vector",
        ));
    }
    let items = native(value.into_collection(CollectionKind::Vector))?;
    if items.len() > 32 {
        return Err(ToolingProjectionError::Invalid("too many enum values"));
    }
    let mut out = Vec::with_capacity(items.len());
    for index in 0..items.len() {
        out.push(string(native(items.element(index))?)?);
    }
    Ok(out)
}

fn decode_tool_parameters(
    value: RootedSchemeObject<'_>,
) -> Result<Vec<ToolParameterSpec>, ToolingProjectionError> {
    let params = native(value.into_collection(CollectionKind::Vector))?;
    if params.is_empty() || params.len() > 16 {
        return Err(ToolingProjectionError::Invalid("tool parameter count"));
    }
    let mut specs = Vec::with_capacity(params.len());
    for index in 0..params.len() {
        let entry = native(native(params.element(index))?.into_collection(CollectionKind::Vector))?;
        if entry.len() != 5 {
            return Err(ToolingProjectionError::Invalid("tool parameter arity"));
        }
        let name = ToolParameterName::new(symbol(native(entry.element(0))?)?)
            .map_err(ToolingProjectionError::Invalid)?;
        let param_type = decode_parameter_type(native(entry.element(1))?)?;
        let description = string(native(entry.element(2))?)?;
        let required = boolean(native(entry.element(3))?)?;
        let allowed_values = decode_enum_values(native(entry.element(4))?)?;
        let spec = ToolParameterSpec::from_request(ToolParameterSpecRequest {
            name,
            param_type,
            description,
            required,
            allowed_values,
        })
        .map_err(ToolingProjectionError::Invalid)?;
        specs.push(spec);
    }
    Ok(specs)
}

/// Admit a bounded native value projection without granting execution.
pub fn decode_tooling(
    value: RootedSchemeObject<'_>,
) -> Result<Vec<ModelToolDeclaration>, ToolingProjectionError> {
    let values = native(value.into_collection(CollectionKind::Vector))?;
    if values.len() > 32 {
        return Err(ToolingProjectionError::Invalid("too many tools"));
    }
    let mut seen = HashSet::with_capacity(values.len());
    let mut declarations = Vec::with_capacity(values.len());
    for index in 0..values.len() {
        let entry = native(values.element(index))?;
        let declaration = decode_tool_entry(entry, &mut seen)?;
        declarations.push(declaration);
    }
    Ok(declarations)
}

fn decode_tool_entry(
    value: RootedSchemeObject<'_>,
    seen: &mut HashSet<String>,
) -> Result<ModelToolDeclaration, ToolingProjectionError> {
    let entry = native(value.into_collection(CollectionKind::Vector))?;
    if entry.len() != 7 {
        return Err(ToolingProjectionError::Invalid("tool declaration arity"));
    }
    let name = symbol(native(entry.element(0))?)?;
    if !seen.insert(name.clone()) {
        return Err(ToolingProjectionError::Invalid("duplicate tool name"));
    }
    let description = string(native(entry.element(1))?)?;
    let capability = match symbol(native(entry.element(2))?)?.as_str() {
        "shell" => ModelToolCapability::Shell,
        "search" => ModelToolCapability::Search,
        _ => {
            return Err(ToolingProjectionError::Invalid(
                "unsupported tool capability",
            ));
        }
    };
    if symbol(native(entry.element(3))?)? != "ask" {
        return Err(ToolingProjectionError::Invalid("tool must ask"));
    }
    let parameters = decode_tool_parameters(native(entry.element(4))?)?;
    let timeout_seconds = positive_integer(native(entry.element(5))?)?;
    let output_limit = positive_integer(native(entry.element(6))?)?;
    let name = ModelToolName::new(name).map_err(ToolingProjectionError::Invalid)?;
    let output_limit = usize::try_from(output_limit)
        .map_err(|_| ToolingProjectionError::Invalid("output limit overflow"))?;
    ModelToolDeclaration::from_request(ToolDeclarationRequest {
        name,
        description,
        capability,
        parameters,
        timeout_seconds,
        output_limit,
    })
    .map_err(ToolingProjectionError::Invalid)
}

/// Bind every admitted Scheme declaration to host capabilities with search tools
/// backed directly by a thin HTTP/2 `tonic` gRPC [`crate::GrpcSearchToolHandler`].
#[cfg(feature = "grpc")]
pub fn catalog_from_declarations_with_grpc_search(
    declarations: impl IntoIterator<Item = ModelToolDeclaration>,
    workspace: PathBuf,
    client: crate::SearchGatewayClient<tonic::transport::Channel>,
    session_id: Arc<std::sync::RwLock<marlin_agent_search_engine::SearchDslSessionId>>,
) -> Result<ModelToolCatalog, ModelToolCatalogError> {
    let mut catalog = ModelToolCatalog::new();
    for declaration in declarations {
        match declaration.capability() {
            ModelToolCapability::Shell => {
                catalog.register_declaration(declaration, workspace.clone())?;
            }
            ModelToolCapability::Search => {
                let handler: Arc<ModelToolHandler> = Arc::new(crate::GrpcSearchToolHandler::new(
                    declaration.clone(),
                    client.clone(),
                    Arc::clone(&session_id),
                ));
                catalog.register(declaration, handler)?;
            }
        }
    }
    Ok(catalog)
}

/// Project the same selected declaration through the linked AOT programme.
///
/// # Safety
/// `export` must be from `runtime`'s live Marlin programme and return a fresh
/// strong root using the Gerbil Scheme Rust ABI.
pub unsafe fn project_linked(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
) -> Result<Vec<ModelToolDeclaration>, ToolingProjectionError> {
    let procedure = native(unsafe { runtime.bind_object_export(export) })?;
    let input = native(runtime.void_object())?;
    decode_tooling(native(procedure.call(&input).into_result())?)
}

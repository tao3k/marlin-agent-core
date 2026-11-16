// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! One native Scheme value projection for any declared tool capability.

use std::{collections::HashSet, fmt::Display, path::PathBuf};

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    RootedSchemeObject, SchemeScalar,
};
use marlin_agent_protocol::ModelToolName;
use marlin_agent_runtime::{
    ModelToolCatalog, ModelToolCatalogError, ModelToolDeclaration, ShellToolDeclarationRequest,
};

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
    if native(value.len().into_result())? > 256 {
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
        let entry = native(native(values.element(index))?.into_collection(CollectionKind::Vector))?;
        if entry.len() != 7 {
            return Err(ToolingProjectionError::Invalid("tool declaration arity"));
        }
        let name = symbol(native(entry.element(0))?)?;
        if !seen.insert(name.clone()) {
            return Err(ToolingProjectionError::Invalid("duplicate tool name"));
        }
        let description = string(native(entry.element(1))?)?;
        if symbol(native(entry.element(2))?)? != "shell" {
            return Err(ToolingProjectionError::Invalid(
                "unsupported tool capability",
            ));
        }
        if symbol(native(entry.element(3))?)? != "ask" {
            return Err(ToolingProjectionError::Invalid("shell tool must ask"));
        }
        let argument = symbol(native(entry.element(4))?)?;
        let timeout = positive_integer(native(entry.element(5))?)?;
        let output_limit = positive_integer(native(entry.element(6))?)?;
        let name = ModelToolName::new(name).map_err(ToolingProjectionError::Invalid)?;
        let output_limit = usize::try_from(output_limit)
            .map_err(|_| ToolingProjectionError::Invalid("output limit overflow"))?;
        declarations.push(
            ModelToolDeclaration::shell(ShellToolDeclarationRequest {
                name,
                description,
                argument,
                timeout_seconds: timeout,
                output_limit,
            })
            .map_err(ToolingProjectionError::Invalid)?,
        );
    }
    Ok(declarations)
}

/// Bind every admitted Scheme declaration to an existing host capability.
/// Tool names, argument names and provider definitions require no Rust case.
pub fn catalog_from_declarations(
    declarations: impl IntoIterator<Item = ModelToolDeclaration>,
    workspace: PathBuf,
) -> Result<ModelToolCatalog, ModelToolCatalogError> {
    let mut catalog = ModelToolCatalog::new();
    for declaration in declarations {
        catalog.register_declaration(declaration, workspace.clone())?;
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

// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed, owner-local access to the selected Lab-backed Scheme Org graph.

use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierKind, NativeError, ObjectKind, RootedCallError,
    RootedSchemeObject, SchemeScalar,
};

const MAX_SOURCE_BYTES: usize = 1024 * 1024;
const MAX_ELEMENTS: usize = 4096;
const MAX_KIND_CHARS: usize = 128;
const MAX_FIELD_QUERIES: usize = 64;
const MAX_FIELD_ITEMS: usize = 8192;
const MAX_FIELD_BYTES: usize = 2 * 1024 * 1024;
const MAX_CONTRACT_ANCHORS: usize = 32;
static NEXT_SOURCE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, thiserror::Error)]
pub enum ContextOrgError {
    #[error("Org VM boundary failed: {0}")]
    Native(#[from] NativeError),
    #[error("Org Scheme operation failed: {0}")]
    Call(#[from] RootedCallError),
    #[error("invalid Org graph projection: {0}")]
    Invalid(&'static str),
}

/// A process-local identity bound to the exact opened Org source instance.
/// The ordinal alone must never be used to identify an element across sources.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OrgElementId {
    source: u64,
    ordinal: u32,
}

impl OrgElementId {
    pub fn ordinal(self) -> u32 {
        self.ordinal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgElementKind(String);

/// A parser-defined field name, including Org property names.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OrgFieldName(String);

impl OrgFieldName {
    pub fn new(value: impl Into<String>) -> Result<Self, ContextOrgError> {
        let value = value.into();
        if value.is_empty() || value.chars().take(129).count() > 128 {
            return Err(ContextOrgError::Invalid("invalid field name"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgFieldRequest {
    pub element: OrgElementId,
    pub name: OrgFieldName,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrgFieldValue {
    Missing,
    Text(String),
    TextList(Vec<String>),
}

macro_rules! contract_text_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub struct $name(String);
        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}
contract_text_id!(OrgContractSourceId);
contract_text_id!(OrgContractId);
contract_text_id!(OrgContractRevision);
contract_text_id!(OrgContractAnchorId);
contract_text_id!(OrgContractLocalId);
contract_text_id!(OrgParserId);
contract_text_id!(OrgSourceDigest);
contract_text_id!(OrgSemanticDigest);
contract_text_id!(OrgSessionIdentity);

impl OrgContractSourceId {
    pub fn new(value: impl Into<String>) -> Result<Self, ContextOrgError> {
        let value = value.into();
        if value.is_empty() || value.chars().take(257).count() > 256 {
            return Err(ContextOrgError::Invalid("invalid contract source identity"));
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrgSourceGeneration(u16);
impl OrgSourceGeneration {
    pub fn new(value: u16) -> Result<Self, ContextOrgError> {
        if value == 0 || value == u16::MAX {
            return Err(ContextOrgError::Invalid("invalid source generation"));
        }
        Ok(Self(value))
    }
    pub fn get(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgContractAnchor {
    pub identity: OrgContractAnchorId,
    pub local_id: OrgContractLocalId,
    pub kind: OrgElementKind,
    pub start_byte: usize,
}

/// Lab's source cut is data, not a durability, authentication or authorization receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgContractCut {
    pub identity: OrgContractSourceId,
    pub generation: OrgSourceGeneration,
    pub source: String,
    pub source_digest: OrgSourceDigest,
    pub semantic_digest: OrgSemanticDigest,
    pub contract: OrgContractId,
    pub contract_revision: OrgContractRevision,
    pub session: Option<OrgSessionIdentity>,
    pub parser: OrgParserId,
    pub anchors: Vec<OrgContractAnchor>,
}

pub struct OrgContractSource<'vm> {
    root: RootedSchemeObject<'vm>,
    identity: OrgContractSourceId,
    generation: OrgSourceGeneration,
    source: String,
}

impl OrgElementKind {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgElement {
    pub id: OrgElementId,
    pub parent: Option<OrgElementId>,
    pub kind: OrgElementKind,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// A source-preserving graph snapshot; fields and contract semantics stay in Scheme.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrgSourceGraph {
    pub source: String,
    pub elements: Vec<OrgElement>,
}

/// Strong root on the owning Scheme VM; it is not a serialized document.
pub struct OrgSource<'vm> {
    root: RootedSchemeObject<'vm>,
    identity: u64,
}

pub struct ContextOrgModule<'vm> {
    runtime: &'vm GerbilRuntime,
    publication: RootedSchemeObject<'vm>,
    open: RootedSchemeObject<'vm>,
    project: RootedSchemeObject<'vm>,
    fields: RootedSchemeObject<'vm>,
    contract_open: RootedSchemeObject<'vm>,
    contract_project: RootedSchemeObject<'vm>,
}

impl<'vm> ContextOrgModule<'vm> {
    /// Resolve the selected Yuanxu publication and its standard native procedures.
    pub fn resolve(runtime: &'vm GerbilRuntime, directory: &str) -> Result<Self, ContextOrgError> {
        fn export<'vm>(
            runtime: &'vm GerbilRuntime,
            directory: &str,
            module: &str,
            name: &str,
        ) -> Result<RootedSchemeObject<'vm>, ContextOrgError> {
            let module = runtime.intern_identifier(IdentifierKind::Symbol, module)?;
            let name = runtime.intern_identifier(IdentifierKind::Symbol, name)?;
            Ok(runtime.resolve_package_export(directory, &module, &name)?)
        }
        let publication = export(
            runtime,
            directory,
            ":marlin-agent-core/init",
            "marlin-agent-core.init.exports",
        )?;
        let open = export(
            runtime,
            directory,
            ":marlin-agent-core/c-abi/context",
            "marlin-context-org-source-open",
        )?;
        let project = export(
            runtime,
            directory,
            ":marlin-agent-core/c-abi/context",
            "marlin-context-org-source-project",
        )?;
        let fields = export(
            runtime,
            directory,
            ":marlin-agent-core/c-abi/context",
            "marlin-context-org-source-fields",
        )?;
        let contract_open = export(
            runtime,
            directory,
            ":marlin-agent-core/c-abi/context",
            "marlin-context-org-contract-open",
        )?;
        let contract_project = export(
            runtime,
            directory,
            ":marlin-agent-core/c-abi/context",
            "marlin-context-org-contract-project",
        )?;
        if open.as_ref().kind()? != ObjectKind::Procedure
            || project.as_ref().kind()? != ObjectKind::Procedure
            || fields.as_ref().kind()? != ObjectKind::Procedure
            || contract_open.as_ref().kind()? != ObjectKind::Procedure
            || contract_project.as_ref().kind()? != ObjectKind::Procedure
        {
            return Err(ContextOrgError::Invalid(
                "Context exports are not procedures",
            ));
        }
        Ok(Self {
            runtime,
            publication,
            open,
            project,
            fields,
            contract_open,
            contract_project,
        })
    }

    /// Parse source with Lab's selected Scheme Orgize implementation.
    pub fn open(&self, source: &str) -> Result<OrgSource<'vm>, ContextOrgError> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(ContextOrgError::Invalid("source exceeds one MiB"));
        }
        let text = self.runtime.string_from_utf8(source)?.into_object();
        let request = self
            .runtime
            .collection_from_objects(CollectionKind::Vector, &[&self.publication, &text])?;
        let root = self
            .runtime
            .call_rooted(&self.open, &request.into_object())?;
        let identity = NEXT_SOURCE_ID
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| ContextOrgError::Invalid("Org source identity space exhausted"))?;
        Ok(OrgSource { root, identity })
    }

    /// Admit an Org Contract source using Lab's parser and identity rules.
    pub fn open_contract(
        &self,
        identity: &OrgContractSourceId,
        generation: OrgSourceGeneration,
        source: &str,
    ) -> Result<OrgContractSource<'vm>, ContextOrgError> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(ContextOrgError::Invalid("source exceeds one MiB"));
        }
        let id = self
            .runtime
            .string_from_utf8(identity.as_str())?
            .into_object();
        let generation_value = self.runtime.scalar_object(SchemeScalar::Fixnum(
            isize::try_from(generation.0)
                .map_err(|_| ContextOrgError::Invalid("invalid source generation"))?,
        ))?;
        let text = self.runtime.string_from_utf8(source)?.into_object();
        let input = self.runtime.collection_from_objects(
            CollectionKind::Vector,
            &[&self.publication, &id, &generation_value, &text],
        )?;
        let root = self
            .runtime
            .call_rooted(&self.contract_open, &input.into_object())?;
        Ok(OrgContractSource {
            root,
            identity: identity.clone(),
            generation,
            source: source.to_owned(),
        })
    }

    /// Copy the Lab source cut without assigning it host-side authority.
    pub fn project_contract(
        &self,
        source: &OrgContractSource<'_>,
    ) -> Result<OrgContractCut, ContextOrgError> {
        let output = self
            .runtime
            .call_rooted(&self.contract_project, &source.root)?
            .into_collection(CollectionKind::Vector)?;
        if output.len() != 10 {
            return Err(ContextOrgError::Invalid("invalid contract envelope"));
        }
        let identity = text(output.element(0)?, 1024)?;
        let generation = integer(&output.element(1)?)?;
        let raw = text(output.element(2)?, MAX_SOURCE_BYTES)?;
        if identity != source.identity.as_str()
            || generation != usize::from(source.generation.0)
            || raw != source.source
        {
            return Err(ContextOrgError::Invalid(
                "contract source does not match request",
            ));
        }
        let source_digest = digest(output.element(3)?)?;
        let semantic_digest = digest(output.element(4)?)?;
        let contract = nonempty_text(output.element(5)?)?;
        let revision = nonempty_text(output.element(6)?)?;
        let session_value = output.element(7)?;
        let session = match session_value.as_ref().kind()? {
            ObjectKind::String => Some(OrgSessionIdentity(nonempty_text(session_value)?)),
            ObjectKind::Boolean
                if matches!(
                    session_value.as_ref().scalar()?,
                    SchemeScalar::Boolean(false)
                ) =>
            {
                None
            }
            _ => return Err(ContextOrgError::Invalid("invalid session identity")),
        };
        let parser = nonempty_text(output.element(8)?)?;
        let rows = output.element(9)?.into_collection(CollectionKind::Vector)?;
        if rows.is_empty() || rows.len() > MAX_CONTRACT_ANCHORS {
            return Err(ContextOrgError::Invalid("invalid anchor count"));
        }
        let mut anchors = Vec::with_capacity(rows.len());
        let mut ids = std::collections::HashSet::with_capacity(rows.len());
        let mut local_ids = std::collections::HashSet::with_capacity(rows.len());
        for i in 0..rows.len() {
            let row = rows.element(i)?.into_collection(CollectionKind::Vector)?;
            if row.len() != 4 {
                return Err(ContextOrgError::Invalid("invalid anchor"));
            }
            let anchor_id = digest(row.element(0)?)?;
            let local_id = nonempty_text(row.element(1)?)?;
            let kind = nonempty_text(row.element(2)?)?;
            let start = integer(&row.element(3)?)?;
            if start > raw.len()
                || !raw.is_char_boundary(start)
                || !ids.insert(anchor_id.clone())
                || !local_ids.insert(local_id.clone())
            {
                return Err(ContextOrgError::Invalid(
                    "invalid anchor identity or position",
                ));
            }
            anchors.push(OrgContractAnchor {
                identity: OrgContractAnchorId(anchor_id),
                local_id: OrgContractLocalId(local_id),
                kind: OrgElementKind(kind),
                start_byte: start,
            });
        }
        Ok(OrgContractCut {
            identity: source.identity.clone(),
            generation: source.generation,
            source: raw,
            source_digest: OrgSourceDigest(source_digest),
            semantic_digest: OrgSemanticDigest(semantic_digest),
            contract: OrgContractId(contract),
            contract_revision: OrgContractRevision(revision),
            session,
            parser: OrgParserId(parser),
            anchors,
        })
    }

    /// Project immutable source and structural element positions without reparsing in Rust.
    pub fn project(&self, source: &OrgSource<'_>) -> Result<OrgSourceGraph, ContextOrgError> {
        let identity = source.identity;
        let root = self
            .runtime
            .call_rooted(&self.project, &source.root)?
            .into_collection(CollectionKind::Vector)?;
        if root.len() != 2 {
            return Err(ContextOrgError::Invalid("invalid graph envelope"));
        }
        let source = root.element(0)?.into_string()?;
        if source.len().into_result()? > MAX_SOURCE_BYTES {
            return Err(ContextOrgError::Invalid("source exceeds one MiB"));
        }
        let source = source.to_string().into_result()?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(ContextOrgError::Invalid("source exceeds one MiB"));
        }
        let records = root.element(1)?.into_collection(CollectionKind::Vector)?;
        if records.len() > MAX_ELEMENTS {
            return Err(ContextOrgError::Invalid("too many elements"));
        }
        let mut elements = Vec::with_capacity(records.len());
        let mut indices = HashMap::with_capacity(records.len());
        for index in 0..records.len() {
            let row = records
                .element(index)?
                .into_collection(CollectionKind::Vector)?;
            if row.len() != 5 {
                return Err(ContextOrgError::Invalid("invalid element"));
            }
            let id = OrgElementId {
                source: identity,
                ordinal: integer(&row.element(0)?)?
                    .try_into()
                    .map_err(|_| ContextOrgError::Invalid("invalid identity"))?,
            };
            let parent = match row.element(1)?.as_ref().scalar()? {
                SchemeScalar::Boolean(false) => None,
                SchemeScalar::Fixnum(value) if value >= 0 => Some(OrgElementId {
                    source: identity,
                    ordinal: value
                        .try_into()
                        .map_err(|_| ContextOrgError::Invalid("invalid parent"))?,
                }),
                _ => return Err(ContextOrgError::Invalid("invalid parent")),
            };
            let kind = row.element(2)?.into_string()?;
            if kind.len().into_result()? > MAX_KIND_CHARS {
                return Err(ContextOrgError::Invalid("element kind too long"));
            }
            let kind = kind.to_string().into_result()?;
            let start = integer(&row.element(3)?)?;
            let end = integer(&row.element(4)?)?;
            if kind.is_empty()
                || kind.chars().take(MAX_KIND_CHARS + 1).count() > MAX_KIND_CHARS
                || start > end
                || end > source.len()
                || !source.is_char_boundary(start)
                || !source.is_char_boundary(end)
                || indices.insert(id, index).is_some()
            {
                return Err(ContextOrgError::Invalid(
                    "invalid element identity, kind or byte span",
                ));
            }
            elements.push(OrgElement {
                id,
                parent,
                kind: OrgElementKind(kind),
                start_byte: start,
                end_byte: end,
            });
        }
        for element in &elements {
            if let Some(parent_id) = element.parent {
                let Some(&index) = indices.get(&parent_id) else {
                    return Err(ContextOrgError::Invalid("missing parent"));
                };
                let parent = &elements[index];
                if parent.id.ordinal >= element.id.ordinal
                    || parent.start_byte > element.start_byte
                    || parent.end_byte < element.end_byte
                {
                    return Err(ContextOrgError::Invalid("invalid parent relation"));
                }
            }
        }
        Ok(OrgSourceGraph { source, elements })
    }

    /// Query parser-owned fields in request order with one bounded VM call.
    pub fn fields(
        &self,
        source: &OrgSource<'_>,
        requests: &[OrgFieldRequest],
    ) -> Result<Vec<OrgFieldValue>, ContextOrgError> {
        if requests.len() > MAX_FIELD_QUERIES {
            return Err(ContextOrgError::Invalid("too many field requests"));
        }
        if requests.is_empty() {
            return Ok(Vec::new());
        }
        let mut rows = Vec::with_capacity(requests.len());
        for request in requests {
            if request.element.source != source.identity {
                return Err(ContextOrgError::Invalid(
                    "field request belongs to another Org source",
                ));
            }
            let id = isize::try_from(request.element.ordinal)
                .map_err(|_| ContextOrgError::Invalid("invalid element identity"))?;
            let id = self.runtime.scalar_object(SchemeScalar::Fixnum(id))?;
            let name = self
                .runtime
                .string_from_utf8(request.name.as_str())?
                .into_object();
            rows.push(
                self.runtime
                    .collection_from_objects(CollectionKind::Vector, &[&id, &name])?
                    .into_object(),
            );
        }
        let row_refs: Vec<_> = rows.iter().collect();
        let rows = self
            .runtime
            .collection_from_objects(CollectionKind::Vector, &row_refs)?
            .into_object();
        let input = self
            .runtime
            .collection_from_objects(CollectionKind::Vector, &[&source.root, &rows])?;
        let result = self
            .runtime
            .call_rooted(&self.fields, &input.into_object())?
            .into_collection(CollectionKind::Vector)?;
        if result.len() != requests.len() {
            return Err(ContextOrgError::Invalid("field result count mismatch"));
        }
        let mut values = Vec::with_capacity(result.len());
        let mut bytes = 0usize;
        let mut items = 0usize;
        for index in 0..result.len() {
            let value = result.element(index)?;
            let field = match value.as_ref().kind()? {
                ObjectKind::String => OrgFieldValue::Text(field_text(value, &mut bytes)?),
                ObjectKind::Vector => {
                    let entries = value.into_collection(CollectionKind::Vector)?;
                    items = items
                        .checked_add(entries.len())
                        .ok_or(ContextOrgError::Invalid("too many field values"))?;
                    if items > MAX_FIELD_ITEMS {
                        return Err(ContextOrgError::Invalid("too many field values"));
                    }
                    let mut list = Vec::with_capacity(entries.len());
                    for i in 0..entries.len() {
                        list.push(field_text(entries.element(i)?, &mut bytes)?);
                    }
                    OrgFieldValue::TextList(list)
                }
                ObjectKind::Boolean
                    if matches!(value.as_ref().scalar()?, SchemeScalar::Boolean(false)) =>
                {
                    OrgFieldValue::Missing
                }
                _ => return Err(ContextOrgError::Invalid("unsupported field value")),
            };
            values.push(field);
        }
        Ok(values)
    }
}

fn text(value: RootedSchemeObject<'_>, max_bytes: usize) -> Result<String, ContextOrgError> {
    let value = value.into_string()?;
    if value.len().into_result()? > max_bytes {
        return Err(ContextOrgError::Invalid("text exceeds limit"));
    }
    let value = value.to_string().into_result()?;
    if value.len() > max_bytes {
        return Err(ContextOrgError::Invalid("text exceeds limit"));
    }
    Ok(value)
}

fn nonempty_text(value: RootedSchemeObject<'_>) -> Result<String, ContextOrgError> {
    let value = text(value, 1024)?;
    if value.is_empty() || value.chars().take(257).count() > 256 {
        return Err(ContextOrgError::Invalid("invalid contract text"));
    }
    Ok(value)
}

fn digest(value: RootedSchemeObject<'_>) -> Result<String, ContextOrgError> {
    let value = text(value, 71)?;
    if value.len() != 71
        || !value.starts_with("sha256:")
        || !value.as_bytes()[7..]
            .iter()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
    {
        return Err(ContextOrgError::Invalid("invalid digest"));
    }
    Ok(value)
}

fn field_text(value: RootedSchemeObject<'_>, total: &mut usize) -> Result<String, ContextOrgError> {
    let value = value.into_string()?;
    let remaining = MAX_FIELD_BYTES.saturating_sub(*total);
    if value.len().into_result()? > remaining {
        return Err(ContextOrgError::Invalid("field output exceeds byte limit"));
    }
    let value = value.to_string().into_result()?;
    *total = total
        .checked_add(value.len())
        .ok_or(ContextOrgError::Invalid("field output exceeds byte limit"))?;
    if *total > MAX_FIELD_BYTES {
        return Err(ContextOrgError::Invalid("field output exceeds byte limit"));
    }
    Ok(value)
}

fn integer(value: &RootedSchemeObject<'_>) -> Result<usize, ContextOrgError> {
    match value.as_ref().scalar()? {
        SchemeScalar::Fixnum(value) if value >= 0 => Ok(value as usize),
        _ => Err(ContextOrgError::Invalid("expected nonnegative integer")),
    }
}

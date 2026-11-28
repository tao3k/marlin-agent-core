// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Direct in-process conversion between live `gerbil_scheme::RootedSchemeObject`
//! handles and Rust `GerbilSchemeValue` / `serde::de::DeserializeOwned` types.
//!
//! This closes the native Gambit C-ABI -> `scheme_types::GerbilSchemeValue` ->
//! Rust struct pipeline so Scheme bridge modules never hand-roll per-field
//! `decode_string` / `decode_vector` / `decode_u32` boilerplate.

#![cfg(not(feature = "external-program"))]

use std::collections::BTreeMap;

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, ObjectKind, RootedObjectBatch,
    RootedSchemeObject, SchemeScalar, TableEquality,
};
use serde::de::DeserializeOwned;

use super::{error::GerbilSchemeTypeDecodeError, typed_value::GerbilSchemeValue};

const MAX_NATIVE_OBJECT_DEPTH: usize = 64;

fn projection_err(context: &str, detail: impl std::fmt::Display) -> GerbilSchemeTypeDecodeError {
    GerbilSchemeTypeDecodeError::RustProjection {
        message: format!("{context}: {detail}"),
    }
}

impl GerbilSchemeValue {
    /// Project a live GC-rooted Gambit/Gerbil object directly into a typed
    /// `GerbilSchemeValue` without any text or JSON serialization.
    pub fn from_rooted_object(
        obj: RootedSchemeObject<'_>,
    ) -> Result<Self, GerbilSchemeTypeDecodeError> {
        from_rooted_object_depth(obj, 0)
    }

    /// Construct a live GC-rooted Gambit/Gerbil object directly from a Rust
    /// `GerbilSchemeValue` on the owning VM thread.
    pub fn to_rooted_object<'rt>(
        &self,
        runtime: &'rt GerbilRuntime,
    ) -> Result<RootedSchemeObject<'rt>, GerbilSchemeTypeDecodeError> {
        match self {
            Self::Null => runtime
                .nil_object()
                .map_err(|e| projection_err("alloc nil", e)),
            Self::Boolean(b) => runtime
                .scalar_object(SchemeScalar::Boolean(*b))
                .map_err(|e| projection_err("alloc boolean", e)),
            Self::Integer(n) => {
                let fix = isize::try_from(*n)
                    .map_err(|_| projection_err("alloc fixnum", "integer out of fixnum range"))?;
                runtime
                    .scalar_object(SchemeScalar::Fixnum(fix))
                    .map_err(|e| projection_err("alloc fixnum", e))
            }
            Self::Float(f) => runtime
                .scalar_object(SchemeScalar::Flonum(*f))
                .map_err(|e| projection_err("alloc flonum", e)),
            Self::Text(s) => runtime
                .string_from_utf8(s)
                .map_err(|e| projection_err("alloc string", e))
                .map(|s| s.into_object()),
            Self::Vector(items) => {
                let mut rooted = Vec::with_capacity(items.len());
                for item in items {
                    rooted.push(item.to_rooted_object(runtime)?);
                }
                let refs: Vec<&RootedSchemeObject<'rt>> = rooted.iter().collect();
                runtime
                    .collection_from_objects(CollectionKind::Vector, &refs)
                    .map_err(|e| projection_err("alloc vector", e))
                    .map(|c| c.into_object())
            }
            Self::Record(fields) => {
                let mut pairs = Vec::with_capacity(fields.len());
                for (k, v) in fields {
                    let k_obj = runtime
                        .string_from_utf8(k)
                        .map_err(|e| projection_err("alloc record key", e))?
                        .into_object();
                    let v_obj = v.to_rooted_object(runtime)?;
                    pairs.push((k_obj, v_obj));
                }
                let pair_refs: Vec<(&RootedSchemeObject<'rt>, &RootedSchemeObject<'rt>)> =
                    pairs.iter().map(|(k, v)| (k, v)).collect();
                runtime
                    .hash_table(TableEquality::Structural, &pair_refs)
                    .map_err(|e| projection_err("alloc hash table", e))
            }
        }
    }
}

/// Directly convert a live GC-rooted Gambit/Gerbil object into any Rust
/// `DeserializeOwned` type through `GerbilSchemeValue`'s native serde deserializer.
pub fn project_rooted_scheme_object<T>(
    obj: RootedSchemeObject<'_>,
) -> Result<T, GerbilSchemeTypeDecodeError>
where
    T: DeserializeOwned,
{
    GerbilSchemeValue::from_rooted_object(obj)?.decode()
}

fn from_rooted_object_depth(
    obj: RootedSchemeObject<'_>,
    depth: usize,
) -> Result<GerbilSchemeValue, GerbilSchemeTypeDecodeError> {
    if depth > MAX_NATIVE_OBJECT_DEPTH {
        return Err(projection_err(
            "native Scheme projection",
            "nesting depth exceeded",
        ));
    }

    let kind = obj
        .as_ref()
        .kind()
        .map_err(|e| projection_err("inspect object kind", e))?;
    match kind {
        ObjectKind::Void | ObjectKind::Nil => Ok(GerbilSchemeValue::Null),
        ObjectKind::Boolean => match obj
            .as_ref()
            .scalar()
            .map_err(|e| projection_err("read boolean", e))?
        {
            SchemeScalar::Boolean(b) => Ok(GerbilSchemeValue::Boolean(b)),
            _ => Err(projection_err("read boolean", "unexpected scalar")),
        },
        ObjectKind::ExactInteger => match obj
            .as_ref()
            .scalar()
            .map_err(|e| projection_err("read exact integer", e))?
        {
            SchemeScalar::Fixnum(n) => Ok(GerbilSchemeValue::Integer(n as i64)),
            _ => Err(projection_err("read exact integer", "unexpected scalar")),
        },
        ObjectKind::Flonum => match obj
            .as_ref()
            .scalar()
            .map_err(|e| projection_err("read flonum", e))?
        {
            SchemeScalar::Flonum(f) => Ok(GerbilSchemeValue::Float(f)),
            _ => Err(projection_err("read flonum", "unexpected scalar")),
        },
        ObjectKind::Character => match obj
            .as_ref()
            .scalar()
            .map_err(|e| projection_err("read char", e))?
        {
            SchemeScalar::Char(ch) => Ok(GerbilSchemeValue::Text(ch.to_string())),
            _ => Err(projection_err("read char", "unexpected scalar")),
        },
        ObjectKind::String => {
            let text = obj
                .into_string()
                .map_err(|e| projection_err("into string", e))?
                .to_string()
                .into_result()
                .map_err(|e| projection_err("copy utf-8 string", e))?;
            Ok(GerbilSchemeValue::Text(text))
        }
        ObjectKind::Symbol | ObjectKind::Keyword => {
            let mut buf = IdentifierBuffer::default();
            obj.as_ref()
                .copy_identifier_names(&mut buf)
                .map_err(|e| projection_err("copy identifier name", e))?;
            let (_, name) = buf
                .get(0)
                .ok_or_else(|| projection_err("copy identifier name", "empty buffer"))?;
            Ok(GerbilSchemeValue::Text(name.to_owned()))
        }
        ObjectKind::Vector => {
            let coll = obj
                .into_collection(CollectionKind::Vector)
                .map_err(|e| projection_err("into vector", e))?;
            let mut items = Vec::with_capacity(coll.len());
            for i in 0..coll.len() {
                let elem = coll
                    .element(i)
                    .map_err(|e| projection_err("vector element", e))?;
                items.push(from_rooted_object_depth(elem, depth + 1)?);
            }
            Ok(GerbilSchemeValue::Vector(items))
        }
        ObjectKind::Pair => {
            let coll = obj
                .into_collection(CollectionKind::List)
                .map_err(|e| projection_err("into list", e))?;
            let mut items = Vec::with_capacity(coll.len());
            for i in 0..coll.len() {
                let elem = coll
                    .element(i)
                    .map_err(|e| projection_err("list element", e))?;
                items.push(from_rooted_object_depth(elem, depth + 1)?);
            }
            Ok(GerbilSchemeValue::Vector(items))
        }
        ObjectKind::GambitTable | ObjectKind::GerbilHashTable => {
            let mut batch = RootedObjectBatch::default();
            obj.copy_table_entries(&mut batch)
                .map_err(|e| projection_err("copy hash table entries", e))?;
            let mut fields = BTreeMap::new();
            let mut idx = 0;
            while idx + 1 < batch.len() {
                let k_obj = batch
                    .root_element(idx)
                    .map_err(|e| projection_err("table key", e))?;
                let v_obj = batch
                    .root_element(idx + 1)
                    .map_err(|e| projection_err("table value", e))?;
                let key = match from_rooted_object_depth(k_obj, depth + 1)? {
                    GerbilSchemeValue::Text(s) => s,
                    other => other.to_string(),
                };
                let val = from_rooted_object_depth(v_obj, depth + 1)?;
                fields.insert(key, val);
                idx += 2;
            }
            Ok(GerbilSchemeValue::Record(fields))
        }
        other => Err(projection_err(
            "unsupported Scheme object kind",
            format!("{other:?}"),
        )),
    }
}

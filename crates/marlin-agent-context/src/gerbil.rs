// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Owner-local bridge to the Lab-backed Scheme Context module.

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, LinkedObjectExport, NativeError as SchemeVmError,
    RootedSchemeObject,
};

use super::{CONTEXT_BATCH_MAX_ROWS, ContextScopeBatch, ContextScopeDigest, SCOPE_DIGEST_BYTES};

#[derive(Clone, Debug, thiserror::Error)]
pub enum ContextBridgeError {
    #[error("Context VM call failed: {0}")]
    Bridge(#[from] SchemeVmError),
    #[error("Context result requires between 1 and {CONTEXT_BATCH_MAX_ROWS} rows; got {0}")]
    RowCount(usize),
    #[error("Context VM returned {actual} digest bytes; expected {expected}")]
    OutputLength { expected: usize, actual: usize },
}

/// Calls the build-owned Context export on the same VM as the input roots.
pub struct ContextAdmissionBridge<'vm> {
    runtime: &'vm GerbilRuntime,
    export: LinkedObjectExport<'vm>,
}

impl<'vm> ContextAdmissionBridge<'vm> {
    /// # Safety
    ///
    /// The export must be `marlin_context_admit_batch` from this runtime's
    /// initialized AOT graph. The upstream strong-root ABI must be in force.
    pub unsafe fn bind(
        runtime: &'vm GerbilRuntime,
        export: unsafe extern "C" fn(i64) -> i64,
    ) -> Result<Self, ContextBridgeError> {
        // SAFETY: the caller attests to the initialized, build-owned ABI export.
        let export = unsafe { runtime.bind_object_export(export) }?;
        Ok(Self { runtime, export })
    }

    /// Return one owned digest per row, in submission order. A bridge error or
    /// malformed result discards the entire batch before any digest is returned.
    pub fn admit(
        &self,
        batch: &ContextScopeBatch<'_>,
    ) -> Result<Vec<ContextScopeDigest>, ContextBridgeError> {
        let input = project_scope_batch(self.runtime, batch)?;
        let output = self.export.call(&input).into_result()?;
        project_scope_digests(output, batch.rows.len())
    }
}

/// Materialize the existing typed Context envelope on its VM owner.
pub fn project_scope_batch<'vm>(
    runtime: &'vm GerbilRuntime,
    batch: &ContextScopeBatch<'_>,
) -> Result<RootedSchemeObject<'vm>, ContextBridgeError> {
    let shared = text_vector(runtime, batch.key.text())?;
    let numbers: Vec<u64> = std::iter::once(batch.key.epoch.0)
        .chain(
            batch
                .rows
                .iter()
                .map(|row| row.session_turn.as_ref().map_or(0, |t| t.turn.0)),
        )
        .collect();
    let numbers = runtime.numeric_vector(&numbers)?;
    let rows: Vec<_> = batch
        .rows
        .iter()
        .map(|row| text_vector(runtime, row.text()))
        .collect::<Result<_, _>>()?;
    let rows = vector(runtime, &rows.iter().collect::<Vec<_>>())?;
    let input = vector(runtime, &[&shared, &numbers, &rows])?;
    Ok(input)
}

/// Copy Lab's ordered SHA-256 scope identities into owned Rust values.
pub fn project_scope_digests(
    output: RootedSchemeObject<'_>,
    rows: usize,
) -> Result<Vec<ContextScopeDigest>, ContextBridgeError> {
    if rows == 0 || rows > CONTEXT_BATCH_MAX_ROWS {
        return Err(ContextBridgeError::RowCount(rows));
    }
    let result = output.into_bytevector()?;
    let expected = rows * SCOPE_DIGEST_BYTES;
    let actual = result.len().into_result()?;
    if actual != expected {
        return Err(ContextBridgeError::OutputLength { expected, actual });
    }
    let mut bytes = [0; CONTEXT_BATCH_MAX_ROWS * SCOPE_DIGEST_BYTES];
    result.copy_into(&mut bytes[..expected]).into_result()?;
    Ok(bytes[..expected]
        .as_chunks::<SCOPE_DIGEST_BYTES>()
        .0
        .iter()
        .copied()
        .map(ContextScopeDigest)
        .collect())
}

fn text_vector<'vm, 'a>(
    runtime: &'vm GerbilRuntime,
    fields: impl IntoIterator<Item = &'a str>,
) -> Result<RootedSchemeObject<'vm>, SchemeVmError> {
    let roots: Vec<_> = fields
        .into_iter()
        .map(|value| {
            runtime
                .string_from_utf8(value)
                .map(|root| root.into_object())
        })
        .collect::<Result<_, _>>()?;
    vector(runtime, &roots.iter().collect::<Vec<_>>())
}

fn vector<'vm>(
    runtime: &'vm GerbilRuntime,
    roots: &[&RootedSchemeObject<'_>],
) -> Result<RootedSchemeObject<'vm>, SchemeVmError> {
    Ok(runtime
        .collection_from_objects(CollectionKind::Vector, roots)?
        .into_object())
}

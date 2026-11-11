//! Typed Context projections into the build-owned Scheme Actor graph.

use super::{DeckRuntimeRejection, SchemeVmError};
use gerbil_scheme::{
    CollectionKind, GerbilRuntime, LinkedObjectExport, RootedSchemeObject, SchemeScalar,
};
use marlin_agent_context::{
    ContextAdmissionRequest, ContextScopeBatch, ContextScopeDigest,
    gerbil::{project_scope_batch, project_scope_digests},
};
use std::sync::Arc;

unsafe extern "C" {
    pub(super) fn ___LNK_marlin__deck__actor(
        state: *mut gerbil_scheme_sys::GerbilGlobalState,
    ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
    fn marlin_deck_runtime_open(root: i64) -> i64;
    fn marlin_deck_runtime_evaluate(root: i64) -> i64;
    fn marlin_deck_runtime_close(root: i64) -> i64;
}

pub(super) struct DeckVm<'vm> {
    runtime: &'vm GerbilRuntime,
    evaluate: LinkedObjectExport<'vm>,
    close: LinkedObjectExport<'vm>,
    control: RootedSchemeObject<'vm>,
    closed: bool,
}

impl<'vm> DeckVm<'vm> {
    pub(super) fn open(runtime: &'vm GerbilRuntime) -> Result<Self, SchemeVmError> {
        // SAFETY: these exports belong to the initialized, build-owned AOT graph.
        let (open, evaluate, close) = unsafe {
            (
                runtime.bind_object_export(marlin_deck_runtime_open)?,
                runtime.bind_object_export(marlin_deck_runtime_evaluate)?,
                runtime.bind_object_export(marlin_deck_runtime_close)?,
            )
        };
        let control = runtime.scalar_object(SchemeScalar::Boolean(false))?;
        let result = open.call(&control).into_result()?;
        // The C ABI uses a Boolean acknowledgement, never a serialized receipt.
        if result.as_ref().scalar()? != SchemeScalar::Boolean(true) {
            return Err(SchemeVmError::Status {
                operation: "Deck lifecycle acknowledgement",
                code: -1,
            });
        }
        Ok(Self {
            runtime,
            evaluate,
            close,
            control,
            closed: false,
        })
    }

    pub(super) fn evaluate(
        &self,
        requests: &[&ContextAdmissionRequest],
    ) -> Result<Vec<ContextScopeDigest>, DeckRuntimeRejection> {
        let mut groups = Vec::new();
        let mut start = 0;
        while start < requests.len() {
            let first = requests[start];
            let mut end = start + 1;
            while end < requests.len() && Arc::ptr_eq(first.scope(), requests[end].scope()) {
                end += 1;
            }
            let rows: Vec<_> = requests[start..end]
                .iter()
                .map(|request| request.turn().as_row())
                .collect();
            let batch = ContextScopeBatch::new(first.scope().as_key(), &rows)
                .map_err(|_| DeckRuntimeRejection::InvalidProjection)?;
            groups.push(project_scope_batch(self.runtime, &batch)?);
            start = end;
        }
        let input = self.vector(&groups.iter().collect::<Vec<_>>())?;
        let output = self.evaluate.call(&input).into_result()?;
        Ok(project_scope_digests(output, requests.len())?)
    }

    fn vector(
        &self,
        roots: &[&RootedSchemeObject<'_>],
    ) -> Result<RootedSchemeObject<'vm>, SchemeVmError> {
        Ok(self
            .runtime
            .collection_from_objects(CollectionKind::Vector, roots)?
            .into_object())
    }

    pub(super) fn close(&mut self) -> Result<(), SchemeVmError> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        let result = self.close.call(&self.control).into_result()?;
        if result.as_ref().scalar()? != SchemeScalar::Boolean(true) {
            return Err(SchemeVmError::Status {
                operation: "Deck lifecycle acknowledgement",
                code: -1,
            });
        }
        Ok(())
    }
}

impl Drop for DeckVm<'_> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

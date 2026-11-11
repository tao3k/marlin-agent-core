//! Typed Context projections into the build-owned Scheme Actor graph.

use super::{DeckMailboxMetrics, DeckRuntimeRejection, SchemeVmError};
use gerbil_scheme::{
    CollectionKind, GerbilRuntime, LinkedObjectExport, NumericBuffer, RootedSchemeObject,
    SchemeScalar,
};
use marlin_agent_context::{
    ContextAdmissionRequest, ContextScopeBatch, ContextScopeDigest,
    gerbil::{project_scope_batch, project_scope_digests},
};
use std::{num::NonZeroU16, sync::Arc};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct MessageId(pub u64);

pub(super) struct Completion {
    pub id: MessageId,
    pub outcome: Result<ContextScopeDigest, DeckRuntimeRejection>,
}

unsafe extern "C" {
    pub(super) fn ___LNK_marlin__deck__actor(
        state: *mut gerbil_scheme_sys::GerbilGlobalState,
    ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
    fn marlin_deck_runtime_open(root: i64) -> i64;
    fn marlin_deck_runtime_submit(root: i64) -> i64;
    fn marlin_deck_runtime_receive(root: i64) -> i64;
    fn marlin_deck_runtime_close(root: i64) -> i64;
}

pub(super) struct DeckVm<'vm> {
    runtime: &'vm GerbilRuntime,
    submit: LinkedObjectExport<'vm>,
    receive: LinkedObjectExport<'vm>,
    close: LinkedObjectExport<'vm>,
    control: RootedSchemeObject<'vm>,
    closed: bool,
    ids: NumericBuffer<u64>,
    final_metrics: DeckMailboxMetrics,
}

impl<'vm> DeckVm<'vm> {
    pub(super) fn open(runtime: &'vm GerbilRuntime) -> Result<Self, SchemeVmError> {
        // SAFETY: these exports belong to the initialized, build-owned AOT graph.
        let (open, submit, receive, close) = unsafe {
            (
                runtime.bind_object_export(marlin_deck_runtime_open)?,
                runtime.bind_object_export(marlin_deck_runtime_submit)?,
                runtime.bind_object_export(marlin_deck_runtime_receive)?,
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
            submit,
            receive,
            close,
            control,
            closed: false,
            ids: NumericBuffer::default(),
            final_metrics: DeckMailboxMetrics::default(),
        })
    }

    pub(super) fn submit(
        &mut self,
        requests: &[&ContextAdmissionRequest],
    ) -> Result<Vec<MessageId>, DeckRuntimeRejection> {
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
        let output = self.submit.call(&input).into_result()?;
        output.as_ref().copy_numeric(&mut self.ids)?;
        if self.ids.as_slice().len() != requests.len() {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        Ok(self.ids.as_slice().iter().copied().map(MessageId).collect())
    }

    pub(super) fn receive(
        &mut self,
        capacity: NonZeroU16,
    ) -> Result<Vec<Completion>, DeckRuntimeRejection> {
        let maximum = self.runtime.scalar_object(SchemeScalar::Fixnum(
            isize::try_from(capacity.get()).expect("validated transfer capacity"),
        ))?;
        let packet = self
            .receive
            .call(&maximum)
            .into_result()?
            .into_collection(CollectionKind::Vector)?;
        if packet.len() != 3 {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        packet.element(0)?.as_ref().copy_numeric(&mut self.ids)?;
        let ids = self.ids.as_slice();
        if ids.is_empty() || ids.len() > usize::from(capacity.get()) {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        let status = packet.element(1)?.into_bytevector()?;
        if status.len().into_result()? != ids.len() {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        let mut statuses = [0; marlin_agent_context::CONTEXT_BATCH_MAX_ROWS];
        status.copy_into(&mut statuses[..ids.len()]).into_result()?;
        let statuses = &statuses[..ids.len()];
        if statuses.iter().any(|status| *status > 1) {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        let successful = statuses.iter().filter(|status| **status == 0).count();
        let output = packet.element(2)?;
        let digests = if successful == 0 {
            if !output.into_bytevector()?.is_empty().into_result()? {
                return Err(DeckRuntimeRejection::InvalidProjection);
            }
            Vec::new()
        } else {
            project_scope_digests(output, successful)?
        };
        let mut digests = digests.into_iter();
        Ok(ids
            .iter()
            .zip(statuses)
            .map(|(id, status)| Completion {
                id: MessageId(*id),
                outcome: if *status == 0 {
                    Ok(digests.next().expect("validated digest cardinality"))
                } else {
                    Err(DeckRuntimeRejection::ContextActor)
                },
            })
            .collect())
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

    pub(super) fn close(&mut self) -> Result<DeckMailboxMetrics, SchemeVmError> {
        if self.closed {
            return Ok(self.final_metrics);
        }
        self.closed = true;
        let result = self.close.call(&self.control).into_result()?;
        result.as_ref().copy_numeric(&mut self.ids)?;
        let [created, retired, submitted, collected] = self.ids.as_slice() else {
            return Err(SchemeVmError::Status {
                operation: "Deck mailbox retirement receipt",
                code: -1,
            });
        };
        if created != retired || submitted != collected {
            return Err(SchemeVmError::Status {
                operation: "Deck mailbox retirement accounting",
                code: -1,
            });
        }
        self.final_metrics = DeckMailboxMetrics {
            actors_created: *created,
            actors_retired: *retired,
            messages_submitted: *submitted,
            results_collected: *collected,
        };
        Ok(self.final_metrics)
    }
}

impl Drop for DeckVm<'_> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

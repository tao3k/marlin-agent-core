// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed Context projections into the build-owned Scheme Actor graph.

use super::{
    DeckMailboxMetrics, DeckModuleInstanceId, DeckModuleRetirementReceipt, DeckRuntimeRejection,
    DeckTimerId, DeckTimerReceipt, SchemeVmError,
};
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

pub(super) struct Poll {
    pub completions: Vec<Completion>,
    pub timers: Vec<DeckTimerReceipt>,
}
unsafe extern "C" {
    pub(super) fn ___LNK_marlin__deck__actor(
        state: *mut gerbil_scheme_sys::GerbilGlobalState,
    ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
    fn marlin_deck_runtime_open(root: i64) -> i64;
    #[cfg(any(debug_assertions, feature = "context-profiling"))]
    fn gerbil_scheme_rust_take_call_error_raw() -> i64;
    fn marlin_deck_runtime_submit(root: i64) -> i64;
    fn marlin_deck_runtime_receive(root: i64) -> i64;
    fn marlin_deck_runtime_close(root: i64) -> i64;
    fn marlin_deck_runtime_activate(root: i64) -> i64;
    fn marlin_deck_runtime_retire(root: i64) -> i64;
    fn marlin_deck_runtime_arm(root: i64) -> i64;
    #[cfg(feature = "context-profiling")]
    fn marlin_deck_runtime_statistics(root: i64) -> i64;
}

pub(super) struct DeckVm<'vm> {
    runtime: &'vm GerbilRuntime,
    submit: LinkedObjectExport<'vm>,
    receive: LinkedObjectExport<'vm>,
    activate: LinkedObjectExport<'vm>,
    retire_module: LinkedObjectExport<'vm>,
    arm: LinkedObjectExport<'vm>,
    close: LinkedObjectExport<'vm>,
    control: RootedSchemeObject<'vm>,
    closed: bool,
    ids: NumericBuffer<u64>,
    final_metrics: DeckMailboxMetrics,
    #[cfg(feature = "context-profiling")]
    pub(super) profile: super::profile::VmStages,
}

impl<'vm> DeckVm<'vm> {
    pub(super) fn open(
        runtime: &'vm GerbilRuntime,
        wake_fds: [u64; 2],
    ) -> Result<Self, SchemeVmError> {
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
        let wake = runtime.numeric_vector(&wake_fds)?;
        let result = open.call(&wake).into_result().inspect_err(|_| {
            #[cfg(any(debug_assertions, feature = "context-profiling"))]
            emit_scheme_diagnostic(runtime, &control);
        })?;
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
            // SAFETY: these exports are in the same build-owned linked graph.
            activate: unsafe { runtime.bind_object_export(marlin_deck_runtime_activate)? },
            retire_module: unsafe { runtime.bind_object_export(marlin_deck_runtime_retire)? },
            arm: unsafe { runtime.bind_object_export(marlin_deck_runtime_arm)? },
            control,
            closed: false,
            ids: NumericBuffer::default(),
            final_metrics: DeckMailboxMetrics::default(),
            #[cfg(feature = "context-profiling")]
            profile: super::profile::VmStages::default(),
        })
    }

    pub(super) fn submit(
        &mut self,
        requests: &[(DeckModuleInstanceId, &ContextAdmissionRequest)],
    ) -> Result<Vec<MessageId>, DeckRuntimeRejection> {
        #[cfg(feature = "context-profiling")]
        let begin = std::time::Instant::now();
        let mut groups = Vec::new();
        let mut module_ids = Vec::new();
        let mut start = 0;
        while start < requests.len() {
            let (module, first) = requests[start];
            let mut end = start + 1;
            while end < requests.len()
                && requests[end].0 == module
                && Arc::ptr_eq(first.scope(), requests[end].1.scope())
            {
                end += 1;
            }
            let rows: Vec<_> = requests[start..end]
                .iter()
                .map(|(_, request)| request.turn().as_row())
                .collect();
            let batch = ContextScopeBatch::new(first.scope().as_key(), &rows)
                .map_err(|_| DeckRuntimeRejection::InvalidProjection)?;
            groups.push(project_scope_batch(self.runtime, &batch)?);
            module_ids.push(module.get());
            start = end;
        }
        let groups = self.vector(&groups.iter().collect::<Vec<_>>())?;
        let module_ids = self.runtime.numeric_vector(&module_ids)?;
        let input = self.vector(&[&module_ids, &groups])?;
        #[cfg(feature = "context-profiling")]
        let submitted = std::time::Instant::now();
        #[cfg(feature = "context-profiling")]
        {
            self.profile.input_us = submitted.duration_since(begin).as_micros();
        }
        let output = self.submit.call(&input).into_result()?;
        #[cfg(feature = "context-profiling")]
        {
            self.profile.submit_us = submitted.elapsed().as_micros();
        }
        output.as_ref().copy_numeric(&mut self.ids)?;
        if self.ids.as_slice().len() != requests.len() {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        Ok(self.ids.as_slice().iter().copied().map(MessageId).collect())
    }

    pub(super) fn receive(&mut self, capacity: NonZeroU16) -> Result<Poll, DeckRuntimeRejection> {
        let maximum = self.runtime.scalar_object(SchemeScalar::Fixnum(
            isize::try_from(capacity.get()).expect("validated transfer capacity"),
        ))?;
        #[cfg(feature = "context-profiling")]
        let entered = std::time::Instant::now();
        let packet = self
            .receive
            .call(&maximum)
            .into_result()?
            .into_collection(CollectionKind::Vector)?;
        #[cfg(feature = "context-profiling")]
        let projected = std::time::Instant::now();
        #[cfg(feature = "context-profiling")]
        {
            self.profile.receive_us = projected.duration_since(entered).as_micros();
        }
        if packet.len() != 4 {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        packet.element(0)?.as_ref().copy_numeric(&mut self.ids)?;
        let ids = self.ids.as_slice();
        if ids.len() > usize::from(capacity.get()) {
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
        #[cfg(feature = "context-profiling")]
        {
            self.profile.output_us = projected.elapsed().as_micros();
        }
        let completions = ids
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
            .collect();
        let mut notices = NumericBuffer::<u64>::default();
        packet.element(3)?.as_ref().copy_numeric(&mut notices)?;
        if !notices.as_slice().len().is_multiple_of(5)
            || ids.len() + notices.as_slice().len() / 5 > usize::from(capacity.get())
            || ids.len() + notices.as_slice().len() / 5 == 0
        {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        let mut timers = Vec::new();
        for notice in notices.as_slice().chunks_exact(5) {
            match *notice {
                [0, 0, 0, 0, 0] => (),
                [1, module, id, due, observed] if observed >= due => {
                    let module = std::num::NonZeroU64::new(module)
                        .map(DeckModuleInstanceId)
                        .ok_or(DeckRuntimeRejection::InvalidProjection)?;
                    let timer = std::num::NonZeroU64::new(id)
                        .map(DeckTimerId)
                        .ok_or(DeckRuntimeRejection::InvalidProjection)?;
                    timers.push(DeckTimerReceipt {
                        module,
                        timer,
                        due_monotonic_us: due,
                        observed_monotonic_us: observed,
                    });
                }
                _ => return Err(DeckRuntimeRejection::InvalidProjection),
            }
        }
        Ok(Poll {
            completions,
            timers,
        })
    }

    fn acknowledge(
        &self,
        export: &LinkedObjectExport<'vm>,
        values: &[u64],
    ) -> Result<(), DeckRuntimeRejection> {
        let input = self.runtime.numeric_vector(values)?;
        if export.call(&input).into_result()?.as_ref().scalar()? != SchemeScalar::Boolean(true) {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        Ok(())
    }
    pub(super) fn activate(
        &self,
        module: DeckModuleInstanceId,
    ) -> Result<(), DeckRuntimeRejection> {
        self.acknowledge(&self.activate, &[module.get()])
    }
    pub(super) fn arm(
        &self,
        module: DeckModuleInstanceId,
        timer: DeckTimerId,
        delay_us: u64,
    ) -> Result<(), DeckRuntimeRejection> {
        self.acknowledge(&self.arm, &[module.get(), timer.get(), delay_us])
    }
    pub(super) fn retire_module(
        &self,
        module: DeckModuleInstanceId,
    ) -> Result<DeckModuleRetirementReceipt, DeckRuntimeRejection> {
        let input = self.runtime.numeric_vector(&[module.get()])?;
        let output = self
            .retire_module
            .call(&input)
            .into_result()?
            .into_collection(CollectionKind::Vector)?;
        if output.len() != 2 {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        let mut counts = NumericBuffer::<u64>::default();
        let mut timers = NumericBuffer::<u64>::default();
        output.element(0)?.as_ref().copy_numeric(&mut counts)?;
        output.element(1)?.as_ref().copy_numeric(&mut timers)?;
        let [created, retired, submitted, collected] = *counts.as_slice() else {
            return Err(DeckRuntimeRejection::InvalidProjection);
        };
        let [timers_retired] = *timers.as_slice() else {
            return Err(DeckRuntimeRejection::InvalidProjection);
        };
        if created != retired || submitted != collected {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        Ok(DeckModuleRetirementReceipt {
            module,
            timers_retired,
            mailbox: DeckMailboxMetrics {
                actors_created: created,
                actors_retired: retired,
                messages_submitted: submitted,
                results_collected: collected,
            },
        })
    }

    #[cfg(feature = "context-profiling")]
    pub(super) fn gc_snapshot(&self) -> Result<[f64; 3], SchemeVmError> {
        // SAFETY: the diagnostic belongs to this initialized linked graph.
        let export = unsafe {
            self.runtime
                .bind_object_export(marlin_deck_runtime_statistics)
        }?;
        let root = export.call(&self.control).into_result()?;
        let mut stats = NumericBuffer::<f64>::default();
        root.as_ref().copy_numeric(&mut stats)?;
        let values = stats.as_slice();
        if values.len() != 20 {
            return Err(SchemeVmError::Status {
                operation: "Deck GC snapshot",
                code: -1,
            });
        }
        Ok([values[6], values[5], values[17]])
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

#[cfg(any(debug_assertions, feature = "context-profiling"))]
fn emit_scheme_diagnostic(runtime: &GerbilRuntime, input: &RootedSchemeObject<'_>) {
    unsafe extern "C" fn take_error(_: i64) -> i64 {
        // SAFETY: consume one SDK-owned diagnostic root on its VM owner.
        unsafe { gerbil_scheme_rust_take_call_error_raw() }
    }
    // SAFETY: the diagnostic getter belongs to the initialized SDK graph.
    if let Ok(export) = unsafe { runtime.bind_object_export(take_error) }
        && let Ok(root) = export.call(input).into_result()
        && let Ok(bytes) = root.into_bytevector()
        && let Ok(bytes) = bytes.to_vec().into_result()
    {
        eprintln!("SCHEME-DIAGNOSTIC {}", String::from_utf8_lossy(&bytes));
    }
}

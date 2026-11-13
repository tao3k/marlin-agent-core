// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! One OS thread owns the VM; Tokio workers never enter Scheme.

use super::vm::{Completion, DeckVm, MessageId};
use super::{
    DeckMailboxMetrics, DeckRuntimeConfig, DeckRuntimeMetrics, DeckRuntimeRejection,
    DeckRuntimeShutdownReceipt, DeckRuntimeStartupError, DeckRuntimeStopReason, Job, Outcome,
    OwnerMessage, SchemeVmError,
};
use crate::DeckRuntimeMetadata;
use gerbil_scheme::{GerbilRuntime, LinkedGerbilProgram};
use marlin_agent_context::CONTEXT_BATCH_MAX_TEXT_BYTES;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{mpsc, watch};

pub(super) fn run(
    config: DeckRuntimeConfig,
    mut jobs: mpsc::Receiver<OwnerMessage>,
    stopping: Arc<AtomicBool>,
    wake: Arc<super::wake::Wake>,
    ready: std::sync::mpsc::SyncSender<Result<DeckRuntimeMetadata, DeckRuntimeStartupError>>,
    finished: watch::Sender<Option<DeckRuntimeShutdownReceipt>>,
) {
    let mut metrics = DeckRuntimeMetrics::default();
    let mut observed_processors = 0;
    let mut pending = VecDeque::new();
    let mut active = HashMap::new();
    let mut control = Controls::new();
    #[cfg(feature = "context-profiling")]
    let mut trace = Vec::new();
    #[cfg(feature = "context-profiling")]
    let mut trace_dropped = 0;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: this owner is the only entry point into the linked program.
        let program =
            unsafe { LinkedGerbilProgram::from_linker(super::vm::___LNK_marlin__deck__actor) };
        let runtime = match GerbilRuntime::initialize_program_single_processor(program) {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = ready.send(Err(error.into()));
                return None;
            }
        };
        let processors = match runtime.active_processors() {
            Ok(processors @ 1) => processors,
            Ok(_) => {
                let _ = ready.send(Err(SchemeVmError::Status {
                    operation: "Deck single processor",
                    code: -1,
                }
                .into()));
                return None;
            }
            Err(error) => {
                let _ = ready.send(Err(error.into()));
                return None;
            }
        };
        observed_processors = processors;
        let (mut vm, metadata) = match DeckVm::open(&runtime, wake.descriptors()) {
            Ok(vm) => vm,
            Err(error) => {
                let _ = ready.send(Err(error));
                return None;
            }
        };
        if ready.send(Ok(metadata)).is_err() {
            return None;
        }
        let mut protocol_failure = None;
        loop {
            if stopping.load(Ordering::Acquire) {
                jobs.close();
            }
            let mut control_turn = 0;
            while control_turn < usize::from(config.queue_capacity.get())
                && pending.len() < usize::from(config.batch_capacity.get())
                && active.len() + pending.len() < usize::from(config.queue_capacity.get())
            {
                match jobs.try_recv() {
                    Ok(message) => {
                        control_turn += 1;
                        if let Err(error) = receive(
                            message,
                            &mut pending,
                            &mut metrics,
                            &mut jobs,
                            &vm,
                            &mut control,
                        ) {
                            protocol_failure = Some(error);
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            if protocol_failure.is_some() {
                break;
            }
            #[cfg(feature = "context-profiling")]
            let turn_begin = std::time::Instant::now();
            #[cfg(feature = "context-profiling")]
            let gc_before = vm.gc_snapshot().expect("diagnostic snapshot");
            #[cfg(feature = "context-profiling")]
            {
                vm.profile = super::profile::VmStages::default();
            }
            #[cfg(feature = "context-profiling")]
            let mut completed_ages = Vec::new();
            metrics.largest_request_batch = metrics.largest_request_batch.max(pending.len());
            let batch = prepare(&mut pending, &mut metrics);
            #[cfg(feature = "context-profiling")]
            let submitted_count = batch.len();
            if !batch.is_empty() {
                let requests: Vec<_> = batch.iter().map(|job| (job.module, &job.request)).collect();
                match vm.submit(&requests) {
                    Ok(ids) if valid_ids(&ids, &active) => {
                        active.extend(ids.into_iter().zip(batch));
                    }
                    result => {
                        let failure = result
                            .err()
                            .unwrap_or(DeckRuntimeRejection::InvalidProjection);
                        for job in batch {
                            finish(job, Err(failure.clone()), &mut metrics);
                        }
                        protocol_failure = Some(failure);
                        break;
                    }
                }
            }
            if let Err(error) = control.retire_ready(&vm, &active, &pending) {
                protocol_failure = Some(error);
                break;
            }
            if jobs.is_closed() && jobs.is_empty() && active.is_empty() && pending.is_empty() {
                break;
            }
            {
                let completed = vm.receive(config.batch_capacity);
                #[cfg(feature = "context-profiling")]
                if let Ok(replies) = &completed {
                    completed_ages = replies
                        .completions
                        .iter()
                        .filter_map(|reply| {
                            active
                                .get(&reply.id)
                                .map(|job| (reply.id.0, job.enqueued_at.elapsed().as_micros()))
                        })
                        .collect();
                }
                let result = completed.and_then(|poll| {
                    control.validate_timers(&poll.timers)?;
                    retire(&mut active, poll.completions, &mut metrics)?;
                    control.publish_timers(poll.timers);
                    Ok(())
                });
                if let Err(failure) = result {
                    protocol_failure = Some(failure);
                    break;
                }
            }
            #[cfg(feature = "context-profiling")]
            {
                let gc_after = vm.gc_snapshot().expect("diagnostic snapshot");
                if trace.len() < 65536 {
                    trace.push(super::profile::Turn {
                        elapsed_us: turn_begin.elapsed().as_micros(),
                        submitted: submitted_count,
                        completed: completed_ages,
                        gc_count: gc_after[0] - gc_before[0],
                        gc_wall_us: (gc_after[1] - gc_before[1]) * 1e6,
                        heap_live_bytes: gc_after[2],
                        stages: std::mem::take(&mut vm.profile),
                    });
                } else {
                    trace_dropped += 1;
                }
            }
        }
        for (_, job) in active.drain() {
            finish(
                job,
                Err(protocol_failure
                    .clone()
                    .unwrap_or(DeckRuntimeRejection::Unavailable)),
                &mut metrics,
            );
        }
        for job in pending.drain(..) {
            finish(
                job,
                Err(protocol_failure
                    .clone()
                    .unwrap_or(DeckRuntimeRejection::Unavailable)),
                &mut metrics,
            );
        }
        let retirement = vm.close();
        let context_service_closed = retirement.is_ok();
        let mailbox = retirement.unwrap_or_default();
        drop(vm);
        drop(runtime);
        Some(DeckRuntimeShutdownReceipt {
            reason: if protocol_failure.is_some() {
                DeckRuntimeStopReason::ProtocolFailed
            } else if context_service_closed {
                DeckRuntimeStopReason::Drained
            } else {
                DeckRuntimeStopReason::SchemeCloseFailed
            },
            vm_processors: processors,
            context_service_closed,
            mailbox,
            metrics,
        })
    }));
    let receipt = match result {
        Ok(receipt) => receipt,
        Err(_) => {
            for (_, job) in active.drain() {
                finish(job, Err(DeckRuntimeRejection::Unavailable), &mut metrics);
            }
            for job in pending.drain(..) {
                finish(job, Err(DeckRuntimeRejection::Unavailable), &mut metrics);
            }
            Some(DeckRuntimeShutdownReceipt {
                reason: DeckRuntimeStopReason::OwnerPanicked,
                vm_processors: observed_processors,
                context_service_closed: false,
                mailbox: DeckMailboxMetrics::default(),
                metrics,
            })
        }
    };
    wake.close();
    for (_, slot) in control.timers.drain() {
        let _ = slot.reply.send(Err(DeckRuntimeRejection::Unavailable));
    }
    #[cfg(feature = "context-profiling")]
    eprintln!("CONTEXT-TRACE truncated_turns={trace_dropped}");
    #[cfg(feature = "context-profiling")]
    for turn in trace {
        turn.emit();
    }
    if let Some(receipt) = receipt {
        let _ = finished.send(Some(receipt));
    }
}

struct TimerSlot {
    module: super::DeckModuleInstanceId,
    reply: tokio::sync::oneshot::Sender<Result<super::DeckTimerReceipt, DeckRuntimeRejection>>,
}
struct Controls {
    next_module: u64,
    next_timer: u64,
    modules: HashSet<super::DeckModuleInstanceId>,
    timers: HashMap<super::DeckTimerId, TimerSlot>,
    retiring: Vec<Arc<super::ModuleState>>,
}
impl Controls {
    fn new() -> Self {
        Self {
            next_module: 2,
            next_timer: 1,
            modules: HashSet::from([super::DeckModuleInstanceId(std::num::NonZeroU64::MIN)]),
            timers: HashMap::new(),
            retiring: Vec::new(),
        }
    }
    fn retire_ready(
        &mut self,
        vm: &DeckVm<'_>,
        active: &HashMap<MessageId, Job>,
        pending: &VecDeque<Job>,
    ) -> Result<(), DeckRuntimeRejection> {
        let mut index = 0;
        while index < self.retiring.len() {
            let module = self.retiring[index].id;
            if active
                .values()
                .chain(pending.iter())
                .any(|job| job.module == module)
            {
                index += 1;
                continue;
            }
            let receipt = vm.retire_module(module)?;
            let timer_count = self
                .timers
                .values()
                .filter(|slot| slot.module == module)
                .count();
            if receipt.timers_retired != timer_count as u64 {
                return Err(DeckRuntimeRejection::InvalidProjection);
            }
            let timers: Vec<_> = self
                .timers
                .iter()
                .filter_map(|(id, slot)| (slot.module == module).then_some(*id))
                .collect();
            for timer in timers {
                let slot = self.timers.remove(&timer).expect("owned Module timer");
                let _ = slot.reply.send(Err(DeckRuntimeRejection::ModuleRetired));
            }
            // Native clocks/Actors have joined before the receipt becomes observable.
            self.modules.remove(&module);
            self.retiring
                .swap_remove(index)
                .finished
                .send_replace(Some(receipt));
        }
        Ok(())
    }
    fn validate_timers(
        &self,
        receipts: &[super::DeckTimerReceipt],
    ) -> Result<(), DeckRuntimeRejection> {
        let unique: HashSet<_> = receipts.iter().map(|receipt| receipt.timer).collect();
        if unique.len() != receipts.len()
            || receipts.iter().any(|receipt| {
                self.timers
                    .get(&receipt.timer)
                    .is_none_or(|slot| slot.module != receipt.module)
            })
        {
            return Err(DeckRuntimeRejection::InvalidProjection);
        }
        Ok(())
    }
    fn publish_timers(&mut self, receipts: Vec<super::DeckTimerReceipt>) {
        for receipt in receipts {
            let slot = self
                .timers
                .remove(&receipt.timer)
                .expect("validated timer ownership");
            let _ = slot.reply.send(Ok(receipt));
        }
    }
}
fn receive(
    message: OwnerMessage,
    pending: &mut VecDeque<Job>,
    metrics: &mut DeckRuntimeMetrics,
    jobs: &mut mpsc::Receiver<OwnerMessage>,
    vm: &DeckVm<'_>,
    control: &mut Controls,
) -> Result<(), DeckRuntimeRejection> {
    match message {
        OwnerMessage::Admit(job) => {
            metrics.requests_received += 1;
            if !control.modules.contains(&job.module) {
                finish(job, Err(DeckRuntimeRejection::InvalidProjection), metrics);
                return Err(DeckRuntimeRejection::InvalidProjection);
            }
            pending.push_back(job);
        }
        OwnerMessage::Stop => jobs.close(),
        OwnerMessage::Activate(reply) => {
            if reply.is_closed() {
                return Ok(());
            }
            let id = super::DeckModuleInstanceId(
                std::num::NonZeroU64::new(control.next_module)
                    .ok_or(DeckRuntimeRejection::InvalidProjection)?,
            );
            control.next_module = control
                .next_module
                .checked_add(1)
                .ok_or(DeckRuntimeRejection::InvalidProjection)?;
            vm.activate(id)?;
            if reply.send(Ok(super::ModuleState::new(id))).is_err() {
                vm.retire_module(id)?;
            } else {
                control.modules.insert(id);
            }
        }
        OwnerMessage::Retire(state) => {
            if !control.modules.contains(&state.id)
                || control.retiring.iter().any(|old| old.id == state.id)
            {
                return Err(DeckRuntimeRejection::InvalidProjection);
            }
            control.retiring.push(state);
        }
        OwnerMessage::Timer {
            module,
            delay_us,
            reply,
        } => {
            if !control.modules.contains(&module) {
                let _ = reply.send(Err(DeckRuntimeRejection::ModuleRetired));
                return Ok(());
            }
            let timer = super::DeckTimerId(
                std::num::NonZeroU64::new(control.next_timer)
                    .ok_or(DeckRuntimeRejection::InvalidProjection)?,
            );
            control.next_timer = control
                .next_timer
                .checked_add(1)
                .ok_or(DeckRuntimeRejection::InvalidProjection)?;
            vm.arm(module, timer, delay_us)?;
            control.timers.insert(timer, TimerSlot { module, reply });
        }
    }
    Ok(())
}

/// Prepare a text-bounded FIFO prefix and retire cancellation before VM entry.
pub(super) fn prepare(pending: &mut VecDeque<Job>, metrics: &mut DeckRuntimeMetrics) -> Vec<Job> {
    let mut batch = Vec::<Job>::new();
    let mut bytes = 0;
    while let Some(job) = pending.front() {
        if let Err(error) = job.check_active() {
            finish(pending.pop_front().unwrap(), Err(error), metrics);
            continue;
        }
        // Match DeckVm::submit: only consecutive requests for the same Module
        // and the same immutable Scope allocation share one wire envelope.
        let additional = if batch.last().is_some_and(|previous| {
            previous.module == job.module
                && Arc::ptr_eq(previous.request.scope(), job.request.scope())
        }) {
            job.request.turn().text_bytes()
        } else {
            job.request.text_bytes()
        };
        if bytes + additional > CONTEXT_BATCH_MAX_TEXT_BYTES {
            break;
        }
        bytes += additional;
        batch.push(pending.pop_front().unwrap());
    }
    if !batch.is_empty() {
        metrics.scheme_batches += 1;
        metrics.scheme_items += batch.len() as u64;
        metrics.largest_scheme_batch = metrics.largest_scheme_batch.max(batch.len());
    }
    batch
}

fn valid_ids(ids: &[MessageId], active: &HashMap<MessageId, Job>) -> bool {
    let unique: HashSet<_> = ids.iter().collect();
    unique.len() == ids.len() && ids.iter().all(|id| id.0 != 0 && !active.contains_key(id))
}

/// Validate the complete reply window before publishing any client outcome.
pub(super) fn retire(
    active: &mut HashMap<MessageId, Job>,
    replies: Vec<Completion>,
    metrics: &mut DeckRuntimeMetrics,
) -> Result<(), DeckRuntimeRejection> {
    let unique: HashSet<_> = replies.iter().map(|reply| reply.id).collect();
    if unique.len() != replies.len() || replies.iter().any(|reply| !active.contains_key(&reply.id))
    {
        return Err(DeckRuntimeRejection::InvalidProjection);
    }
    for reply in replies {
        let job = active.remove(&reply.id).expect("validated reply identity");
        finish(job, reply.outcome, metrics);
    }
    Ok(())
}

fn finish(job: Job, result: Outcome, metrics: &mut DeckRuntimeMetrics) {
    let result = job.check_active().and(result);
    match &result {
        Ok(_) => metrics.requests_succeeded += 1,
        Err(DeckRuntimeRejection::Cancelled) => metrics.requests_cancelled += 1,
        Err(DeckRuntimeRejection::Deadline) => metrics.requests_expired += 1,
        Err(_) => metrics.requests_failed += 1,
    }
    let _ = job.reply.send(result);
}

#[cfg(test)]
#[path = "../../tests/unit/module_events.rs"]
mod tests;

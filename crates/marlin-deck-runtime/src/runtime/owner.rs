//! One OS thread owns the VM; Tokio workers never enter Scheme.

use super::vm::{Completion, DeckVm, MessageId};
use super::{
    DeckMailboxMetrics, DeckRuntimeConfig, DeckRuntimeMetrics, DeckRuntimeRejection,
    DeckRuntimeShutdownReceipt, DeckRuntimeStopReason, Job, Outcome, OwnerMessage, SchemeVmError,
};
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
    ready: std::sync::mpsc::SyncSender<Result<(), SchemeVmError>>,
    finished: watch::Sender<Option<DeckRuntimeShutdownReceipt>>,
) {
    let mut metrics = DeckRuntimeMetrics::default();
    let mut observed_processors = 0;
    let mut pending = VecDeque::new();
    let mut active = HashMap::new();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: this owner is the only entry point into the linked program.
        let program =
            unsafe { LinkedGerbilProgram::from_linker(super::vm::___LNK_marlin__deck__actor) };
        let runtime = match GerbilRuntime::initialize_program_single_processor(program) {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = ready.send(Err(error));
                return None;
            }
        };
        let processors = match runtime.active_processors() {
            Ok(processors @ 1) => processors,
            Ok(_) => {
                let _ = ready.send(Err(SchemeVmError::Status {
                    operation: "Deck single processor",
                    code: -1,
                }));
                return None;
            }
            Err(error) => {
                let _ = ready.send(Err(error));
                return None;
            }
        };
        observed_processors = processors;
        let mut vm = match DeckVm::open(&runtime) {
            Ok(vm) => vm,
            Err(error) => {
                let _ = ready.send(Err(error));
                return None;
            }
        };
        if ready.send(Ok(())).is_err() {
            return None;
        }
        let mut protocol_failure = None;
        loop {
            if stopping.load(Ordering::Acquire) {
                jobs.close();
            }
            if pending.is_empty() && active.is_empty() {
                match jobs.blocking_recv() {
                    Some(message) => receive(message, &mut pending, &mut metrics, &mut jobs),
                    None => break,
                }
            }
            while pending.len() < usize::from(config.batch_capacity.get())
                && active.len() + pending.len() < usize::from(config.queue_capacity.get())
            {
                match jobs.try_recv() {
                    Ok(message) => receive(message, &mut pending, &mut metrics, &mut jobs),
                    Err(_) => break,
                }
            }
            metrics.largest_request_batch = metrics.largest_request_batch.max(pending.len());
            let batch = prepare(&mut pending, &mut metrics);
            if !batch.is_empty() {
                let requests: Vec<_> = batch.iter().map(|job| &job.request).collect();
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
            if !active.is_empty() {
                let completed = vm.receive(config.batch_capacity);
                let result =
                    completed.and_then(|replies| retire(&mut active, replies, &mut metrics));
                if let Err(failure) = result {
                    protocol_failure = Some(failure);
                    break;
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
    if let Some(receipt) = receipt {
        let _ = finished.send(Some(receipt));
    }
}

fn receive(
    message: OwnerMessage,
    pending: &mut VecDeque<Job>,
    metrics: &mut DeckRuntimeMetrics,
    jobs: &mut mpsc::Receiver<OwnerMessage>,
) {
    match message {
        OwnerMessage::Admit(job) => {
            metrics.requests_received += 1;
            pending.push_back(job);
        }
        OwnerMessage::Stop => jobs.close(),
    }
}

/// Prepare a text-bounded FIFO prefix and retire cancellation before VM entry.
pub(super) fn prepare(pending: &mut VecDeque<Job>, metrics: &mut DeckRuntimeMetrics) -> Vec<Job> {
    let mut batch = Vec::new();
    let mut bytes = 0;
    while let Some(job) = pending.front() {
        if let Err(error) = job.check_active() {
            finish(pending.pop_front().unwrap(), Err(error), metrics);
            continue;
        }
        if bytes + job.request.text_bytes() > CONTEXT_BATCH_MAX_TEXT_BYTES {
            break;
        }
        bytes += job.request.text_bytes();
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

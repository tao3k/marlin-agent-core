//! One OS thread owns the VM; Tokio workers never enter Scheme.

use super::vm::DeckVm;
use super::{
    DeckRuntimeConfig, DeckRuntimeMetrics, DeckRuntimeRejection, DeckRuntimeShutdownReceipt,
    DeckRuntimeStopReason, Job, Outcome, OwnerMessage, SchemeVmError,
};
use gerbil_scheme::{GerbilRuntime, LinkedGerbilProgram};
use marlin_agent_context::CONTEXT_BATCH_MAX_TEXT_BYTES;
use std::{
    collections::VecDeque,
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
        let mut vm = match DeckVm::open(&runtime, config.actors) {
            Ok(vm) => vm,
            Err(error) => {
                let _ = ready.send(Err(error));
                return None;
            }
        };
        if ready.send(Ok(())).is_err() {
            return None;
        }
        let mut pending = VecDeque::new();
        loop {
            if stopping.load(Ordering::Acquire) {
                jobs.close();
            }
            if pending.is_empty() {
                match jobs.blocking_recv() {
                    Some(message) => receive(message, &mut pending, &mut metrics, &mut jobs),
                    None => break,
                }
            }
            while pending.len() < usize::from(config.batch_capacity.get()) {
                match jobs.try_recv() {
                    Ok(message) => receive(message, &mut pending, &mut metrics, &mut jobs),
                    Err(_) => break,
                }
            }
            metrics.largest_request_batch = metrics.largest_request_batch.max(pending.len());
            turn(&mut pending, &mut metrics, |requests| {
                vm.evaluate(requests, config.actor_quantum)
            });
        }
        let actors_closed = vm.close().is_ok();
        drop(vm);
        drop(runtime);
        Some(DeckRuntimeShutdownReceipt {
            reason: if actors_closed {
                DeckRuntimeStopReason::Drained
            } else {
                DeckRuntimeStopReason::SchemeCloseFailed
            },
            vm_processors: processors,
            actors_closed,
            metrics,
        })
    }));
    let receipt = match result {
        Ok(receipt) => receipt,
        Err(_) => Some(DeckRuntimeShutdownReceipt {
            reason: DeckRuntimeStopReason::OwnerPanicked,
            vm_processors: observed_processors,
            actors_closed: false,
            metrics,
        }),
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

/// Retire a bounded FIFO prefix, then let the owner refill before the next VM turn.
pub(super) fn turn(
    pending: &mut VecDeque<Job>,
    metrics: &mut DeckRuntimeMetrics,
    evaluate: impl FnOnce(
        &[&marlin_agent_context::ContextAdmissionRequest],
    )
        -> Result<Vec<marlin_agent_context::ContextScopeDigest>, DeckRuntimeRejection>,
) {
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
    if batch.is_empty() {
        return;
    }
    metrics.scheme_batches += 1;
    metrics.scheme_items += batch.len() as u64;
    metrics.largest_scheme_batch = metrics.largest_scheme_batch.max(batch.len());
    let requests: Vec<_> = batch.iter().map(|job| &job.request).collect();
    let result = evaluate(&requests).and_then(|digests| {
        if digests.len() == batch.len() {
            Ok(digests)
        } else {
            Err(DeckRuntimeRejection::InvalidProjection)
        }
    });
    match result {
        Ok(digests) => {
            for (job, digest) in batch.into_iter().zip(digests) {
                finish(job, Ok(digest), metrics);
            }
        }
        Err(error) => {
            for job in batch {
                finish(job, Err(error.clone()), metrics);
            }
        }
    }
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

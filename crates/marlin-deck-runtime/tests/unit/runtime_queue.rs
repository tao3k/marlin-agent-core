// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::*;
use std::collections::{HashMap, VecDeque};
use vm::{Completion, MessageId};
#[path = "context_fixture.rs"]
mod context_fixture;

fn job(index: usize) -> (Job, oneshot::Receiver<Outcome>) {
    let (reply, result) = oneshot::channel();
    (
        Job {
            module: super::DeckModuleInstanceId(std::num::NonZeroU64::MIN),
            #[cfg(feature = "context-profiling")]
            enqueued_at: std::time::Instant::now(),
            request: context_fixture::request(&context_fixture::scope("source"), index),
            cancellation: CancellationToken::new(),
            deadline: Instant::now() + Duration::from_secs(30),
            reply,
        },
        result,
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelled_and_expired_work_never_enters_scheme() {
    let (cancelled, mut cancelled_result) = job(0);
    cancelled.cancellation.cancel();
    let (mut expired, mut expired_result) = job(1);
    expired.deadline = Instant::now();
    let mut pending = VecDeque::from([cancelled, expired]);
    let mut metrics = DeckRuntimeMetrics::default();
    assert!(owner::prepare(&mut pending, &mut metrics).is_empty());
    assert!(matches!(
        cancelled_result.try_recv().unwrap(),
        Err(DeckRuntimeRejection::Cancelled)
    ));
    assert!(matches!(
        expired_result.try_recv().unwrap(),
        Err(DeckRuntimeRejection::Deadline)
    ));
    assert_eq!(metrics.requests_cancelled, 1);
    assert_eq!(metrics.requests_expired, 1);
    assert_eq!(metrics.scheme_batches, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancellation_during_vm_turn_overrides_a_failed_result() {
    let (job, mut result) = job(0);
    let cancellation = job.cancellation.clone();
    let mut pending = VecDeque::from([job]);
    let mut metrics = DeckRuntimeMetrics::default();
    let batch = owner::prepare(&mut pending, &mut metrics);
    let mut active = HashMap::from([(MessageId(1), batch.into_iter().next().unwrap())]);
    cancellation.cancel();
    owner::retire(
        &mut active,
        vec![Completion {
            id: MessageId(1),
            outcome: Err(DeckRuntimeRejection::InvalidProjection),
        }],
        &mut metrics,
    )
    .unwrap();
    assert!(matches!(
        result.try_recv().unwrap(),
        Err(DeckRuntimeRejection::Cancelled)
    ));
    assert_eq!(metrics.requests_cancelled, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn duplicate_replies_are_rejected_before_any_result_is_published() {
    let (first, mut a) = job(0);
    let (second, mut b) = job(1);
    let mut active = HashMap::from([(MessageId(1), first), (MessageId(2), second)]);
    let mut metrics = DeckRuntimeMetrics::default();
    let replies = [1, 1]
        .into_iter()
        .map(|id| Completion {
            id: MessageId(id),
            outcome: Err(DeckRuntimeRejection::ContextActor),
        })
        .collect();
    assert!(matches!(
        owner::retire(&mut active, replies, &mut metrics),
        Err(DeckRuntimeRejection::InvalidProjection)
    ));
    assert!(a.try_recv().is_err());
    assert!(b.try_recv().is_err());
    assert_eq!(active.len(), 2);
    assert_eq!(metrics.requests_failed, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn completed_actor_retires_without_waiting_for_an_unfinished_actor() {
    let (first, mut a) = job(0);
    let (second, mut b) = job(1);
    let mut active = HashMap::from([(MessageId(1), first), (MessageId(2), second)]);
    let mut metrics = DeckRuntimeMetrics::default();
    owner::retire(
        &mut active,
        vec![Completion {
            id: MessageId(2),
            outcome: Err(DeckRuntimeRejection::ContextActor),
        }],
        &mut metrics,
    )
    .unwrap();
    assert!(a.try_recv().is_err());
    assert!(matches!(
        b.try_recv().unwrap(),
        Err(DeckRuntimeRejection::ContextActor)
    ));
    assert_eq!(active.len(), 1);
    assert_eq!(metrics.requests_failed, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_full_channel_rejects_without_waiting_or_entering_scheme() {
    let (queue, mut jobs) = mpsc::channel(1);
    let (occupied, _result) = job(0);
    queue
        .try_send(OwnerMessage::Admit(occupied))
        .unwrap_or_else(|_| panic!("empty queue"));
    let (_finished, stopped) = watch::channel(None);
    let deck = DeckRuntime {
        metadata: Arc::new(crate::DeckRuntimeMetadata {
            module_id: crate::DeckRuntimeModuleId("queue-fixture".into()),
            version: 0,
        }),
        queue,
        wake: Arc::new(wake::Wake::new().unwrap()),
        default_module: ModuleState::new(DeckModuleInstanceId(std::num::NonZeroU64::MIN)),
        stopping: Arc::new(AtomicBool::new(false)),
        stopped,
        owner: Arc::new(Mutex::new(None)),
    };
    let module = deck.context_module();
    assert!(matches!(
        module.timer(Duration::ZERO).await,
        Err(DeckRuntimeRejection::TimerDelay)
    ));
    assert!(matches!(
        module.retire().await,
        Err(DeckRuntimeRejection::Backpressure)
    ));
    assert!(!*module.state.closed.lock().unwrap());
    let result = deck
        .admit_context(
            context_fixture::request(&context_fixture::scope("source"), 1),
            Duration::from_secs(30),
            CancellationToken::new(),
        )
        .await;
    assert!(matches!(result, Err(DeckRuntimeRejection::Backpressure)));
    assert!(matches!(jobs.try_recv(), Ok(OwnerMessage::Admit(_))));
    assert!(jobs.try_recv().is_err());
}

fn text_limited_queue() -> (
    Arc<marlin_agent_context::ContextScope>,
    VecDeque<Job>,
    Vec<oneshot::Receiver<Outcome>>,
) {
    use marlin_agent_context::{
        ContextBundleId, ContextDestinationId, ContextOrganizationId, ContextSourceCutId,
        ContextTaskId,
    };
    let mut shared = (*context_fixture::scope("source")).clone();
    let text = "🦦".repeat(256);
    shared.bundle = ContextBundleId::new(&text);
    shared.organization = ContextOrganizationId::new(&text);
    shared.source_cut = ContextSourceCutId::new(&text);
    shared.project = marlin_agent_protocol::ProjectRuntimeProjectId::new(&text);
    shared.worktree = marlin_agent_protocol::ProjectRuntimeWorktreeId::new(&text);
    let shared = Arc::new(shared);
    let mut pending = VecDeque::new();
    let mut replies = Vec::new();
    for index in 0..16 {
        let (mut job, reply) = job(index);
        let mut turn = job.request.turn().clone();
        turn.task = ContextTaskId::new(&text);
        turn.destination = ContextDestinationId::new(&text);
        job.request = ContextAdmissionRequest::new(Arc::clone(&shared), turn).unwrap();
        pending.push_back(job);
        replies.push(reply);
    }
    (shared, pending, replies)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_text_limited_turn_leaves_a_fifo_remainder_for_the_next_turn() {
    use marlin_agent_context::{
        CONTEXT_BATCH_MAX_TEXT_BYTES, ContextAdmissionError, ContextScopeBatch,
    };
    let (shared, mut pending, _replies) = text_limited_queue();
    let mut metrics = DeckRuntimeMetrics::default();
    let batch = owner::prepare(&mut pending, &mut metrics);
    let first = batch.len();
    // Five 1024-byte shared fields and 2069 bytes per row fit 13 rows.
    // Counting shared fields per request incorrectly stops after four.
    assert_eq!(first, 13);
    let mut rows: Vec<_> = batch
        .iter()
        .map(|job| job.request.turn().as_row())
        .collect();
    let projected = ContextScopeBatch::new(shared.as_key(), &rows).unwrap();
    assert_eq!(projected.text_bytes(), 5120 + 2069 * first);
    assert!(projected.text_bytes() <= CONTEXT_BATCH_MAX_TEXT_BYTES);
    rows.push(pending.front().unwrap().request.turn().as_row());
    assert!(matches!(
        ContextScopeBatch::new(shared.as_key(), &rows),
        Err(ContextAdmissionError::TextBudget)
    ));
    assert_eq!(pending.len(), 16 - first);
    assert_eq!(
        pending
            .front()
            .unwrap()
            .request
            .turn()
            .session
            .as_ref()
            .unwrap()
            .turn
            .0,
        first as u64
    );
    assert_eq!(metrics.scheme_items, first as u64);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn text_budget_counts_each_scope_and_module_group_without_reordering() {
    for (layout, expected) in [(0, 4), (1, 4), (2, 8)] {
        let (shared, mut pending, _replies) = text_limited_queue();
        for (index, job) in pending.iter_mut().enumerate() {
            match layout {
                // Equal scope values in different Arc allocations are distinct wire groups.
                0 => {
                    job.request = ContextAdmissionRequest::new(
                        Arc::new((*shared).clone()),
                        job.request.turn().clone(),
                    )
                    .unwrap()
                }
                1 | 2 => {
                    let group = if layout == 1 { index } else { index / 3 };
                    job.module = DeckModuleInstanceId(
                        std::num::NonZeroU64::new(1 + (group % 2) as u64).unwrap(),
                    );
                }
                _ => unreachable!(),
            }
        }
        let mut metrics = DeckRuntimeMetrics::default();
        let batch = owner::prepare(&mut pending, &mut metrics);
        assert_eq!(batch.len(), expected, "layout {layout}");
        assert_eq!(pending.len(), 16 - expected);
        for (index, job) in batch.iter().chain(pending.iter()).enumerate() {
            assert_eq!(
                job.request.turn().session.as_ref().unwrap().turn.0,
                index as u64
            );
        }
        assert_eq!(metrics.scheme_items, expected as u64);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelled_rows_do_not_spend_text_budget_or_split_a_scope_group() {
    let (_, mut pending, mut replies) = text_limited_queue();
    pending[3].cancellation.cancel();
    let mut metrics = DeckRuntimeMetrics::default();
    let batch = owner::prepare(&mut pending, &mut metrics);
    assert_eq!(batch.len(), 13);
    assert_eq!(pending.len(), 2);
    assert_eq!(
        pending
            .front()
            .unwrap()
            .request
            .turn()
            .session
            .as_ref()
            .unwrap()
            .turn
            .0,
        14
    );
    assert!(matches!(
        replies[3].try_recv().unwrap(),
        Err(DeckRuntimeRejection::Cancelled)
    ));
    assert_eq!(metrics.requests_cancelled, 1);
    assert_eq!(metrics.scheme_items, 13);
}

#[test]
fn an_elapsed_budget_remains_a_deadline_when_its_caller_has_retired() {
    let (mut job, result) = job(0);
    drop(result);
    job.deadline = Instant::now();
    assert!(matches!(
        job.check_active(),
        Err(DeckRuntimeRejection::Deadline)
    ));
    job.cancellation.cancel();
    assert!(matches!(
        job.check_active(),
        Err(DeckRuntimeRejection::Cancelled)
    ));
}

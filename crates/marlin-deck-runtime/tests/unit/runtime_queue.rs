use super::*;
use std::collections::VecDeque;
#[path = "context_fixture.rs"]
mod context_fixture;

fn job(index: usize) -> (Job, oneshot::Receiver<Outcome>) {
    let (reply, result) = oneshot::channel();
    (
        Job {
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
    owner::turn(&mut pending, &mut metrics, |_| {
        panic!("retired work entered Scheme")
    });
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
    owner::turn(&mut pending, &mut metrics, |_| {
        cancellation.cancel();
        Err(DeckRuntimeRejection::InvalidProjection)
    });
    assert!(matches!(
        result.try_recv().unwrap(),
        Err(DeckRuntimeRejection::Cancelled)
    ));
    assert_eq!(metrics.requests_cancelled, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn malformed_batch_is_rejected_before_any_result_is_published() {
    let (first, mut a) = job(0);
    let (second, mut b) = job(1);
    let mut pending = VecDeque::from([first, second]);
    let mut metrics = DeckRuntimeMetrics::default();
    owner::turn(&mut pending, &mut metrics, |_| Ok(Vec::new()));
    for result in [&mut a, &mut b] {
        assert!(matches!(
            result.try_recv().unwrap(),
            Err(DeckRuntimeRejection::InvalidProjection)
        ));
    }
    assert_eq!(metrics.requests_failed, 2);
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
        queue,
        stopping: Arc::new(AtomicBool::new(false)),
        stopped,
        owner: Arc::new(Mutex::new(None)),
    };
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_text_limited_turn_leaves_a_fifo_remainder_for_the_next_turn() {
    use marlin_agent_context::{
        CONTEXT_BATCH_MAX_TEXT_BYTES, ContextBundleId, ContextDestinationId, ContextOrganizationId,
        ContextSourceCutId, ContextTaskId,
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
    let mut metrics = DeckRuntimeMetrics::default();
    let mut first = 0;
    owner::turn(&mut pending, &mut metrics, |requests| {
        assert!(
            requests
                .iter()
                .map(|request| request.text_bytes())
                .sum::<usize>()
                <= CONTEXT_BATCH_MAX_TEXT_BYTES
        );
        first = requests.len();
        Err(DeckRuntimeRejection::InvalidProjection)
    });
    assert!(first > 0 && first < 16);
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
    assert_eq!(metrics.requests_failed, first as u64);
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

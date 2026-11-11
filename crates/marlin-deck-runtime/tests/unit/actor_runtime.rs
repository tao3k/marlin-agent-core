#[path = "context_fixture.rs"]
mod context_fixture;
use context_fixture::{request, scope};
use marlin_deck_runtime::{
    DeckRuntime, DeckRuntimeConfig, DeckRuntimeRejection, DeckRuntimeStartupError,
    DeckRuntimeStopReason,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

// One process owns exactly one VM lifetime; the SDK forbids reinitialization.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn context_sessions_cross_real_single_vm_actors_and_retire() {
    let invalid =
        DeckRuntimeConfig::new(1.try_into().unwrap()).with_batch_capacity(65.try_into().unwrap());
    assert!(matches!(
        DeckRuntime::start(invalid),
        Err(DeckRuntimeStartupError::BatchCapacity)
    ));
    let config = DeckRuntimeConfig::new(128.try_into().unwrap());
    let deck = DeckRuntime::start(config).expect("open real linked Scheme VM");
    let scopes = [scope("source-a"), scope("source-b")];
    let budget = Duration::from_secs(60);
    let mut expected = Vec::new();
    for index in 0..32 {
        expected.push(
            deck.admit_context(
                request(&scopes[index % 2], index),
                budget,
                CancellationToken::new(),
            )
            .await
            .unwrap(),
        );
    }
    assert_ne!(expected[0], expected[1]);
    let first_digest = expected[0];
    let mut calls = tokio::task::JoinSet::new();
    for (index, expected) in expected.into_iter().enumerate() {
        let deck = deck.clone();
        let request = request(&scopes[index % 2], index);
        calls.spawn(async move {
            assert_eq!(
                deck.admit_context(request, budget, CancellationToken::new())
                    .await
                    .unwrap(),
                expected
            );
        });
    }
    while let Some(call) = calls.join_next().await {
        call.unwrap();
    }
    // Exercise u64 values beyond Scheme's fixnum range through the numeric ABI.
    let mut wide_scope = (*scopes[0]).clone();
    wide_scope.epoch = marlin_agent_context::ContextScopeEpoch(u64::MAX);
    let mut wide_turn = request(&scopes[0], 0).turn().clone();
    wide_turn.session.as_mut().unwrap().turn = marlin_agent_context::ContextTurnIndex(u64::MAX);
    let wide = marlin_agent_context::ContextAdmissionRequest::new(
        std::sync::Arc::new(wide_scope),
        wide_turn,
    )
    .unwrap();
    let digest = deck
        .admit_context(wide.clone(), budget, CancellationToken::new())
        .await
        .unwrap();
    assert_ne!(digest, first_digest);
    assert_eq!(
        deck.admit_context(wide, budget, CancellationToken::new())
            .await
            .unwrap(),
        digest
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(matches!(
        deck.admit_context(request(&scopes[0], 99), budget, cancelled)
            .await,
        Err(DeckRuntimeRejection::Cancelled)
    ));
    assert!(matches!(
        deck.admit_context(
            request(&scopes[0], 99),
            Duration::ZERO,
            CancellationToken::new()
        )
        .await,
        Err(DeckRuntimeRejection::Deadline)
    ));
    let receipt = deck.shutdown().await.unwrap();
    assert_eq!(receipt.reason, DeckRuntimeStopReason::Drained);
    assert_eq!(receipt.vm_processors, 1);
    assert!(receipt.context_service_closed);
    assert_eq!(receipt.metrics.requests_received, 66);
    assert_eq!(receipt.metrics.requests_succeeded, 66);
    assert_eq!(receipt.metrics.scheme_items, 66);
    assert_eq!(receipt.metrics.requests_failed, 0);
    assert_eq!(receipt.mailbox.actors_created, 1);
    assert_eq!(receipt.mailbox.actors_retired, 1);
    assert_eq!(receipt.mailbox.messages_submitted, 66);
    assert_eq!(receipt.mailbox.results_collected, 66);
    assert_eq!(deck.shutdown().await.unwrap(), receipt);
    assert!(matches!(
        deck.admit_context(request(&scopes[0], 99), budget, CancellationToken::new())
            .await,
        Err(DeckRuntimeRejection::Unavailable)
    ));
    assert!(matches!(
        DeckRuntime::start(config),
        Err(DeckRuntimeStartupError::Scheme(_))
    ));
}

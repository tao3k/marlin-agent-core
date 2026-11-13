// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

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
    assert_eq!(deck.metadata().module_id.as_str(), "deck-runtime");
    assert!(deck.metadata().version > 0);
    assert_eq!(deck.clone().metadata(), deck.metadata());
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
    // Independent instances use the same library and VM, with separate ownership.
    let left = deck.activate_context_module().await.unwrap();
    let right = deck.activate_context_module().await.unwrap();
    assert_ne!(left.id(), right.id());
    let before = std::time::Instant::now();
    let timer = tokio::time::timeout(
        Duration::from_secs(2),
        right.timer(Duration::from_millis(5)),
    )
    .await
    .expect("idle Scheme timer did not wake Rust")
    .unwrap();
    assert_eq!(timer.module, right.id());
    assert!(timer.observed_monotonic_us >= timer.due_monotonic_us);
    assert!(before.elapsed() >= Duration::from_millis(5));
    let mut long_timer = Box::pin(left.timer(Duration::from_secs(3600)));
    std::future::poll_fn(|cx| {
        assert!(std::future::Future::poll(long_timer.as_mut(), cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    left.admit_context(request(&scopes[0], 0), budget, CancellationToken::new())
        .await
        .unwrap();
    let retired = left.retire().await.unwrap();
    assert_eq!(retired.module, left.id());
    assert_eq!(retired.mailbox.actors_created, 1);
    assert_eq!(retired.mailbox.actors_retired, 1);
    assert_eq!(retired.timers_retired, 1);
    assert_eq!(left.retire().await.unwrap(), retired);
    assert!(matches!(
        long_timer.await,
        Err(DeckRuntimeRejection::ModuleRetired)
    ));
    assert!(matches!(
        left.timer(Duration::from_millis(1)).await,
        Err(DeckRuntimeRejection::ModuleRetired)
    ));
    assert!(matches!(
        left.admit_context(request(&scopes[0], 0), budget, CancellationToken::new())
            .await,
        Err(DeckRuntimeRejection::ModuleRetired)
    ));
    right
        .admit_context(request(&scopes[0], 0), budget, CancellationToken::new())
        .await
        .unwrap();
    right
        .admit_context(request(&scopes[0], 1), budget, CancellationToken::new())
        .await
        .unwrap();
    let mut abandoned_timer = Box::pin(right.timer(Duration::from_secs(3600)));
    std::future::poll_fn(|cx| {
        assert!(std::future::Future::poll(abandoned_timer.as_mut(), cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    drop(abandoned_timer);
    assert_eq!(right.retire().await.unwrap().timers_retired, 1);

    let racing = deck.activate_context_module().await.unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(65));
    let mut contenders = tokio::task::JoinSet::new();
    for index in 0..64 {
        let module = racing.clone();
        let barrier = barrier.clone();
        let request = request(&scopes[0], index);
        contenders.spawn(async move {
            barrier.wait().await;
            module
                .admit_context(request, budget, CancellationToken::new())
                .await
        });
    }
    barrier.wait().await;
    let race_retirement = racing.retire().await.unwrap();
    let mut race_successes = 0;
    while let Some(result) = contenders.join_next().await {
        match result.unwrap() {
            Ok(_) => race_successes += 1,
            Err(DeckRuntimeRejection::ModuleRetired) => (),
            Err(error) => panic!("unexpected admission/retirement race result: {error}"),
        }
    }
    assert_eq!(race_retirement.mailbox.messages_submitted, race_successes);
    assert_eq!(race_retirement.mailbox.results_collected, race_successes);
    let total = 69 + race_successes;
    let receipt = deck.shutdown().await.unwrap();
    assert_eq!(receipt.reason, DeckRuntimeStopReason::Drained);
    assert_eq!(receipt.vm_processors, 1);
    assert!(receipt.context_service_closed);
    assert_eq!(receipt.metrics.requests_received, total);
    assert_eq!(receipt.metrics.requests_succeeded, total);
    assert_eq!(receipt.metrics.scheme_items, total);
    assert_eq!(receipt.metrics.requests_failed, 0);
    assert_eq!(
        receipt.mailbox.actors_created,
        3 + race_retirement.mailbox.actors_created
    );
    assert_eq!(
        receipt.mailbox.actors_retired,
        receipt.mailbox.actors_created
    );
    assert_eq!(receipt.mailbox.messages_submitted, total);
    assert_eq!(receipt.mailbox.results_collected, total);
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

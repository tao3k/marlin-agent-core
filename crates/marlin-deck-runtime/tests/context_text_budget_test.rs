// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "actor-runtime")]

#[path = "unit/context_fixture.rs"]
mod context_fixture;

use marlin_agent_context::{
    ContextAdmissionRequest, ContextBundleId, ContextDestinationId, ContextOrganizationId,
    ContextSourceCutId, ContextTaskId,
};
use marlin_agent_protocol::{ProjectRuntimeProjectId, ProjectRuntimeWorktreeId};
use marlin_deck_runtime::{DeckRuntime, DeckRuntimeConfig, DeckRuntimeStopReason};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

// A separate process preserves the SDK's single VM lifetime admission.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn long_shared_scope_preserves_every_typed_receipt_and_retires() {
    let text = "🦦".repeat(256);
    let mut scope = (*context_fixture::scope("long-source")).clone();
    scope.bundle = ContextBundleId::new(&text);
    scope.organization = ContextOrganizationId::new(&text);
    scope.project = ProjectRuntimeProjectId::new(&text);
    scope.worktree = ProjectRuntimeWorktreeId::new(&text);
    scope.source_cut = ContextSourceCutId::new(&text);
    let scope = Arc::new(scope);
    let deck = DeckRuntime::start(DeckRuntimeConfig::new(128.try_into().unwrap()))
        .expect("open the actual linked Scheme VM");
    let mut calls = tokio::task::JoinSet::new();
    let mut expected = Vec::new();
    for index in 0..16 {
        // Frozen fixture bytes, independent of the production Scheme encoder.
        let tuple = format!(
            "(ai-agentic-context/scope-v1 \"{text}\" \"{text}\" 1 \"{text}\" \"{text}\" \"{text}\" \"assistant\" \"{text}\" \"{text}\" \"conversation\" {index})"
        );
        expected.push(<[u8; 32]>::from(Sha256::digest(tuple.as_bytes())));
        let mut turn = context_fixture::request(&scope, index).turn().clone();
        turn.task = ContextTaskId::new(&text);
        turn.destination = ContextDestinationId::new(&text);
        let request = ContextAdmissionRequest::new(Arc::clone(&scope), turn).unwrap();
        assert_eq!(request.text_bytes(), 5120 + 2069);
        let deck = deck.clone();
        calls.spawn(async move {
            let digest = deck
                .admit_context(request, Duration::from_secs(60), CancellationToken::new())
                .await
                .expect("long Context row admitted");
            (index, *digest.as_bytes())
        });
    }
    let mut seen = HashSet::new();
    while let Some(call) = calls.join_next().await {
        let (index, digest) = call.unwrap();
        assert!(seen.insert(index), "duplicate Context receipt");
        assert_eq!(digest, expected[index], "wrong digest for Context {index}");
    }
    assert_eq!(seen.len(), 16);
    let receipt = deck.shutdown().await.unwrap();
    assert_eq!(receipt.reason, DeckRuntimeStopReason::Drained);
    assert!(receipt.context_service_closed);
    assert_eq!(receipt.metrics.requests_succeeded, 16);
    assert_eq!(receipt.metrics.requests_failed, 0);
    assert_eq!(receipt.mailbox.messages_submitted, 16);
    assert_eq!(receipt.mailbox.results_collected, 16);
    assert_eq!(receipt.mailbox.actors_created, 1);
    assert_eq!(receipt.mailbox.actors_retired, 1);
}

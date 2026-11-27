// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Adversarial stability tests refining upstream `PriorityIsolatedBoundedMailbox` (SM11)
//! and downstream `dev/quint/stability/MarlinSearchDslMailboxOverlay.qnt` against
//! `LinkedSearchDslMailbox` in `marlin-agent-tui`.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use marlin_agent_search_engine::{WorkspaceIndexConfig, WorkspaceSearchIndex};
use marlin_agent_tooling::SearchQueryTier;

fn empty_shared_index() -> (
    tempfile::TempDir,
    Arc<RwLock<WorkspaceSearchIndex>>,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let idx = WorkspaceSearchIndex::build_with_config(dir.path(), WorkspaceIndexConfig::default());
    (dir, Arc::new(RwLock::new(idx)))
}

#[test]
fn dusa_flood_sheds_excess_load_without_starving_fast_ascent_queries() {
    let (_tmp, idx) = empty_shared_index();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let (runner, mut mailbox) = super::search_dsl_async_channel();

    // Flood 128 L2 DusaSolver jobs BEFORE the VM thread drains anything.
    // Under SM11, bounded L2 admission must shed excess jobs immediately without
    // blocking Tokio workers or queuing all 128 unboundedly in RAM.
    let mut dusa_handles = Vec::with_capacity(128);
    let mut shed_count = 0usize;
    rt.block_on(async {
        for i in 0..128 {
            let fut = runner(
                Arc::clone(&idx),
                format!("(search-choice-program (solve r{i}))"),
                512,
                SearchQueryTier::DusaSolver,
            );
            dusa_handles.push(tokio::spawn(fut));
        }
        // Give spawned tasks a few polls so ingress attempts execute.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        for handle in &dusa_handles {
            if handle.is_finished() {
                shed_count += 1;
            }
        }
    });

    assert!(
        shed_count >= 64,
        "SM11 violation: L2 Dusa flood of 128 jobs was NOT load-shed (only {shed_count} rejected immediately); unbounded queuing risks OOM and head-of-line timeout collapse"
    );

    // Priority isolation check: even while L2 DusaSolver ingress is 100% saturated,
    // high-priority L1 FastAscent queries MUST still be admitted without starvation!
    let mut fast_handles = Vec::new();
    let mut fast_shed = 0usize;
    rt.block_on(async {
        for i in 0..8 {
            let fut = runner(
                Arc::clone(&idx),
                format!("(ascent-fragment ((x) <-- (sym x)) ; {i})"),
                512,
                SearchQueryTier::FastAscent,
            );
            fast_handles.push(tokio::spawn(fut));
        }
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        for handle in &fast_handles {
            if handle.is_finished() {
                fast_shed += 1;
            }
        }
    });

    assert_eq!(
        fast_shed, 0,
        "SM11 priority inversion: saturated L2 Dusa tier starved {fast_shed}/8 high-priority L1 FastAscent queries"
    );

    // Drop caller handles cleanly and verify active compaction drains everything to 0.
    for h in dusa_handles.into_iter().chain(fast_handles) {
        h.abort();
    }
    rt.block_on(async {
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
    });
    mailbox.ingest_pending();
    assert!(
        mailbox.fast_queue.is_empty() && mailbox.dusa_queue.is_empty(),
        "SM11 zombie compaction violation: aborted callers remained in queues (fast={}, dusa={})",
        mailbox.fast_queue.len(),
        mailbox.dusa_queue.len()
    );
}

#[test]
fn cancelled_dusa_callers_are_compacted_and_queues_stay_bounded() {
    let (_tmp, idx) = empty_shared_index();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let (runner, mut mailbox) = super::search_dsl_async_channel();

    // Enqueue 40 Dusa jobs, ingest them, then cancel all callers.
    let mut handles = Vec::new();
    rt.block_on(async {
        for i in 0..40 {
            let fut = runner(
                Arc::clone(&idx),
                format!("(search-choice-program (solve c{i}))"),
                256,
                SearchQueryTier::DusaSolver,
            );
            handles.push(tokio::spawn(fut));
        }
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
    });
    mailbox.ingest_pending();
    let live_after_flood = mailbox.dusa_queue.len();
    assert!(
        live_after_flood <= 32,
        "SM11 bound violation: dusa_queue accepted {live_after_flood} jobs > 32 cap"
    );

    for h in handles {
        h.abort();
    }
    rt.block_on(async {
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
    });

    // Next fresh L2 job must reclaim zombie slots and enqueue cleanly.
    let fresh = rt.block_on(async {
        let handle = tokio::spawn(runner(
            Arc::clone(&idx),
            "(search-choice-program (solve fresh))".to_owned(),
            256,
            SearchQueryTier::DusaSolver,
        ));
        for _ in 0..4 {
            tokio::task::yield_now().await;
        }
        handle
    });
    assert!(
        !fresh.is_finished(),
        "SM11 zombie exhaustion: fresh L2 job was rejected after prior callers cancelled"
    );
    mailbox.ingest_pending();
    assert_eq!(mailbox.dusa_queue.len(), 1);
    fresh.abort();
    rt.block_on(async {
        tokio::time::sleep(Duration::from_millis(2)).await;
    });
    mailbox.ingest_pending();
    assert_eq!(mailbox.dusa_queue.len(), 0);
}

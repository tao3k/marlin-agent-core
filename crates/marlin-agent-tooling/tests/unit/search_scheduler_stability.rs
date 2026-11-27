// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Adversarial async cancellation & lost-wakeup stability challenges for
//! `DebouncedSyncCoordinator` refining `MarlinSearchSyncSafetyOverlay.qnt` (SM9).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use marlin_agent_runtime::{TokioAgentRuntime, TokioAgentRuntimeBuildRequest};

use super::{
    CacheDependencyProfile, DebouncedSyncCoordinator, FencedCacheLookup, QueryCacheKey,
    SchedulerCounters, SingleFlightQueryCache,
};
use crate::search_tool::SharedSearchIndexSlot;

/// Adversarial test refining `SingleFlightCancellationSafety.qnt` (`stepLeaderDropLockout`):
/// if the leader future awaiting `ensure_synced` is cancelled/dropped mid-sync
/// (e.g. tool timeout or user preemption), `in_flight` MUST be cleared via an RAII
/// `Drop` guard so subsequent callers complete instead of deadlocking forever.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_sync_leader_does_not_permanently_deadlock_followers() {
    let coord = Arc::new(DebouncedSyncCoordinator::new(Duration::ZERO));
    let counters = Arc::new(SchedulerCounters::default());
    let (owner, _events) =
        TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(8));
    let ctx = owner.context();

    let started = Arc::new(tokio::sync::Notify::new());
    let release_gate = Arc::new(std::sync::Barrier::new(2));

    let c1 = Arc::clone(&coord);
    let cnt1 = Arc::clone(&counters);
    let ctx1 = ctx.clone();
    let started1 = Arc::clone(&started);
    let gate1 = Arc::clone(&release_gate);

    // Run leader inside a short timeout so the leader future is DROPPED while
    // `spawn_blocking(sync_fn)` is still running!
    let leader_res = tokio::time::timeout(
        Duration::from_millis(40),
        c1.ensure_synced(&ctx1, &cnt1, move || {
            started1.notify_one();
            gate1.wait();
        }),
    )
    .await;
    assert!(leader_res.is_err(), "leader future must time out and be dropped");

    // Unblock the detached blocking closure so the thread finishes cleanly.
    release_gate.wait();

    // Now a second caller MUST acquire and finish promptly instead of deadlocking
    // forever on a leaked `in_flight = true` flag!
    let executed = Arc::new(AtomicUsize::new(0));
    let exec_clone = Arc::clone(&executed);
    let follower_res = tokio::time::timeout(
        Duration::from_millis(500),
        coord.ensure_synced(&ctx, &counters, move || {
            exec_clone.fetch_add(1, Ordering::SeqCst);
        }),
    )
    .await;

    assert!(
        follower_res.is_ok(),
        "SM9 CHALLENGE FAILED (leader-drop permanent deadlock): cancelling leader left in_flight=true forever and deadlocked subsequent callers!"
    );
    assert_eq!(executed.load(Ordering::SeqCst), 1);
}

/// Adversarial stress test refining `SingleFlightCancellationSafety.qnt` (`stepUnarmedLostWakeup`):
/// 16 concurrent callers racing across 40 rapid zero-debounce cycles MUST NEVER suffer
/// a lost-wakeup deadlock on `tokio::sync::Notify`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn high_contention_singleflight_waiters_never_suffer_lost_wakeup() {
    let coord = Arc::new(DebouncedSyncCoordinator::new(Duration::ZERO));
    let counters = Arc::new(SchedulerCounters::default());
    let (owner, _events) =
        TokioAgentRuntime::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(32));

    for _round in 0..40 {
        let mut tasks = Vec::with_capacity(16);
        for _ in 0..16 {
            let c = Arc::clone(&coord);
            let cnt = Arc::clone(&counters);
            let ctx = owner.context();
            tasks.push(tokio::spawn(async move {
                c.ensure_synced(&ctx, &cnt, || {}).await;
            }));
        }
        let all_done = tokio::time::timeout(Duration::from_secs(2), async {
            for t in tasks {
                t.await.expect("coalesced sync task panicked");
            }
        })
        .await;
        assert!(
            all_done.is_ok(),
            "SM9 CHALLENGE FAILED (lost wakeup): coalesced waiters deadlocked under contention!"
        );
    }
}

/// SM15 Adversarial Challenge 1: cancelling the single-flight query leader MUST NOT
/// cascade-fail healthy waiting followers; a surviving follower MUST re-elect as
/// leader and deliver the computed result to all remaining callers.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sm15_singleflight_leader_cancel_reelects_healthy_followers() {
    let cache = Arc::new(SingleFlightQueryCache::new(32));
    let counters = Arc::new(SchedulerCounters::default());
    let key = QueryCacheKey {
        generation: "gen-sm15-1".to_owned(),
        query: "(brute-search \"sm15\")".to_owned(),
        output_limit: 4096,
    };

    let leader_entered = Arc::new(tokio::sync::Notify::new());
    let attempts = Arc::new(AtomicUsize::new(0));

    // Spawn leader that blocks longer than its own timeout so it gets cancelled mid-flight.
    let c_leader = Arc::clone(&cache);
    let cnt_leader = Arc::clone(&counters);
    let k_leader = key.clone();
    let entered_leader = Arc::clone(&leader_entered);
    let att_leader = Arc::clone(&attempts);
    let leader_task = tokio::spawn(async move {
        let _ = tokio::time::timeout(
            Duration::from_millis(60),
            c_leader.get_or_compute(k_leader, &cnt_leader, move || async move {
                att_leader.fetch_add(1, Ordering::SeqCst);
                entered_leader.notify_one();
                tokio::time::sleep(Duration::from_secs(5)).await;
                Ok("stale-cancelled-leader".to_owned())
            }),
        )
        .await;
    });

    // Wait until leader holds `in_flight` for `key`.
    leader_entered.notified().await;

    // Spawn 4 healthy followers that coalesce onto the doomed leader.
    let mut followers = Vec::with_capacity(4);
    for _ in 0..4 {
        let c = Arc::clone(&cache);
        let cnt = Arc::clone(&counters);
        let k = key.clone();
        let att = Arc::clone(&attempts);
        followers.push(tokio::spawn(async move {
            c.get_or_compute(k, &cnt, move || async move {
                att.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(15)).await;
                Ok("recovered-by-reelected-follower".to_owned())
            })
            .await
        }));
    }

    leader_task.await.expect("leader task panicked");

    for (idx, f) in followers.into_iter().enumerate() {
        let res = tokio::time::timeout(Duration::from_secs(2), f)
            .await
            .expect("follower timed out")
            .expect("follower task panicked");
        assert_eq!(
            res.as_deref(),
            Ok("recovered-by-reelected-follower"),
            "SM15 CHALLENGE FAILED (leader-cancel cascade): healthy follower {idx} failed with {res:?} instead of re-electing!"
        );
    }
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        2,
        "expected exactly 1 cancelled leader + 1 re-elected follower compute"
    );
}

/// SM15 Adversarial Challenge 2: `SharedSearchIndexSlot::get_or_open` MUST serialize
/// concurrent cold-start opens into ONE authoritative `Arc` (zero split-brain builds)
/// AND isolate different workspace roots (zero cross-worktree contamination).
#[test]
fn sm15_shared_slot_serializes_cold_open_and_isolates_workspace_roots() {
    let base = std::env::temp_dir().join(format!(
        "marlin-sm15-slot-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let ws_a = base.join("ws-alpha");
    let ws_b = base.join("ws-beta");
    std::fs::create_dir_all(&ws_a).expect("mkdir ws_a");
    std::fs::create_dir_all(&ws_b).expect("mkdir ws_b");
    std::fs::write(ws_a.join("alpha.rs"), "pub fn unique_alpha_token_sm15() {}\n").expect("write a");
    std::fs::write(ws_b.join("beta.rs"), "pub fn unique_beta_token_sm15() {}\n").expect("write b");

    let slot = SharedSearchIndexSlot::empty();
    let barrier = Arc::new(std::sync::Barrier::new(6));
    let mut threads = Vec::with_capacity(6);
    for _ in 0..6 {
        let s = slot.clone();
        let ws = ws_a.clone();
        let b = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            b.wait();
            s.get_or_open(&ws)
        }));
    }
    let handles: Vec<_> = threads
        .into_iter()
        .map(|t| t.join().expect("cold-open thread panicked"))
        .collect();
    for (idx, h) in handles.iter().enumerate().skip(1) {
        assert!(
            Arc::ptr_eq(&handles[0], h),
            "SM15 CHALLENGE FAILED (split-brain cold open): concurrent caller {idx} received a distinct WorkspaceSearchIndex Arc!"
        );
    }

    // Switching workspace root to ws_b MUST NOT return ws_a's cached index!
    let idx_b = slot.get_or_open(&ws_b);
    assert!(
        !Arc::ptr_eq(&handles[0], &idx_b),
        "SM15 CHALLENGE FAILED (cross-workspace contamination): ws_b received ws_a's cached index!"
    );
    let digest_a = handles[0].read().expect("read a").generation_digest().as_str().to_owned();
    let digest_b = idx_b.read().expect("read b").generation_digest().as_str().to_owned();
    assert_ne!(
        digest_a, digest_b,
        "SM15 CHALLENGE FAILED: ws_b index shares ws_a's generation digest"
    );

    let _ = std::fs::remove_dir_all(&base);
}

/// SECI Adversarial Challenge 1 (refining `SearchEvidenceChainIntegrity.qnt`):
/// Comment-only edits invalidate ONLY L0 content-sensitive entries while keeping
/// L1/L2 structural AST & Dusa entries reusable at the same structural epoch;
/// structural AST edits selectively evict ONLY dependent entries and advance epoch.
#[test]
fn seci_selective_content_vs_structural_invalidation_and_kv_epoch_fencing() {
    let cache = SingleFlightQueryCache::new(32);
    let k_l0 = QueryCacheKey {
        generation: "gen-1".to_owned(),
        query: "(brute-search \"execute_dsl\")".to_owned(),
        output_limit: 4096,
    };
    let k_struct_runner = QueryCacheKey {
        generation: "gen-1".to_owned(),
        query: "(search-choice-program ... runner.rs)".to_owned(),
        output_limit: 4096,
    };
    let k_struct_codec = QueryCacheKey {
        generation: "gen-1".to_owned(),
        query: "(search-choice-program ... codec.rs)".to_owned(),
        output_limit: 4096,
    };
    let k_neg_scope = QueryCacheKey {
        generation: "gen-1".to_owned(),
        query: "(ascent-fragment ... no-duplicate-in-edb-bridge)".to_owned(),
        output_limit: 4096,
    };

    let ep1 = cache
        .insert_with_profile(
            k_l0.clone(),
            "l0-hit".to_owned(),
            CacheDependencyProfile {
                content_sensitive: true,
                file_deps: vec!["crates/marlin-agent-search-engine/src/edb_bridge/runner.rs".to_owned()],
                negative_scope_prefix: None,
            },
        )
        .expect("insert l0");
    assert_eq!(ep1, 1);

    cache
        .insert_with_profile(
            k_struct_runner.clone(),
            "tier:runner=resident".to_owned(),
            CacheDependencyProfile {
                content_sensitive: false,
                file_deps: vec!["crates/marlin-agent-search-engine/src/edb_bridge/runner.rs".to_owned()],
                negative_scope_prefix: None,
            },
        )
        .expect("insert runner structural");
    cache
        .insert_with_profile(
            k_struct_codec.clone(),
            "tier:codec=resident".to_owned(),
            CacheDependencyProfile {
                content_sensitive: false,
                file_deps: vec!["crates/marlin-agent-search-engine/src/edb_bridge/codec.rs".to_owned()],
                negative_scope_prefix: None,
            },
        )
        .expect("insert codec structural");
    cache
        .insert_with_profile(
            k_neg_scope.clone(),
            "neg:all-clean".to_owned(),
            CacheDependencyProfile {
                content_sensitive: false,
                file_deps: Vec::new(),
                negative_scope_prefix: Some(
                    "crates/marlin-agent-search-engine/src/edb_bridge/".to_owned(),
                ),
            },
        )
        .expect("insert negative scope");

    // 1. Comment-only edit on runner.rs -> evicts ONLY k_l0; structural epoch stays 1!
    let evicted_comment = cache.invalidate_content_paths(&[
        "crates/marlin-agent-search-engine/src/edb_bridge/runner.rs",
    ]);
    assert_eq!(evicted_comment, 1);
    assert_eq!(cache.structural_epoch(), 1);
    assert_eq!(cache.fenced_lookup(&k_l0, 1), FencedCacheLookup::Miss);
    assert_eq!(
        cache.fenced_lookup(&k_struct_runner, 1),
        FencedCacheLookup::Hit {
            value: "tier:runner=resident".to_owned(),
            epoch: 1
        }
    );

    // 2. Structural AST edit on runner.rs -> evicts k_struct_runner, advances epoch to 2;
    //    stale KV-cached epoch-1 lookup on codec is FENCED (RejectedStaleEpoch),
    //    while fresh epoch-2 lookup on unaffected codec.rs SUCCEEDS (zero wasted work)!
    let evicted_struct = cache.invalidate_structural_paths(&[
        "crates/marlin-agent-search-engine/src/edb_bridge/runner.rs",
    ]);
    assert_eq!(evicted_struct, 1);
    assert_eq!(cache.structural_epoch(), 2);
    assert_eq!(cache.fenced_lookup(&k_struct_runner, 2), FencedCacheLookup::Miss);
    assert_eq!(
        cache.fenced_lookup(&k_struct_codec, 1),
        FencedCacheLookup::RejectedStaleEpoch {
            cached_epoch: 1,
            active_epoch: 2
        }
    );
    assert_eq!(
        cache.fenced_lookup(&k_struct_codec, 2),
        FencedCacheLookup::Hit {
            value: "tier:codec=resident".to_owned(),
            epoch: 2
        }
    );

    // 3. DE01 negative-domain expansion: new file added under edb_bridge/ ->
    //    evicts k_neg_scope, preserves unaffected k_struct_codec at epoch 3!
    let evicted_neg = cache.invalidate_negative_scope(
        "crates/marlin-agent-search-engine/src/edb_bridge/new_rogue_provider.rs",
    );
    assert_eq!(evicted_neg, 1);
    assert_eq!(cache.structural_epoch(), 3);
    assert_eq!(cache.fenced_lookup(&k_neg_scope, 3), FencedCacheLookup::Miss);
    assert_eq!(
        cache.fenced_lookup(&k_struct_codec, 3),
        FencedCacheLookup::Hit {
            value: "tier:codec=resident".to_owned(),
            epoch: 3
        }
    );

    // 4. Poison retraction + forensic tombstone permanently blocks re-insertion and reads!
    cache.retract_and_tombstone(k_struct_codec.clone());
    assert_eq!(
        cache.fenced_lookup(&k_struct_codec, cache.structural_epoch()),
        FencedCacheLookup::RejectedPoisonTombstone
    );
    assert_eq!(
        cache.insert_with_profile(
            k_struct_codec,
            "repoisoned-attempt".to_owned(),
            CacheDependencyProfile::default(),
        ),
        Err("poison-tombstone-blocked")
    );
}

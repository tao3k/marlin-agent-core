// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[path = "../tests/unit/context_fixture.rs"]
mod context_fixture;
#[path = "support/scope_identity.rs"]
mod scope_identity;
use marlin_deck_runtime::{DeckRuntime, DeckRuntimeConfig, DeckRuntimeStopReason};
use std::{
    error::Error,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

const TOKIO_WORKERS: usize = 4;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args()
        .skip(1)
        .filter(|arg| !matches!(arg.as_str(), "--bench" | "--test"))
        .collect();
    let batch: u16 = args.first().map(String::as_str).unwrap_or("64").parse()?;
    let count: usize = args.get(1).map(String::as_str).unwrap_or("1024").parse()?;
    if count == 0 {
        return Err("count must be positive".into());
    }
    // Prepare the Rust-only known answers before starting the VM. No reference
    // requests, extra Scheme warmup or admission verdict cache is involved.
    let expected: Vec<_> = (0..count.max(64))
        .map(scope_identity::expected_digest)
        .collect();
    let startup_begin = Instant::now();
    let deck = DeckRuntime::start(
        DeckRuntimeConfig::new(128.try_into()?).with_batch_capacity(batch.try_into()?),
    )?;
    let startup = startup_begin.elapsed();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(TOKIO_WORKERS)
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let scope = context_fixture::scope("benchmark-source");
        let mut completed = Vec::with_capacity(count);
        let mut warmup = Vec::with_capacity(64);
        for index in 0..64 {
            let digest = deck.admit_context(context_fixture::request(&scope, index), Duration::from_secs(60), CancellationToken::new()).await?;
            warmup.push((index, *digest.as_bytes()));
        }
        let started = Instant::now();
        for wave in (0..count).step_by(64) {
            let mut calls = tokio::task::JoinSet::new();
            for index in wave..(wave + 64).min(count) {
                let deck = deck.clone(); let request = context_fixture::request(&scope, index);
                calls.spawn(async move {
                    let start = Instant::now();
                    let digest = deck.admit_context(request, Duration::from_secs(60), CancellationToken::new()).await?;
                    let elapsed = start.elapsed().as_nanos();
                    Ok::<_, marlin_deck_runtime::DeckRuntimeRejection>((index, elapsed, *digest.as_bytes()))
                });
            }
            while let Some(result) = calls.join_next().await { completed.push(result??); }
        }
        let elapsed = started.elapsed();
        let oracle_begin = Instant::now();
        scope_identity::verify(&expected[..64], warmup.iter().map(|(index, digest)| (*index, digest)))?;
        scope_identity::verify(&expected[..count], completed.iter().map(|(index, _, digest)| (*index, digest)))?;
        let oracle = oracle_begin.elapsed();
        let mut latencies: Vec<_> = completed.iter().map(|(_, elapsed, _)| *elapsed).collect();
        latencies.sort_unstable();
        let shutdown_begin = Instant::now();
        let receipt = deck.shutdown().await?;
        let shutdown = shutdown_begin.elapsed();
        assert_eq!(receipt.reason, DeckRuntimeStopReason::Drained);
        assert_eq!(receipt.vm_processors, 1);
        assert!(receipt.context_service_closed);
        assert_eq!(receipt.metrics.requests_succeeded, count as u64 + 64);
        assert_eq!(receipt.mailbox.actors_created, 1);
        assert_eq!(receipt.mailbox.actors_retired, 1);
        assert_eq!(receipt.mailbox.results_collected, count as u64 + 64);
        println!(
            "batch_capacity={batch} requests={count} elapsed_s={:.6} requests_per_s={:.1} p50_us={:.1} p95_us={:.1} p99_us={:.1} p999_us={:.1} max_us={:.1} over_20ms={} startup_s={:.6} shutdown_s={:.6} vm_processors={} tokio_workers={TOKIO_WORKERS} digest_oracle=independent_known_answer digest_verified={} oracle_us={} metrics={:?} mailbox={:?}",
            elapsed.as_secs_f64(),
            count as f64 / elapsed.as_secs_f64(),
            latencies[count / 2] as f64 / 1000.0,
            latencies[(count * 95 / 100).min(count - 1)] as f64 / 1000.0,
            latencies[(count * 99 / 100).min(count - 1)] as f64 / 1000.0,
            latencies[(count * 999 / 1000).min(count - 1)] as f64 / 1000.0,
            latencies[count - 1] as f64 / 1000.0,
            latencies.iter().filter(|latency| **latency > 20_000_000).count(),
            startup.as_secs_f64(),
            shutdown.as_secs_f64(),
            receipt.vm_processors,
            count + 64,
            oracle.as_micros(),
            receipt.metrics,
            receipt.mailbox,
        );
        Ok::<_, Box<dyn Error>>(())
    })
}

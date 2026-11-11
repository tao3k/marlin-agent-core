#[path = "../tests/unit/context_fixture.rs"]
mod context_fixture;
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
    let deck = DeckRuntime::start(
        DeckRuntimeConfig::new(128.try_into()?).with_batch_capacity(batch.try_into()?),
    )?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(TOKIO_WORKERS)
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let scope = context_fixture::scope("benchmark-source");
        let mut latencies = Vec::with_capacity(count);
        for index in 0..64 { deck.admit_context(context_fixture::request(&scope, index), Duration::from_secs(60), CancellationToken::new()).await?; }
        let started = Instant::now();
        for wave in (0..count).step_by(64) {
            let mut calls = tokio::task::JoinSet::new();
            for index in wave..(wave + 64).min(count) {
                let deck = deck.clone(); let request = context_fixture::request(&scope, index);
                calls.spawn(async move {
                    let start = Instant::now();
                    deck.admit_context(request, Duration::from_secs(60), CancellationToken::new()).await?;
                    Ok::<_, marlin_deck_runtime::DeckRuntimeRejection>(start.elapsed().as_nanos())
                });
            }
            while let Some(result) = calls.join_next().await { latencies.push(result??); }
        }
        let elapsed = started.elapsed(); latencies.sort_unstable();
        let receipt = deck.shutdown().await?;
        assert_eq!(receipt.reason, DeckRuntimeStopReason::Drained);
        assert_eq!(receipt.vm_processors, 1);
        assert!(receipt.context_service_closed);
        assert_eq!(receipt.metrics.requests_succeeded, count as u64 + 64);
        println!(
            "batch_capacity={batch} requests={count} elapsed_s={:.6} requests_per_s={:.1} p50_us={:.1} p95_us={:.1} vm_processors={} tokio_workers={TOKIO_WORKERS} metrics={:?}",
            elapsed.as_secs_f64(),
            count as f64 / elapsed.as_secs_f64(),
            latencies[count / 2] as f64 / 1000.0,
            latencies[(count * 95 / 100).min(count - 1)] as f64 / 1000.0,
            receipt.vm_processors,
            receipt.metrics,
        );
        Ok::<_, Box<dyn Error>>(())
    })
}

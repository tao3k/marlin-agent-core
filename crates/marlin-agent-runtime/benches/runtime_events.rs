// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded runtime text-event fanout, including enqueue and dequeue.
//! The Scheme projection, TUI rendering, and terminal latency have separate gates.

use std::{hint::black_box, time::Duration};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use marlin_agent_runtime::{RuntimeEvent, RuntimeEventSink, RuntimeEventStream};

const EVENTS_PER_BATCH: usize = 1024;

fn channels(
    with_capture: bool,
) -> (
    RuntimeEventSink,
    RuntimeEventStream,
    Option<RuntimeEventStream>,
) {
    let (sink, primary) = RuntimeEventSink::channel(EVENTS_PER_BATCH);
    if with_capture {
        let (sink, capture) = sink.with_capture(EVENTS_PER_BATCH);
        (sink, primary, Some(capture))
    } else {
        (sink, primary, None)
    }
}

fn fanout(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("benchmark Tokio runtime");
    for (name, with_capture) in [("primary", false), ("primary-and-capture", true)] {
        c.bench_function(&format!("runtime/event-fanout/{name}-1024"), |b| {
            b.iter_batched(
                || channels(with_capture),
                |(sink, mut primary, mut capture)| {
                    runtime.block_on(async move {
                        for _ in 0..EVENTS_PER_BATCH {
                            sink.emit(RuntimeEvent::new("bench", "x"))
                                .await
                                .expect("uncontended event admission");
                        }
                        for _ in 0..EVENTS_PER_BATCH {
                            black_box(primary.recv().await.expect("primary event"));
                            if let Some(capture) = capture.as_mut() {
                                black_box(capture.recv().await.expect("capture event"));
                            }
                        }
                    });
                },
                BatchSize::SmallInput,
            );
        });
    }
}

criterion_group! { name = benches; config = Criterion::default().sample_size(30).measurement_time(Duration::from_secs(2)); targets = fanout }
criterion_main!(benches);

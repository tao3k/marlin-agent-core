// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Pure in-memory tool admission; excludes handler execution and journaling.

use std::{hint::black_box, sync::Arc, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use marlin_agent_protocol::{ModelToolCall, ModelToolCallId, ModelToolDefinition, ModelToolName};
use marlin_agent_runtime::{
    ModelToolCatalog, ModelToolOutcome, RuntimeContext, RuntimeFuture, ToolRuntime,
};

struct Handler;

impl ToolRuntime for Handler {
    type Invocation = ModelToolCall;
    type Output = ModelToolOutcome;

    fn run_tool(
        &self,
        _call: ModelToolCall,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async { ModelToolOutcome::Completed("ok".into()) })
    }
}

fn admission(c: &mut Criterion) {
    let mut catalog = ModelToolCatalog::new();
    let name = ModelToolName::new("read_workspace").unwrap();
    catalog
        .register(
            ModelToolDefinition {
                name: name.clone(),
                description: "Read a bounded workspace fact".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            Arc::new(Handler),
        )
        .unwrap();
    let offer = catalog.offer();
    for count in [1, 32] {
        let calls = (0..count)
            .map(|index| ModelToolCall {
                id: ModelToolCallId::new(format!("call-{index}")).unwrap(),
                name: name.clone(),
                arguments: serde_json::Map::new(),
            })
            .collect::<Vec<_>>();
        c.bench_function(&format!("model-tool/admit-{count}"), |b| {
            b.iter(|| black_box(&offer).admit_calls(black_box(&calls)).unwrap())
        });
    }
}

criterion_group! { name = benches; config = Criterion::default().sample_size(30).measurement_time(Duration::from_secs(2)); targets = admission }
criterion_main!(benches);

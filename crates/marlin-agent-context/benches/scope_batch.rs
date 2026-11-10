use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use marlin_agent_context::{
    ContextBundleId, ContextDestinationId, ContextOrganizationId, ContextScopeBatch,
    ContextScopeEpoch, ContextScopeKey, ContextScopeRow, ContextSessionTurn, ContextSourceCutId,
    ContextTaskId, ContextTurnIndex,
};
use marlin_agent_protocol::{
    ProjectRuntimeAgentId, ProjectRuntimeProjectId, ProjectRuntimeSessionId,
    ProjectRuntimeWorktreeId,
};

fn prepare_scope_batch(c: &mut Criterion) {
    let bundle = ContextBundleId::new("brief-to-draft");
    let organization = ContextOrganizationId::new("editorial");
    let project = ProjectRuntimeProjectId::new("release-notes");
    let worktree = ProjectRuntimeWorktreeId::new("release-notes/main");
    let cut = ContextSourceCutId::new("cut:root");
    let actor = ProjectRuntimeAgentId::new("writer");
    let task = ContextTaskId::new("draft");
    let destination = ContextDestinationId::new("draft-review");
    let session = ProjectRuntimeSessionId::new("session-1");
    let rows: Vec<_> = (0..64)
        .map(|turn| ContextScopeRow {
            actor: &actor,
            task: &task,
            destination: &destination,
            session_turn: Some(ContextSessionTurn {
                session: &session,
                turn: ContextTurnIndex(turn),
            }),
        })
        .collect();
    let mut group = c.benchmark_group("context_host_prepare");
    for count in [1, 8, 32, 64] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, count| {
            b.iter(|| {
                black_box(
                    ContextScopeBatch::new(
                        ContextScopeKey {
                            bundle: &bundle,
                            organization: &organization,
                            project: &project,
                            worktree: &worktree,
                            source_cut: &cut,
                            epoch: ContextScopeEpoch(1),
                        },
                        black_box(&rows[..*count]),
                    )
                    .expect("bounded Context benchmark batch"),
                )
            });
        });
    }
    group.finish();
}

criterion_group!(benches, prepare_scope_batch);
criterion_main!(benches);

// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Component diagnostics, separate from the unchanged production workload.
#[path = "../tests/unit/context_fixture.rs"]
mod context_fixture;
use gerbil_scheme::{CollectionKind, GerbilRuntime, LinkedGerbilProgram, NumericBuffer};
use marlin_agent_context::{ContextScopeBatch, gerbil::project_scope_batch};
use marlin_deck_runtime as _;
use std::{error::Error, time::Instant};
unsafe extern "C" {
    fn ___LNK_marlin__deck__actor(
        state: *mut gerbil_scheme_sys::GerbilGlobalState,
    ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
    fn marlin_deck_context_profile(root: i64) -> i64;
    fn marlin_deck_context_retention(root: i64) -> i64;
    fn marlin_deck_runtime_statistics(root: i64) -> i64;
    fn gerbil_scheme_rust_take_call_error_raw() -> i64;
}
unsafe extern "C" fn take_error(_: i64) -> i64 {
    // SAFETY: the initialized owner consumes one SDK diagnostic root.
    unsafe { gerbil_scheme_rust_take_call_error_raw() }
}
fn main() -> Result<(), Box<dyn Error>> {
    // SAFETY: the qualification feature links the declared graph and exports.
    let program = unsafe { LinkedGerbilProgram::from_linker(___LNK_marlin__deck__actor) };
    let runtime = GerbilRuntime::initialize_program_single_processor(program)?;
    if std::env::args().skip(1).any(|arg| arg == "retention") {
        return retention(&runtime);
    }
    let scope = context_fixture::scope("benchmark-source");
    let requests: Vec<_> = (0..4096)
        .map(|i| context_fixture::request(&scope, i))
        .collect();
    let started = Instant::now();
    let mut groups = Vec::new();
    for part in requests.chunks(64) {
        let rows: Vec<_> = part.iter().map(|r| r.turn().as_row()).collect();
        let batch = ContextScopeBatch::new(scope.as_key(), &rows)?;
        groups.push(project_scope_batch(&runtime, &batch)?);
    }
    let input = runtime
        .collection_from_objects(CollectionKind::Vector, &groups.iter().collect::<Vec<_>>())?
        .into_object();
    println!(
        "rust_projection_rows=4096 elapsed_us={}",
        started.elapsed().as_micros()
    );
    // SAFETY: this entry belongs to the initialized qualification image.
    let export = unsafe { runtime.bind_object_export(marlin_deck_context_profile) }?;
    let result = match export.call(&input).into_result() {
        Ok(value) => value,
        Err(error) => {
            // SAFETY: the SDK getter returns a unique owned diagnostic root.
            let diagnostic = unsafe { runtime.bind_object_export(take_error) }?;
            if let Ok(root) = diagnostic.call(&input).into_result() {
                eprintln!(
                    "{}",
                    String::from_utf8_lossy(&root.into_bytevector()?.to_vec().into_result()?)
                );
            }
            return Err(error.into());
        }
    };
    let packet = result.into_collection(CollectionKind::Vector)?;
    let stages = packet.element(0)?.into_collection(CollectionKind::Vector)?;
    let names = [
        "decode",
        "lab_validation",
        "lab_public_admission",
        "checked_admission",
        "digest",
        "encode",
        "scheme_pipeline",
        "rechecked_admission",
        "scheme_actor_pipeline",
    ];
    let mut ids = NumericBuffer::<u64>::default();
    let mut before = NumericBuffer::<f64>::default();
    let mut after = NumericBuffer::<f64>::default();
    for index in 0..stages.len() {
        let row = stages
            .element(index)?
            .into_collection(CollectionKind::Vector)?;
        row.element(0)?.as_ref().copy_numeric(&mut ids)?;
        row.element(1)?.as_ref().copy_numeric(&mut before)?;
        row.element(2)?.as_ref().copy_numeric(&mut after)?;
        let b = before.as_slice();
        let a = after.as_slice();
        println!("stats_stage={} before={b:?} after={a:?}", ids.as_slice()[0]);
        println!(
            "stage={} rows=4096 wall_us={:.3} cpu_us={:.3} gc_us={:.3} gc_count={} vm_accounting_before={} vm_accounting_after={} vm_accounting_delta={} physical_allocation_budget_qualified=false",
            names[ids.as_slice()[0] as usize],
            ids.as_slice()[1] as f64 / 1000.,
            (a[0] + a[1] - b[0] - b[1]) * 1e6,
            (a[3] + a[4] - b[3] - b[4]) * 1e6,
            a[6] - b[6],
            b[7],
            a[7],
            a[7] - b[7]
        );
    }
    packet.element(2)?.as_ref().copy_numeric(&mut ids)?;
    println!(
        "rechecking_preserves_function_identity={}",
        ids.as_slice()[0] == 1
    );
    println!(
        "publication_preserves_function_identity={}",
        ids.as_slice().get(1) == Some(&1)
    );
    assert_eq!(
        ids.as_slice(),
        &[1, 1],
        "canonical Lab admission was wrapped again"
    );
    let output = packet.element(1)?.into_collection(CollectionKind::Vector)?;
    let started = Instant::now();
    let mut count = 0;
    let mut encoded = Vec::new();
    for index in 0..output.len() {
        let digests =
            marlin_agent_context::gerbil::project_scope_digests(output.element(index)?, 64)?;
        count += digests.len();
        for digest in digests {
            encoded.extend_from_slice(digest.as_bytes());
        }
    }
    assert_eq!(count, 4096);
    if let Some(path) = std::env::args().skip(1).find(|arg| arg != "--bench") {
        assert_eq!(encoded.len(), 4096 * 32);
        std::fs::write(path, encoded)?;
    }
    println!(
        "rust_result_projection_rows={count} elapsed_us={}",
        started.elapsed().as_micros()
    );
    // Existing diagnostic export: this is an upper-bound probe of a typed
    // roundtrip plus native statistics collection and copying, not pure ABI time.
    let control = runtime.scalar_object(gerbil_scheme::SchemeScalar::Boolean(false))?;
    // SAFETY: the statistics entry is part of this initialized production graph.
    let stats = unsafe { runtime.bind_object_export(marlin_deck_runtime_statistics) }?;
    let mut counters = NumericBuffer::<f64>::default();
    stats
        .call(&control)
        .into_result()?
        .as_ref()
        .copy_numeric(&mut counters)?;
    assert_eq!(counters.as_slice().len(), 20);
    let gc_before = (counters.as_slice()[6], counters.as_slice()[5]);
    let started = Instant::now();
    for _ in 0..10_000 {
        stats
            .call(&control)
            .into_result()?
            .as_ref()
            .copy_numeric(&mut counters)?;
        assert_eq!(counters.as_slice().len(), 20);
    }
    let elapsed = started.elapsed();
    println!(
        "abi_statistics_roundtrips=10000 total_us={} mean_us={:.3} gc_count={} gc_wall_us={:.3} scope=typed_export_plus_statistics_and_copy",
        elapsed.as_micros(),
        elapsed.as_secs_f64() * 100.,
        counters.as_slice()[6] - gc_before.0,
        (counters.as_slice()[5] - gc_before.1) * 1e6
    );
    Ok(())
}

fn retention(runtime: &GerbilRuntime) -> Result<(), Box<dyn Error>> {
    let control = runtime.scalar_object(gerbil_scheme::SchemeScalar::Boolean(false))?;
    // SAFETY: this diagnostic belongs to the initialized qualification graph.
    let export = unsafe { runtime.bind_object_export(marlin_deck_context_retention) }?;
    let output = export
        .call(&control)
        .into_result()?
        .into_collection(CollectionKind::Vector)?;
    let snapshots = output.element(0)?.into_collection(CollectionKind::Vector)?;
    let names = [
        "before_selection",
        "lab_context",
        "marlin_context",
        "deck_publication",
        "empty_module",
        "module_closed",
    ];
    assert_eq!(snapshots.len(), names.len());
    let mut stats = NumericBuffer::<f64>::default();
    for (index, name) in names.iter().enumerate() {
        let snapshot = snapshots
            .element(index)?
            .into_collection(CollectionKind::Vector)?;
        snapshot.element(0)?.as_ref().copy_numeric(&mut stats)?;
        let mut registry = NumericBuffer::<u64>::default();
        snapshot.element(1)?.as_ref().copy_numeric(&mut registry)?;
        assert_eq!(registry.as_slice().len(), 4);
        let mut graph = NumericBuffer::<u64>::default();
        snapshot.element(2)?.as_ref().copy_numeric(&mut graph)?;
        assert_eq!(graph.as_slice().len(), 9);
        let graph = graph.as_slice();
        assert!(graph[0] >= graph[1] + graph[2]);
        assert!(graph[1] >= registry.as_slice()[1]);
        assert!(graph[3] >= registry.as_slice()[2]);
        assert!(graph[4] >= registry.as_slice()[3]);
        assert!(graph[5] + graph[6] <= graph[1] + graph[2]);
        println!(
            "retention_graph_stage={name} contexts={} module_contexts={} prelude_contexts={} bindings={} code_bodies={} pending_promises={} determined_promises={} import_visits={} export_visits={} ownership_graph_mutated=false",
            graph[0],
            graph[1],
            graph[2],
            graph[3],
            graph[4],
            graph[5],
            graph[6],
            graph[7],
            graph[8]
        );
        println!(
            "retention_registry_stage={name} entries={} unique_contexts={} context_bindings={} context_code_bodies={}",
            registry.as_slice()[0],
            registry.as_slice()[1],
            registry.as_slice()[2],
            registry.as_slice()[3]
        );
        assert_eq!(stats.as_slice().len(), 20);
        let values = stats.as_slice();
        println!(
            "retention_stage={name} forced_gc_diagnostic=true performance_qualified=false heap_capacity_bytes={} post_gc_live_bytes={} movable_live_bytes={} still_live_bytes={} last_gc_cpu_us={:.3}",
            values[15],
            values[17],
            values[18],
            values[19],
            (values[12] + values[13]) * 1e6
        );
    }
    let mut counters = NumericBuffer::<u64>::default();
    output.element(1)?.as_ref().copy_numeric(&mut counters)?;
    assert_eq!(
        counters.as_slice(),
        &[0, 0, 0, 0],
        "retention probe unexpectedly ran business messages or Actors"
    );
    println!("retention_business_messages=0 actors_created=0 actors_retired=0");
    Ok(())
}

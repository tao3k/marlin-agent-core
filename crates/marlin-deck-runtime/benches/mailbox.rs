//! One Rust entry into the VM drives planned mailbox messages and typed receipts.

use gerbil_scheme::{GerbilRuntime, LinkedGerbilProgram, NumericBuffer};
use marlin_deck_runtime as _;
use std::{error::Error, time::Instant};

unsafe extern "C" {
    fn ___LNK_marlin__deck__actor(
        state: *mut gerbil_scheme_sys::GerbilGlobalState,
    ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
    fn marlin_deck_mailbox_benchmark(root: i64) -> i64;
    fn gerbil_scheme_rust_take_call_error_raw() -> i64;
}

// Adapt the SDK's owner-local, consumed diagnostic root to its object API.
unsafe extern "C" fn take_benchmark_error(_: i64) -> i64 {
    // SAFETY: invoked only by this benchmark's initialized VM owner.
    unsafe { gerbil_scheme_rust_take_call_error_raw() }
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args()
        .skip(1)
        .filter(|argument| !matches!(argument.as_str(), "--bench" | "--test"))
        .collect();
    if arguments.len() > 4
        || arguments
            .get(2)
            .is_some_and(|argument| argument != "latency")
    {
        return Err(
            "expected message count, identity count and optional latency arrival-rate".into(),
        );
    }
    let count: u64 = arguments
        .first()
        .map(String::as_str)
        .unwrap_or("1000000")
        .parse()?;
    let identities: u64 = arguments
        .get(1)
        .map(String::as_str)
        .unwrap_or("1")
        .parse()?;
    let latency = arguments
        .get(2)
        .is_some_and(|argument| argument == "latency");
    let rate: u64 = if latency {
        arguments
            .get(3)
            .map(String::as_str)
            .unwrap_or("1000000")
            .parse()?
    } else {
        0
    };
    if count == 0 || identities == 0 || (latency && rate == 0) {
        return Err("message, identity and enabled arrival rates must be positive".into());
    }

    // SAFETY: the qualification feature links this graph and its rooted export together.
    let program = unsafe { LinkedGerbilProgram::from_linker(___LNK_marlin__deck__actor) };
    let runtime = GerbilRuntime::initialize_program_single_processor(program)?;
    assert_eq!(runtime.active_processors()?, 1);
    let input = runtime.numeric_vector(&[count, identities, rate])?;
    // SAFETY: the export is compiled into this initialized qualification graph.
    let export = unsafe { runtime.bind_object_export(marlin_deck_mailbox_benchmark) }?;
    let started = Instant::now();
    let result = match export.call(&input).into_result() {
        Ok(result) => result,
        Err(error) => {
            // SAFETY: the SDK diagnostic transfers one owned bytevector root.
            let diagnostic = unsafe { runtime.bind_object_export(take_benchmark_error) }?;
            if let Ok(message) = diagnostic.call(&input).into_result() {
                let bytes = message.into_bytevector()?.to_vec().into_result()?;
                eprintln!(
                    "Scheme benchmark failure: {}",
                    String::from_utf8_lossy(&bytes)
                );
            }
            return Err(error.into());
        }
    };
    let whole_call = started.elapsed();
    let mut values = NumericBuffer::<i64>::default();
    result.as_ref().copy_numeric(&mut values)?;
    let [
        received,
        keys,
        wall,
        cpu,
        gc,
        allocated,
        created,
        retired,
        p50,
        p95,
        p99,
        p999,
        maximum,
        late,
        gc_count,
        heap_capacity,
        post_gc_live,
    ] = values.as_slice()
    else {
        return Err("invalid mailbox benchmark receipt cardinality".into());
    };
    assert_eq!(u64::try_from(*received)?, count);
    assert_eq!(u64::try_from(*keys)?, identities);
    assert_eq!(created, retired);
    let seconds = *wall as f64 / 1_000_000_000.0;
    println!(
        "mailbox_messages={count} logical_identities={identities} messages_per_s={:.1} wall_s={seconds:.9} cpu_s={:.9} gc_cpu_s={:.9} vm_accounting_delta={allocated} physical_allocation_budget_qualified=false gc_count={gc_count} heap_capacity_bytes={heap_capacity} post_gc_live_bytes={post_gc_live} actors_created={created} actors_retired={retired} whole_c_abi_call_s={:.9}",
        count as f64 / seconds,
        *cpu as f64 / 1_000_000_000.0,
        *gc as f64 / 1_000_000_000.0,
        whole_call.as_secs_f64(),
    );
    if latency {
        println!(
            "mailbox_latency_samples={count} planned_arrivals_per_s={rate} p50_upper_ms={:.3} p95_upper_ms={:.3} p99_upper_ms={:.3} p999_upper_ms={:.3} max_ms={:.6} over_20ms={late}",
            *p50 as f64 / 1_000_000.0,
            *p95 as f64 / 1_000_000.0,
            *p99 as f64 / 1_000_000.0,
            *p999 as f64 / 1_000_000.0,
            *maximum as f64 / 1_000_000.0,
        );
        if *p99 >= 20_000_000 {
            return Err("Mailbox planned-to-consumed p99 exceeded the 20 ms budget".into());
        }
    }
    Ok(())
}

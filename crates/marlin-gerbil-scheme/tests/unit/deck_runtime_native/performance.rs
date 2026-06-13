use super::support::{fake_select_model_route_fast, route_request};
use marlin_gerbil_scheme::GerbilDeckRuntimeNativeModelRouteSelector;
use std::time::Instant;

#[test]
fn gerbil_deck_runtime_native_selector_performance_gate_stays_in_process() {
    let selector = GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_model_route_fast);
    let request = route_request("cargo test", "sub-agent");
    let iterations = 2_000;

    let started = Instant::now();
    for _ in 0..iterations {
        let receipt = selector
            .evaluate(&request)
            .expect("native selector performance gate should project typed selection");
        assert!(receipt.matched);
    }
    let elapsed = started.elapsed();

    assert!(
        elapsed.as_secs_f64() < 3.0,
        "native selector gate exceeded in-process budget: {elapsed:?} for {iterations} iterations"
    );
}

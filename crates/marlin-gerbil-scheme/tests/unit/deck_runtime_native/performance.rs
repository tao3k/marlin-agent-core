use super::support::{fake_select_model_route_fast, route_request};
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeNativeBridgeStatus, GerbilDeckRuntimeNativeModelRouteSelector,
};
use std::time::Instant;

const MAX_AVERAGE_MICROS_PER_NATIVE_POLICY_CALL: u128 = 1_000;

#[test]
fn gerbil_deck_runtime_native_selector_performance_gate_stays_sub_millisecond() {
    let selector = GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_model_route_fast);
    let request = route_request("cargo test", "sub-agent");
    let iterations = 2_000;

    let started = Instant::now();
    for _ in 0..iterations {
        let receipt = selector
            .evaluate(&request)
            .expect("native selector performance gate should project typed selection");
        assert!(receipt.matched);
        assert_eq!(
            receipt.native_bridge.status,
            GerbilDeckRuntimeNativeBridgeStatus::Ready
        );
    }
    let elapsed = started.elapsed();
    let average_micros = elapsed.as_micros() / iterations as u128;

    assert!(
        average_micros <= MAX_AVERAGE_MICROS_PER_NATIVE_POLICY_CALL,
        "native selector gate exceeded native policy budget: average={average_micros}us total={elapsed:?} iterations={iterations}"
    );
}

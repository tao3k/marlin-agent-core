use super::support::{
    StrategySelectionProjection, strategy_selection_manifest, strategy_selection_schema_id,
    strategy_selection_type_id,
};
use marlin_gerbil_scheme::{GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue};
use std::time::Instant;

#[test]
fn scheme_typed_value_projection_performance_gate_stays_in_process() {
    let registry = GerbilSchemeTypeRegistry::new(strategy_selection_manifest())
        .expect("strategy manifest should build registry");
    let envelopes = (0..2_000)
        .map(|index| {
            GerbilSchemeTypedValue::new(
                strategy_selection_type_id(),
                GerbilSchemeValue::record([
                    (
                        "schema_id",
                        "marlin.deck-runtime.strategy-selection.v1".into(),
                    ),
                    ("matched", true.into()),
                    ("action", format!("dynamic-hook-action-{index}").into()),
                ]),
            )
            .with_schema_id(strategy_selection_schema_id())
        })
        .collect::<Vec<_>>();

    let started = Instant::now();
    for envelope in &envelopes {
        let projection: StrategySelectionProjection = registry
            .decode_typed_value(envelope)
            .expect("validated Scheme typed value should project into Rust type");
        assert!(projection.matched);
    }
    let elapsed = started.elapsed();

    assert!(
        elapsed.as_secs_f64() < 3.0,
        "Scheme typed value projection gate exceeded in-process budget: {elapsed:?}"
    );
}

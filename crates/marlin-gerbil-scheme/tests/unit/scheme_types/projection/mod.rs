mod agent_policy_routing;
mod dynamic;
mod dynamic_graph_policy;
mod graph_loop_continuation;
mod native_abi;
mod performance;
mod policy_pack_projection;
mod serde_payload;
mod typed_contract;

use marlin_gerbil_scheme::GerbilSchemeTypeDecodeError;

pub(super) mod support {
    pub(super) use crate::scheme_types::support::{
        StrategyDecisionProjection, StrategySelectionProjection, nested_strategy_manifest,
        strategy_decision_schema_id, strategy_decision_type_id, strategy_selection_manifest,
        strategy_selection_schema_id, strategy_selection_type_id,
    };
}

fn assert_rust_projection_decode_error(error: GerbilSchemeTypeDecodeError, needle: &str) {
    let GerbilSchemeTypeDecodeError::RustProjection { message } = error else {
        panic!("unexpected non-serde decode error: {error}");
    };

    assert!(
        message.contains(needle),
        "serde decode error did not contain {needle:?}: {message}"
    );
}

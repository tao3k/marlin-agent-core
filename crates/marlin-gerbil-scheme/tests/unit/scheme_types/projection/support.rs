// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

pub(super) use crate::scheme_types::support::{
    StrategyDecisionProjection, StrategySelectionProjection, nested_strategy_manifest,
    strategy_decision_schema_id, strategy_decision_type_id, strategy_selection_manifest,
    strategy_selection_schema_id, strategy_selection_type_id,
};
use marlin_gerbil_scheme::GerbilSchemeTypeDecodeError;

pub(super) fn assert_rust_projection_decode_error(
    error: GerbilSchemeTypeDecodeError,
    needle: &str,
) {
    let GerbilSchemeTypeDecodeError::RustProjection { message } = error else {
        panic!("unexpected non-serde decode error: {error}");
    };

    assert!(
        message.contains(needle),
        "serde decode error did not contain {needle:?}: {message}"
    );
}

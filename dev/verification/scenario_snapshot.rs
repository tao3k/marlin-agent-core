// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use asp_rust::{RustScenarioBenchmarkReceipt, RustScenarioBenchmarkStatus};

/// Render Marlin's stable scenario benchmark evidence snapshot.
///
/// The upstream harness owns validation; this wrapper owns Marlin's redacted
/// evidence surface so measured values do not churn snapshots.
pub fn render_rust_scenario_benchmark_snapshot(receipt: &RustScenarioBenchmarkReceipt) -> String {
    let status = match receipt.status {
        RustScenarioBenchmarkStatus::Pass => "pass",
        RustScenarioBenchmarkStatus::Fail => "fail",
        RustScenarioBenchmarkStatus::Invalid => "invalid",
    };

    format!(
        "status: {status}\n\
         observed_total: <measured>\n\
         observed_memory_bytes: <measured>\n\
         timings: fixture_ms=<measured>\n"
    )
}

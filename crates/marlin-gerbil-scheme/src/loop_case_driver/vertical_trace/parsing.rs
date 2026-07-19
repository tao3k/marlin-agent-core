//! Parses and verifies Rust-owned vertical-trace rows.

use super::row::vertical_trace_receipt_from_row;
use super::types::{
    GerbilLoopCaseDriverVerticalTraceError, GerbilLoopCaseDriverVerticalTraceReceipt,
};
use std::collections::BTreeMap;

/// Parse a Rust-owned CLI trace emitted by the Scheme case-driver smoke.
pub fn parse_gerbil_loop_case_driver_vertical_trace(
    stdout: &str,
) -> Result<Vec<GerbilLoopCaseDriverVerticalTraceReceipt>, GerbilLoopCaseDriverVerticalTraceError> {
    let mut rows: BTreeMap<usize, BTreeMap<String, String>> = BTreeMap::new();

    for line in stdout.lines() {
        let Some(rest) = line.strip_prefix("vertical-case.") else {
            continue;
        };
        let Some((key, value)) = rest.split_once('=') else {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace line missing '=': {line}"
            )));
        };
        let Some((index, field)) = key.split_once('.') else {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace key missing field: {key}"
            )));
        };
        let index = index.parse::<usize>().map_err(|error| {
            GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace index {index:?} is invalid: {error}"
            ))
        })?;
        rows.entry(index)
            .or_default()
            .insert(field.to_owned(), value.to_owned());
    }

    if rows.is_empty() {
        return Err(GerbilLoopCaseDriverVerticalTraceError::new(
            "vertical trace did not contain any vertical-case rows",
        ));
    }

    let mut receipts = Vec::with_capacity(rows.len());
    for (expected_index, (index, row)) in rows.into_iter().enumerate() {
        if index != expected_index {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace index {index} is not contiguous at {expected_index}"
            )));
        }
        receipts.push(vertical_trace_receipt_from_row(index, &row)?);
    }

    Ok(receipts)
}

/// Parse and validate a complete Scheme vertical mainline trace.
pub fn verify_gerbil_loop_case_driver_vertical_trace(
    stdout: &str,
    expected_count: usize,
) -> Result<Vec<GerbilLoopCaseDriverVerticalTraceReceipt>, GerbilLoopCaseDriverVerticalTraceError> {
    let receipts = parse_gerbil_loop_case_driver_vertical_trace(stdout)?;
    if receipts.len() != expected_count {
        return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
            "vertical trace receipt count {} does not match expected {expected_count}",
            receipts.len()
        )));
    }

    for (index, receipt) in receipts.iter().enumerate() {
        receipt.ensure_trusted(index)?;
    }

    Ok(receipts)
}

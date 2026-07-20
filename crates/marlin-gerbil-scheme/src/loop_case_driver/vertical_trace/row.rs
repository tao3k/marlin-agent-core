//! Decodes one vertical-trace row into typed receipt data.

use super::types::{
    GerbilLoopCaseDriverVerticalTraceError, GerbilLoopCaseDriverVerticalTraceReceipt,
};
use crate::loop_case_driver::{
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverCaseId, GerbilLoopCaseDriverLoopProgramId,
    GerbilLoopCaseDriverProfileRef, GerbilLoopCaseSchemeBoundary,
    GerbilLoopCaseSerializationBoundary,
};
use std::collections::BTreeMap;

pub(super) fn vertical_trace_receipt_from_row(
    index: usize,
    row: &BTreeMap<String, String>,
) -> Result<GerbilLoopCaseDriverVerticalTraceReceipt, GerbilLoopCaseDriverVerticalTraceError> {
    let policy_mixin_stack_present =
        optional_vertical_trace_bool(row, "policy-mixin-stack-present?", false)?;
    let policy_mixin_stack_slot_merge_laws = optional_vertical_trace_delimited_strings(
        index,
        row,
        "policy-mixin-stack-slot-merge-laws",
        '|',
        policy_mixin_stack_present,
    )?;

    Ok(GerbilLoopCaseDriverVerticalTraceReceipt {
        case_id: GerbilLoopCaseDriverCaseId::new(required_vertical_trace_field(
            index, row, "case-id",
        )?),
        profile_ref: GerbilLoopCaseDriverProfileRef::new(required_vertical_trace_field(
            index,
            row,
            "profile-ref",
        )?),
        compiler_owner: required_vertical_trace_field(index, row, "compiler-owner")?.to_owned(),
        compiler_profile_id: required_vertical_trace_field(index, row, "compiler-profile-id")?
            .to_owned(),
        loop_program_id: GerbilLoopCaseDriverLoopProgramId::new(required_vertical_trace_field(
            index,
            row,
            "loop-program-id",
        )?),
        capability_tags: required_vertical_trace_capability_tags(index, row, "capability-tags")?,
        live_gate_env: required_vertical_trace_field(index, row, "live-gate-env")?.to_owned(),
        live_llm_required: required_vertical_trace_bool(index, row, "live-llm-required?")?,
        live_llm_allowed: required_vertical_trace_bool(index, row, "live-llm-allowed?")?,
        live_llm_denial_receipt: required_vertical_trace_field(
            index,
            row,
            "live-llm-denial-receipt",
        )?
        .to_owned(),
        llm_repair_intent: required_vertical_trace_field(index, row, "llm-repair-intent")?
            .to_owned(),
        session_transform: required_vertical_trace_field(index, row, "session-transform")?
            .to_owned(),
        tool_intent_count: required_vertical_trace_usize(index, row, "tool-intent-count")?,
        memory_intent_count: required_vertical_trace_usize(index, row, "memory-intent-count")?,
        placement_intent_count: required_vertical_trace_usize(
            index,
            row,
            "placement-intent-count",
        )?,
        runtime_handoff_kind: required_vertical_trace_field(index, row, "runtime-handoff-kind")?
            .to_owned(),
        runtime_receipt_kind: required_vertical_trace_field(index, row, "runtime-receipt-kind")?
            .to_owned(),
        derived_session_kind: required_vertical_trace_field(index, row, "derived-session-kind")?
            .to_owned(),
        module_kind: required_vertical_trace_field(index, row, "module-kind")?.to_owned(),
        module_user_module: required_vertical_trace_field(index, row, "module-user-module")?
            .to_owned(),
        module_selection_tags: required_vertical_trace_capability_tags(
            index,
            row,
            "module-selection-tags",
        )?,
        module_source_ref: required_vertical_trace_field(index, row, "module-source-ref")?
            .to_owned(),
        module_entrypoint: required_vertical_trace_field(index, row, "module-entrypoint")?
            .to_owned(),
        module_enabled: required_vertical_trace_bool(index, row, "module-enabled?")?,
        resolved_policy_pack_policy_epoch: required_vertical_trace_u64(
            index,
            row,
            "resolved-policy-pack-policy-epoch",
        )?,
        loop_program_policy_epoch: required_vertical_trace_u64(
            index,
            row,
            "loop-program-policy-epoch",
        )?,
        transition_count: required_vertical_trace_usize(index, row, "transition-count")?,
        transition_actions: required_vertical_trace_delimited_strings(
            index,
            row,
            "transition-actions",
            '|',
        )?,
        transition_events: required_vertical_trace_delimited_strings(
            index,
            row,
            "transition-events",
            '|',
        )?,
        mechanism_policy_count: required_vertical_trace_usize(
            index,
            row,
            "mechanism-policy-count",
        )?,
        mechanism_policy_ids: required_vertical_trace_delimited_strings(
            index,
            row,
            "mechanism-policy-ids",
            '|',
        )?,
        policy_digest_length: required_vertical_trace_usize(index, row, "policy-digest-length")?,
        policy_digest_octets: required_vertical_trace_u8_list(
            index,
            row,
            "policy-digest-octets",
            ',',
        )?,
        capability_mask: required_vertical_trace_u64(index, row, "capability-mask")?,
        budget_max_attempts: required_vertical_trace_u64(index, row, "budget-max-attempts")?,
        budget_max_cost_units: required_vertical_trace_u64(index, row, "budget-max-cost-units")?,
        budget_max_wall_time_ms: required_vertical_trace_u64(
            index,
            row,
            "budget-max-wall-time-ms",
        )?,
        policy_forced_slot_count: required_vertical_trace_usize(
            index,
            row,
            "policy-forced-slot-count",
        )?,
        policy_merge_receipt_count: required_vertical_trace_usize(
            index,
            row,
            "policy-merge-receipt-count",
        )?,
        policy_conflict_merge_receipt_count: required_vertical_trace_usize(
            index,
            row,
            "policy-conflict-merge-receipt-count",
        )?,
        policy_merge_kinds: required_vertical_trace_delimited_strings(
            index,
            row,
            "policy-merge-kinds",
            '|',
        )?,
        policy_merge_statuses: required_vertical_trace_delimited_strings(
            index,
            row,
            "policy-merge-statuses",
            '|',
        )?,
        policy_mixin_stack_present,
        policy_mixin_stack_receipt_kind: optional_vertical_trace_field(
            row,
            "policy-mixin-stack-receipt-kind",
            "none",
        )
        .to_owned(),
        policy_mixin_stack_profile_id: optional_vertical_trace_field(
            row,
            "policy-mixin-stack-profile-id",
            "none",
        )
        .to_owned(),
        policy_mixin_stack_mixin_count: optional_vertical_trace_usize(
            index,
            row,
            "policy-mixin-stack-mixin-count",
            0,
        )?,
        policy_mixin_stack_slot_merge_law_count: optional_vertical_trace_usize(
            index,
            row,
            "policy-mixin-stack-slot-merge-law-count",
            0,
        )?,
        policy_mixin_stack_slot_merge_laws,
        policy_mixin_stack_linearization_owner: optional_vertical_trace_field(
            row,
            "policy-mixin-stack-linearization-owner",
            "none",
        )
        .to_owned(),
        policy_mixin_stack_slot_merge_owner: optional_vertical_trace_field(
            row,
            "policy-mixin-stack-slot-merge-owner",
            "none",
        )
        .to_owned(),
        scheme_boundary: required_vertical_trace_scheme_boundary(index, row, "scheme-boundary")?,
        serialization_boundary: required_vertical_trace_serialization_boundary(
            index,
            row,
            "serialization-boundary",
        )?,
    })
}

fn required_vertical_trace_field<'a>(
    index: usize,
    row: &'a BTreeMap<String, String>,
    field: &'static str,
) -> Result<&'a str, GerbilLoopCaseDriverVerticalTraceError> {
    row.get(field).map(String::as_str).ok_or_else(|| {
        GerbilLoopCaseDriverVerticalTraceError::new(format!(
            "vertical trace {index} is missing {field}"
        ))
    })
}

fn required_vertical_trace_u64(
    index: usize,
    row: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<u64, GerbilLoopCaseDriverVerticalTraceError> {
    required_vertical_trace_field(index, row, field)?
        .parse::<u64>()
        .map_err(|error| {
            GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} field {field} is not u64: {error}"
            ))
        })
}

fn required_vertical_trace_usize(
    index: usize,
    row: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<usize, GerbilLoopCaseDriverVerticalTraceError> {
    required_vertical_trace_field(index, row, field)?
        .parse::<usize>()
        .map_err(|error| {
            GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} field {field} is not usize: {error}"
            ))
        })
}

fn required_vertical_trace_bool(
    index: usize,
    row: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<bool, GerbilLoopCaseDriverVerticalTraceError> {
    match required_vertical_trace_field(index, row, field)? {
        "#t" | "true" => Ok(true),
        "#f" | "false" => Ok(false),
        value => Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
            "vertical trace {index} field {field} is not bool: {value}"
        ))),
    }
}

fn required_vertical_trace_delimited_strings(
    index: usize,
    row: &BTreeMap<String, String>,
    field: &'static str,
    separator: char,
) -> Result<Vec<String>, GerbilLoopCaseDriverVerticalTraceError> {
    let raw = required_vertical_trace_field(index, row, field)?.trim();
    if raw.is_empty() {
        return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
            "vertical trace {index} field {field} is empty"
        )));
    }
    Ok(raw
        .split(separator)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn optional_vertical_trace_delimited_strings(
    index: usize,
    row: &BTreeMap<String, String>,
    field: &'static str,
    separator: char,
    require_non_empty: bool,
) -> Result<Vec<String>, GerbilLoopCaseDriverVerticalTraceError> {
    let Some(raw) = row.get(field).map(|value| value.trim()) else {
        return if require_non_empty {
            Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} is missing {field}"
            )))
        } else {
            Ok(Vec::new())
        };
    };
    if raw.is_empty() {
        return if require_non_empty {
            Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} field {field} is empty"
            )))
        } else {
            Ok(Vec::new())
        };
    }
    Ok(raw
        .split(separator)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn optional_vertical_trace_field<'a>(
    row: &'a BTreeMap<String, String>,
    field: &'static str,
    default: &'a str,
) -> &'a str {
    row.get(field)
        .map(|value| value.as_str())
        .unwrap_or(default)
}

fn optional_vertical_trace_bool(
    row: &BTreeMap<String, String>,
    field: &'static str,
    default: bool,
) -> Result<bool, GerbilLoopCaseDriverVerticalTraceError> {
    let Some(value) = row.get(field) else {
        return Ok(default);
    };
    match value.as_str() {
        "#t" | "true" => Ok(true),
        "#f" | "false" => Ok(false),
        other => Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
            "vertical trace field {field} has invalid boolean {other}"
        ))),
    }
}

fn optional_vertical_trace_usize(
    index: usize,
    row: &BTreeMap<String, String>,
    field: &'static str,
    default: usize,
) -> Result<usize, GerbilLoopCaseDriverVerticalTraceError> {
    let Some(value) = row.get(field) else {
        return Ok(default);
    };
    value.parse::<usize>().map_err(|error| {
        GerbilLoopCaseDriverVerticalTraceError::new(format!(
            "vertical trace {index} field {field} is not usize: {error}"
        ))
    })
}

fn required_vertical_trace_u8_list(
    index: usize,
    row: &BTreeMap<String, String>,
    field: &'static str,
    separator: char,
) -> Result<Vec<u8>, GerbilLoopCaseDriverVerticalTraceError> {
    required_vertical_trace_delimited_strings(index, row, field, separator)?
        .into_iter()
        .map(|value| {
            value.parse::<u8>().map_err(|error| {
                GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} field {field} contains invalid octet {value:?}: {error}"
                ))
            })
        })
        .collect()
}

fn required_vertical_trace_capability_tags(
    index: usize,
    row: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<Vec<GerbilLoopCaseDriverCapability>, GerbilLoopCaseDriverVerticalTraceError> {
    let raw = required_vertical_trace_field(index, row, field)?.trim();
    let Some(inner) = raw
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
            "vertical trace {index} field {field} is not a Scheme capability list: {raw}"
        )));
    };
    Ok(inner
        .split_whitespace()
        .map(GerbilLoopCaseDriverCapability::new)
        .collect())
}

fn required_vertical_trace_scheme_boundary(
    index: usize,
    row: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<GerbilLoopCaseSchemeBoundary, GerbilLoopCaseDriverVerticalTraceError> {
    match required_vertical_trace_field(index, row, field)? {
        "scheme-types->rust-types" => Ok(GerbilLoopCaseSchemeBoundary::SchemeTypesToRustTypes),
        boundary => Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
            "vertical trace {index} field {field} has unsupported scheme boundary {boundary}"
        ))),
    }
}

fn required_vertical_trace_serialization_boundary(
    index: usize,
    row: &BTreeMap<String, String>,
    field: &'static str,
) -> Result<GerbilLoopCaseSerializationBoundary, GerbilLoopCaseDriverVerticalTraceError> {
    match required_vertical_trace_field(index, row, field)? {
        "rust-owned-cli-trace-cross-process" => {
            Ok(GerbilLoopCaseSerializationBoundary::RustOwnedCliTraceCrossProcess)
        }
        boundary => Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
            "vertical trace {index} field {field} has unsupported serialization boundary {boundary}"
        ))),
    }
}

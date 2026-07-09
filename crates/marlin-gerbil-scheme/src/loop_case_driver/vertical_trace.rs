//! Rust-owned vertical trace validation for Scheme loop case-driver receipts.

use std::{collections::BTreeMap, error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID;

use super::{
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverCaseId, GerbilLoopCaseDriverLoopProgramId,
    GerbilLoopCaseDriverProfileRef, GerbilLoopCaseSchemeBoundary,
    GerbilLoopCaseSerializationBoundary,
};

/// Rust-owned view of a Scheme-emitted vertical mainline trace.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilLoopCaseDriverVerticalTraceReceipt {
    case_id: GerbilLoopCaseDriverCaseId,
    profile_ref: GerbilLoopCaseDriverProfileRef,
    compiler_owner: String,
    compiler_profile_id: String,
    loop_program_id: GerbilLoopCaseDriverLoopProgramId,
    capability_tags: Vec<GerbilLoopCaseDriverCapability>,
    live_gate_env: String,
    live_llm_required: bool,
    live_llm_allowed: bool,
    live_llm_denial_receipt: String,
    llm_repair_intent: String,
    session_transform: String,
    tool_intent_count: usize,
    memory_intent_count: usize,
    placement_intent_count: usize,
    runtime_handoff_kind: String,
    runtime_receipt_kind: String,
    derived_session_kind: String,
    module_kind: String,
    module_user_module: String,
    module_selection_tags: Vec<GerbilLoopCaseDriverCapability>,
    module_source_ref: String,
    module_entrypoint: String,
    module_enabled: bool,
    resolved_policy_pack_policy_epoch: u64,
    loop_program_policy_epoch: u64,
    transition_count: usize,
    transition_actions: Vec<String>,
    transition_events: Vec<String>,
    mechanism_policy_count: usize,
    mechanism_policy_ids: Vec<String>,
    policy_digest_length: usize,
    policy_digest_octets: Vec<u8>,
    capability_mask: u64,
    budget_max_attempts: u64,
    budget_max_cost_units: u64,
    budget_max_wall_time_ms: u64,
    policy_forced_slot_count: usize,
    policy_merge_receipt_count: usize,
    policy_conflict_merge_receipt_count: usize,
    policy_merge_kinds: Vec<String>,
    policy_merge_statuses: Vec<String>,
    policy_mixin_stack_present: bool,
    policy_mixin_stack_receipt_kind: String,
    policy_mixin_stack_profile_id: String,
    policy_mixin_stack_mixin_count: usize,
    policy_mixin_stack_slot_merge_law_count: usize,
    policy_mixin_stack_slot_merge_laws: Vec<String>,
    policy_mixin_stack_linearization_owner: String,
    policy_mixin_stack_slot_merge_owner: String,
    scheme_boundary: GerbilLoopCaseSchemeBoundary,
    serialization_boundary: GerbilLoopCaseSerializationBoundary,
}

/// Error returned when a Scheme vertical mainline trace cannot be trusted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilLoopCaseDriverVerticalTraceError {
    message: String,
}

impl GerbilLoopCaseDriverVerticalTraceError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for GerbilLoopCaseDriverVerticalTraceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GerbilLoopCaseDriverVerticalTraceError {}

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

impl GerbilLoopCaseDriverVerticalTraceReceipt {
    #[must_use]
    pub fn case_id(&self) -> &GerbilLoopCaseDriverCaseId {
        &self.case_id
    }

    #[must_use]
    pub fn transition_count(&self) -> usize {
        self.transition_count
    }

    pub fn transition_actions(&self) -> impl Iterator<Item = &str> {
        self.transition_actions.iter().map(String::as_str)
    }

    pub fn transition_events(&self) -> impl Iterator<Item = &str> {
        self.transition_events.iter().map(String::as_str)
    }

    #[must_use]
    pub fn mechanism_policy_count(&self) -> usize {
        self.mechanism_policy_count
    }

    pub fn mechanism_policy_ids(&self) -> impl Iterator<Item = &str> {
        self.mechanism_policy_ids.iter().map(String::as_str)
    }

    #[must_use]
    pub fn compiler_owner(&self) -> &str {
        &self.compiler_owner
    }

    #[must_use]
    pub fn profile_ref(&self) -> &GerbilLoopCaseDriverProfileRef {
        &self.profile_ref
    }

    #[must_use]
    pub fn capability_mask(&self) -> u64 {
        self.capability_mask
    }

    #[must_use]
    pub fn policy_epoch(&self) -> u64 {
        self.loop_program_policy_epoch
    }

    #[must_use]
    pub fn policy_digest_octets(&self) -> &[u8] {
        &self.policy_digest_octets
    }

    pub fn capability_tags(&self) -> impl Iterator<Item = &GerbilLoopCaseDriverCapability> {
        self.capability_tags.iter()
    }

    #[must_use]
    pub fn live_gate_env(&self) -> &str {
        &self.live_gate_env
    }

    #[must_use]
    pub fn live_llm_required(&self) -> bool {
        self.live_llm_required
    }

    #[must_use]
    pub fn live_llm_allowed(&self) -> bool {
        self.live_llm_allowed
    }

    #[must_use]
    pub fn live_llm_denial_receipt(&self) -> &str {
        &self.live_llm_denial_receipt
    }

    #[must_use]
    pub fn llm_repair_intent(&self) -> &str {
        &self.llm_repair_intent
    }

    #[must_use]
    pub fn session_transform(&self) -> &str {
        &self.session_transform
    }

    #[must_use]
    pub fn tool_intent_count(&self) -> usize {
        self.tool_intent_count
    }

    #[must_use]
    pub fn memory_intent_count(&self) -> usize {
        self.memory_intent_count
    }

    #[must_use]
    pub fn placement_intent_count(&self) -> usize {
        self.placement_intent_count
    }

    #[must_use]
    pub fn runtime_handoff_kind(&self) -> &str {
        &self.runtime_handoff_kind
    }

    #[must_use]
    pub fn runtime_receipt_kind(&self) -> &str {
        &self.runtime_receipt_kind
    }

    #[must_use]
    pub fn derived_session_kind(&self) -> &str {
        &self.derived_session_kind
    }

    #[must_use]
    pub fn module_kind(&self) -> &str {
        &self.module_kind
    }

    #[must_use]
    pub fn module_user_module(&self) -> &str {
        &self.module_user_module
    }

    pub fn module_selection_tags(&self) -> impl Iterator<Item = &GerbilLoopCaseDriverCapability> {
        self.module_selection_tags.iter()
    }

    #[must_use]
    pub fn module_source_ref(&self) -> &str {
        &self.module_source_ref
    }

    #[must_use]
    pub fn module_entrypoint(&self) -> &str {
        &self.module_entrypoint
    }

    #[must_use]
    pub fn module_enabled(&self) -> bool {
        self.module_enabled
    }

    #[must_use]
    pub fn loop_program_id(&self) -> &GerbilLoopCaseDriverLoopProgramId {
        &self.loop_program_id
    }

    #[must_use]
    pub fn policy_forced_slot_count(&self) -> usize {
        self.policy_forced_slot_count
    }

    #[must_use]
    pub fn policy_merge_receipt_count(&self) -> usize {
        self.policy_merge_receipt_count
    }

    #[must_use]
    pub fn policy_conflict_merge_receipt_count(&self) -> usize {
        self.policy_conflict_merge_receipt_count
    }

    pub fn policy_merge_kinds(&self) -> impl Iterator<Item = &str> {
        self.policy_merge_kinds.iter().map(String::as_str)
    }

    pub fn policy_merge_statuses(&self) -> impl Iterator<Item = &str> {
        self.policy_merge_statuses.iter().map(String::as_str)
    }

    #[must_use]
    pub fn policy_mixin_stack_present(&self) -> bool {
        self.policy_mixin_stack_present
    }

    #[must_use]
    pub fn policy_mixin_stack_receipt_kind(&self) -> &str {
        &self.policy_mixin_stack_receipt_kind
    }

    #[must_use]
    pub fn policy_mixin_stack_profile_id(&self) -> &str {
        &self.policy_mixin_stack_profile_id
    }

    #[must_use]
    pub fn policy_mixin_stack_mixin_count(&self) -> usize {
        self.policy_mixin_stack_mixin_count
    }

    #[must_use]
    pub fn policy_mixin_stack_slot_merge_law_count(&self) -> usize {
        self.policy_mixin_stack_slot_merge_law_count
    }

    pub fn policy_mixin_stack_slot_merge_laws(&self) -> impl Iterator<Item = &str> {
        self.policy_mixin_stack_slot_merge_laws
            .iter()
            .map(String::as_str)
    }

    #[must_use]
    pub fn policy_mixin_stack_linearization_owner(&self) -> &str {
        &self.policy_mixin_stack_linearization_owner
    }

    #[must_use]
    pub fn policy_mixin_stack_slot_merge_owner(&self) -> &str {
        &self.policy_mixin_stack_slot_merge_owner
    }

    #[must_use]
    pub fn has_capability(&self, tag: &GerbilLoopCaseDriverCapability) -> bool {
        self.capability_tags
            .iter()
            .any(|capability| capability == tag)
    }

    fn ensure_trusted(&self, index: usize) -> Result<(), GerbilLoopCaseDriverVerticalTraceError> {
        if self.case_id.as_str().trim().is_empty() {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has an empty case id"
            )));
        }
        if self.profile_ref.as_str().trim().is_empty() {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has an empty profile ref"
            )));
        }
        if self.compiler_owner != "gerbil-poo-flow" {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} compiler owner {} is not gerbil-poo-flow",
                self.compiler_owner
            )));
        }
        if self.profile_ref.as_str() != self.compiler_profile_id {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} profile {} does not match compiler profile {}",
                self.profile_ref, self.compiler_profile_id
            )));
        }
        if self.loop_program_id.as_str().trim().is_empty() {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has an empty loop program id"
            )));
        }
        if self.capability_tags.is_empty() {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has no capability tags"
            )));
        }
        if let Some(tag) = self
            .capability_tags
            .iter()
            .find(|tag| !tag.as_str().starts_with('+'))
        {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} capability tag {} is not a POO capability lane",
                tag.as_str()
            )));
        }
        if self.live_llm_required {
            if self.live_gate_env != "MARLIN_LIVE_LLM" {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} live LLM repair must use MARLIN_LIVE_LLM gate, got {}",
                    self.live_gate_env
                )));
            }
            if self.live_llm_allowed {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} live LLM repair should be denied in smoke trace"
                )));
            }
            if self.live_llm_denial_receipt != "deferred-no-live-llm" {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} live LLM repair denial receipt {} is not deferred-no-live-llm",
                    self.live_llm_denial_receipt
                )));
            }
            if self.llm_repair_intent == "none" || self.llm_repair_intent.trim().is_empty() {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} live LLM repair has no typed repair intent"
                )));
            }
            if !self.has_capability(&GerbilLoopCaseDriverCapability::new("+tool-repair")) {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} live LLM repair lacks +tool-repair capability"
                )));
            }
            if !self
                .transition_actions
                .iter()
                .any(|action| action == "invoke_model")
            {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} live LLM repair lacks model invocation"
                )));
            }
            if self.tool_intent_count == 0 {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} live LLM repair has no tool intent"
                )));
            }
        } else {
            if self.live_gate_env != "none" {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} non-LLM case should not declare live gate {}, got {}",
                    "MARLIN_LIVE_LLM", self.live_gate_env
                )));
            }
            if self.live_llm_allowed {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} non-LLM case should not allow live LLM"
                )));
            }
            if self.live_llm_denial_receipt != "not-required" {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} non-LLM denial receipt {} is not not-required",
                    self.live_llm_denial_receipt
                )));
            }
            if self.llm_repair_intent != "none" {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} non-LLM repair intent {} should be none",
                    self.llm_repair_intent
                )));
            }
        }
        if self.session_transform != "loop-policy-profile->loop-program" {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} session transform {} is not loop-policy-profile->loop-program",
                self.session_transform
            )));
        }
        if self.placement_intent_count == 0 {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has no runtime placement intent"
            )));
        }
        let declares_memory_capability = self
            .has_capability(&GerbilLoopCaseDriverCapability::new("+memory"))
            || self.has_capability(&GerbilLoopCaseDriverCapability::new("+memory-recall"));
        if declares_memory_capability && self.memory_intent_count == 0 {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} declares memory capability without memory intent"
            )));
        }
        if self.memory_intent_count > 0 && !declares_memory_capability {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} declares memory intent without memory capability"
            )));
        }
        if self.runtime_handoff_kind != "loop-program-runtime-handoff" {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} runtime handoff kind {} is not loop-program-runtime-handoff",
                self.runtime_handoff_kind
            )));
        }
        if self.runtime_receipt_kind != "loop-program-runtime-receipt" {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} runtime receipt kind {} is not loop-program-runtime-receipt",
                self.runtime_receipt_kind
            )));
        }
        if self.derived_session_kind != "derived-session/from-loop-receipt" {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} derived session kind {} is not derived-session/from-loop-receipt",
                self.derived_session_kind
            )));
        }
        if self.module_kind != "marlin.config-interface.loop-policy-profile-projection.v1" {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} module kind {} is not the loop policy profile projection module",
                self.module_kind
            )));
        }
        if self.module_user_module != "funflow" {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} module user module {} is not funflow",
                self.module_user_module
            )));
        }
        if self.module_selection_tags != self.capability_tags {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} module selection tags do not match capability tags"
            )));
        }
        if self.module_source_ref != self.profile_ref.as_str() {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} module source ref {} does not match profile ref {}",
                self.module_source_ref, self.profile_ref
            )));
        }
        if self.module_entrypoint != "marlinLoopPolicyProfileCompilerReceipts" {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} module entrypoint {} is not the Scheme compiler receipt entrypoint",
                self.module_entrypoint
            )));
        }
        if !self.module_enabled {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} module projection is disabled"
            )));
        }
        if self.resolved_policy_pack_policy_epoch != self.loop_program_policy_epoch {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} resolved policy epoch {} does not match loop program epoch {}",
                self.resolved_policy_pack_policy_epoch, self.loop_program_policy_epoch
            )));
        }
        if self.transition_count == 0 {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has no loop transitions"
            )));
        }
        if self.transition_actions.len() != self.transition_count {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} action count {} does not match transition count {}",
                self.transition_actions.len(),
                self.transition_count
            )));
        }
        if self.transition_events.len() != self.transition_count {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} event count {} does not match transition count {}",
                self.transition_events.len(),
                self.transition_count
            )));
        }
        if self.transition_events.first().map(String::as_str) != Some("start") {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} first transition event must be start"
            )));
        }
        if self.transition_actions.last().map(String::as_str) != Some("stop") {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} final transition action must be stop"
            )));
        }
        if self.mechanism_policy_count == 0 {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has no mechanism policies"
            )));
        }
        if self.mechanism_policy_ids.len() != self.mechanism_policy_count {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} mechanism id count {} does not match mechanism policy count {}",
                self.mechanism_policy_ids.len(),
                self.mechanism_policy_count
            )));
        }
        if let Some(policy_id) = self
            .mechanism_policy_ids
            .iter()
            .find(|policy_id| policy_id.trim().is_empty())
        {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has an empty mechanism policy id {policy_id:?}"
            )));
        }
        if self.policy_digest_length != 32 {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} policy digest length {} does not equal 32",
                self.policy_digest_length
            )));
        }
        if self.policy_digest_octets.len() != self.policy_digest_length {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} policy digest octet count {} does not match length {}",
                self.policy_digest_octets.len(),
                self.policy_digest_length
            )));
        }
        if self.capability_mask == 0 {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has an empty capability mask"
            )));
        }
        if self.budget_max_attempts == 0
            || self.budget_max_cost_units == 0
            || self.budget_max_wall_time_ms == 0
        {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has an empty runtime budget"
            )));
        }
        if self.policy_forced_slot_count == 0 {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has no forced policy slots"
            )));
        }
        if self.policy_merge_receipt_count == 0 {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} has no policy merge receipts"
            )));
        }
        if self.policy_merge_kinds.len() != self.policy_merge_receipt_count {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} policy merge kind count {} does not match merge receipt count {}",
                self.policy_merge_kinds.len(),
                self.policy_merge_receipt_count
            )));
        }
        if self.policy_merge_statuses.len() != self.policy_merge_receipt_count {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} policy merge status count {} does not match merge receipt count {}",
                self.policy_merge_statuses.len(),
                self.policy_merge_receipt_count
            )));
        }
        let conflict_status_count = self
            .policy_merge_statuses
            .iter()
            .filter(|status| status.as_str() == "conflict")
            .count();
        if self.policy_conflict_merge_receipt_count != conflict_status_count {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} policy conflict merge receipt count {} does not match status count {conflict_status_count}",
                self.policy_conflict_merge_receipt_count
            )));
        }
        if let Some(status) = self
            .policy_merge_statuses
            .iter()
            .find(|status| !matches!(status.as_str(), "applied" | "conflict"))
        {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} policy merge status {status} is not applied or conflict"
            )));
        }
        if self.policy_mixin_stack_present {
            if self.policy_mixin_stack_receipt_kind != GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID
            {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} mixin-stack receipt kind {} is not {}",
                    self.policy_mixin_stack_receipt_kind,
                    GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID
                )));
            }
            if self.policy_mixin_stack_profile_id != self.profile_ref.as_str() {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} mixin-stack profile {} does not match profile ref {}",
                    self.policy_mixin_stack_profile_id, self.profile_ref
                )));
            }
            if self.policy_mixin_stack_mixin_count == 0 {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} mixin-stack has no policy mixins"
                )));
            }
            if self.policy_mixin_stack_slot_merge_law_count == 0 {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} mixin-stack has no slot merge laws"
                )));
            }
            if self.policy_mixin_stack_slot_merge_laws.len()
                != self.policy_mixin_stack_slot_merge_law_count
            {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} mixin-stack law count {} does not match law vector {}",
                    self.policy_mixin_stack_slot_merge_law_count,
                    self.policy_mixin_stack_slot_merge_laws.len()
                )));
            }
            if self.policy_mixin_stack_slot_merge_law_count != self.policy_merge_receipt_count {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} mixin-stack law count {} does not match policy merge receipt count {}",
                    self.policy_mixin_stack_slot_merge_law_count, self.policy_merge_receipt_count
                )));
            }
            if let Some(law) = self
                .policy_mixin_stack_slot_merge_laws
                .iter()
                .find(|law| !law.contains('='))
            {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} mixin-stack law {law} is not slot=merge"
                )));
            }
            if self.policy_mixin_stack_linearization_owner != "poo-flow.c3-c4" {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} mixin-stack linearization owner {} is not poo-flow.c3-c4",
                    self.policy_mixin_stack_linearization_owner
                )));
            }
            if self.policy_mixin_stack_slot_merge_owner != "poo-flow.slot-merge-algebra" {
                return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                    "vertical trace {index} mixin-stack slot merge owner {} is not poo-flow.slot-merge-algebra",
                    self.policy_mixin_stack_slot_merge_owner
                )));
            }
        } else if self.policy_mixin_stack_receipt_kind != "none"
            || self.policy_mixin_stack_profile_id != "none"
            || self.policy_mixin_stack_mixin_count != 0
            || self.policy_mixin_stack_slot_merge_law_count != 0
            || !self.policy_mixin_stack_slot_merge_laws.is_empty()
            || self.policy_mixin_stack_linearization_owner != "none"
            || self.policy_mixin_stack_slot_merge_owner != "none"
        {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} absent mixin-stack receipt still carries payload"
            )));
        }
        if self.scheme_boundary != GerbilLoopCaseSchemeBoundary::SchemeTypesToRustTypes {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} scheme boundary {} is not native Scheme-to-Rust",
                self.scheme_boundary.as_str()
            )));
        }
        if self.serialization_boundary
            != GerbilLoopCaseSerializationBoundary::RustOwnedCliTraceCrossProcess
        {
            return Err(GerbilLoopCaseDriverVerticalTraceError::new(format!(
                "vertical trace {index} serialization boundary {} is not Rust-owned CLI trace",
                self.serialization_boundary.as_str()
            )));
        }
        Ok(())
    }
}

fn vertical_trace_receipt_from_row(
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

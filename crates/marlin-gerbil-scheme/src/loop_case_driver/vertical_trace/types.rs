//! Rust-owned vertical trace validation for Scheme loop case-driver receipts.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::GERBIL_POLICY_MIXIN_STACK_COMPILER_SCHEMA_ID;

use crate::loop_case_driver::{
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverCaseId, GerbilLoopCaseDriverLoopProgramId,
    GerbilLoopCaseDriverProfileRef, GerbilLoopCaseSchemeBoundary,
    GerbilLoopCaseSerializationBoundary,
};

/// Rust-owned view of a Scheme-emitted vertical mainline trace.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilLoopCaseDriverVerticalTraceReceipt {
    pub(super) case_id: GerbilLoopCaseDriverCaseId,
    pub(super) profile_ref: GerbilLoopCaseDriverProfileRef,
    pub(super) compiler_owner: String,
    pub(super) compiler_profile_id: String,
    pub(super) loop_program_id: GerbilLoopCaseDriverLoopProgramId,
    pub(super) capability_tags: Vec<GerbilLoopCaseDriverCapability>,
    pub(super) live_gate_env: String,
    pub(super) live_llm_required: bool,
    pub(super) live_llm_allowed: bool,
    pub(super) live_llm_denial_receipt: String,
    pub(super) llm_repair_intent: String,
    pub(super) session_transform: String,
    pub(super) tool_intent_count: usize,
    pub(super) memory_intent_count: usize,
    pub(super) placement_intent_count: usize,
    pub(super) runtime_handoff_kind: String,
    pub(super) runtime_receipt_kind: String,
    pub(super) derived_session_kind: String,
    pub(super) module_kind: String,
    pub(super) module_user_module: String,
    pub(super) module_selection_tags: Vec<GerbilLoopCaseDriverCapability>,
    pub(super) module_source_ref: String,
    pub(super) module_entrypoint: String,
    pub(super) module_enabled: bool,
    pub(super) resolved_policy_pack_policy_epoch: u64,
    pub(super) loop_program_policy_epoch: u64,
    pub(super) transition_count: usize,
    pub(super) transition_actions: Vec<String>,
    pub(super) transition_events: Vec<String>,
    pub(super) mechanism_policy_count: usize,
    pub(super) mechanism_policy_ids: Vec<String>,
    pub(super) policy_digest_length: usize,
    pub(super) policy_digest_octets: Vec<u8>,
    pub(super) capability_mask: u64,
    pub(super) budget_max_attempts: u64,
    pub(super) budget_max_cost_units: u64,
    pub(super) budget_max_wall_time_ms: u64,
    pub(super) policy_forced_slot_count: usize,
    pub(super) policy_merge_receipt_count: usize,
    pub(super) policy_conflict_merge_receipt_count: usize,
    pub(super) policy_merge_kinds: Vec<String>,
    pub(super) policy_merge_statuses: Vec<String>,
    pub(super) policy_mixin_stack_present: bool,
    pub(super) policy_mixin_stack_receipt_kind: String,
    pub(super) policy_mixin_stack_profile_id: String,
    pub(super) policy_mixin_stack_mixin_count: usize,
    pub(super) policy_mixin_stack_slot_merge_law_count: usize,
    pub(super) policy_mixin_stack_slot_merge_laws: Vec<String>,
    pub(super) policy_mixin_stack_linearization_owner: String,
    pub(super) policy_mixin_stack_slot_merge_owner: String,
    pub(super) scheme_boundary: GerbilLoopCaseSchemeBoundary,
    pub(super) serialization_boundary: GerbilLoopCaseSerializationBoundary,
}

/// Error returned when a Scheme vertical mainline trace cannot be trusted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilLoopCaseDriverVerticalTraceError {
    message: String,
}

impl GerbilLoopCaseDriverVerticalTraceError {
    pub(super) fn new(message: impl Into<String>) -> Self {
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

    pub(super) fn ensure_trusted(
        &self,
        index: usize,
    ) -> Result<(), GerbilLoopCaseDriverVerticalTraceError> {
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

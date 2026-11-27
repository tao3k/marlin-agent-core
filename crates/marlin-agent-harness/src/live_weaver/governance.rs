// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Fail-closed Cedar Governance Gate for live harness scenarios.
//!
//! Compiles Scheme-projected Cedar policies from `scheme/agent-harness/objects.ss`
//! through `marlin_agent_kernel::governance_cedar` and enforces zero-write
//! invariants on every live model tool call (e.g. Search / Helm / ASP / Dusa).

use std::collections::BTreeMap;

use marlin_agent_kernel::{
    CedarContextValue, CedarEntityRef, CedarEvaluationDecision, CedarEvaluationRequest,
    CedarPolicyId, CompiledCedarPolicy, SynthesizedCedarPolicy, compile_cedar_policy,
    evaluate_compiled_cedar_policies,
};

/// One Cedar policy projected across the rooted Scheme C-ABI boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedCedarPolicySpec {
    pub policy_id: String,
    pub source: String,
}

/// Declarative governance policy projected from Scheme `HarnessGovernancePolicy.`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedHarnessGovernance {
    pub policy_id: String,
    pub write_allowed: bool,
    pub allowed_capabilities: Vec<String>,
    pub denied_capabilities: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub denied_tools: Vec<String>,
    pub cedar_policies: Vec<ProjectedCedarPolicySpec>,
    pub denial_message: String,
}

/// Typed, auditable governance decision receipt recorded for every tool call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceDecisionReceipt {
    pub call_id: String,
    pub tool_name: String,
    pub capability: String,
    pub is_write: bool,
    pub permitted: bool,
    pub cedar_decision: &'static str,
    pub matched_policy_ids: Vec<String>,
    pub reason: String,
}

/// Compiled fail-closed governance gate combining Scheme POO capability/tool
/// allow/deny rules with compiled Cedar policies.
#[derive(Clone, Debug)]
pub struct CompiledHarnessGovernanceGate {
    policy: ProjectedHarnessGovernance,
    compiled_cedar: Vec<CompiledCedarPolicy>,
}

impl CompiledHarnessGovernanceGate {
    /// Compile all Scheme-projected Cedar policies fail-closed.
    pub fn compile(policy: ProjectedHarnessGovernance) -> Result<Self, String> {
        let mut compiled_cedar = Vec::with_capacity(policy.cedar_policies.len());
        for spec in &policy.cedar_policies {
            let id = CedarPolicyId::new(spec.policy_id.clone())
                .map_err(|e| format!("invalid Cedar policy id `{}`: {e}", spec.policy_id))?;
            let synthesized = SynthesizedCedarPolicy {
                policy_id: id,
                source: spec.source.clone(),
            };
            let compiled = compile_cedar_policy(&synthesized)
                .map_err(|e| format!("failed to compile Cedar policy `{}`: {e}", spec.policy_id))?;
            compiled_cedar.push(compiled);
        }
        Ok(Self {
            policy,
            compiled_cedar,
        })
    }

    #[must_use]
    pub fn policy(&self) -> &ProjectedHarnessGovernance {
        &self.policy
    }

    /// Authorize or deny one model tool call, returning an auditable
    /// [`GovernanceDecisionReceipt`] and emitting a `tracing` span.
    #[must_use]
    pub fn evaluate_tool_call(
        &self,
        scenario_id: &str,
        call_id: &str,
        tool_name: &str,
        tool_capability: Option<&str>,
    ) -> GovernanceDecisionReceipt {
        let capability = classify_capability(tool_name, tool_capability);
        let is_write = is_write_capable(tool_name, &capability);

        let span = tracing::info_span!(
            "harness.governance.evaluate",
            scenario.id = scenario_id,
            governance.policy_id = self.policy.policy_id.as_str(),
            tool.call_id = call_id,
            tool.name = tool_name,
            tool.capability = capability.as_str(),
            tool.is_write = is_write,
            governance.decision = tracing::field::Empty,
        );
        let _entered = span.enter();

        // Fail-closed Layer 1: explicit write prohibition & denied tools/capabilities.
        if !self.policy.write_allowed && is_write {
            let cedar_receipt = self.evaluate_cedar(scenario_id, tool_name, &capability, true);
            span.record("governance.decision", "deny_write");
            return GovernanceDecisionReceipt {
                call_id: call_id.to_owned(),
                tool_name: tool_name.to_owned(),
                capability,
                is_write: true,
                permitted: false,
                cedar_decision: cedar_receipt.0,
                matched_policy_ids: cedar_receipt.1,
                reason: self.policy.denial_message.clone(),
            };
        }

        if self.policy.denied_tools.iter().any(|t| t == tool_name)
            || self
                .policy
                .denied_capabilities
                .iter()
                .any(|c| c == &capability)
        {
            let cedar_receipt = self.evaluate_cedar(scenario_id, tool_name, &capability, is_write);
            span.record("governance.decision", "deny_policy");
            return GovernanceDecisionReceipt {
                call_id: call_id.to_owned(),
                tool_name: tool_name.to_owned(),
                capability,
                is_write,
                permitted: false,
                cedar_decision: cedar_receipt.0,
                matched_policy_ids: cedar_receipt.1,
                reason: self.policy.denial_message.clone(),
            };
        }

        // Fail-closed Layer 2: explicit allowlists when declared.
        if !self.policy.allowed_tools.is_empty()
            && !self.policy.allowed_tools.iter().any(|t| t == tool_name)
        {
            span.record("governance.decision", "deny_unlisted_tool");
            return GovernanceDecisionReceipt {
                call_id: call_id.to_owned(),
                tool_name: tool_name.to_owned(),
                capability,
                is_write,
                permitted: false,
                cedar_decision: "ImplicitDeny",
                matched_policy_ids: Vec::new(),
                reason: format!(
                    "Governance policy `{}` only permits tools {:?}; `{tool_name}` denied.",
                    self.policy.policy_id, self.policy.allowed_tools
                ),
            };
        }

        if !self.policy.allowed_capabilities.is_empty()
            && !self
                .policy
                .allowed_capabilities
                .iter()
                .any(|c| c == &capability)
        {
            span.record("governance.decision", "deny_unlisted_capability");
            return GovernanceDecisionReceipt {
                call_id: call_id.to_owned(),
                tool_name: tool_name.to_owned(),
                capability: capability.clone(),
                is_write,
                permitted: false,
                cedar_decision: "ImplicitDeny",
                matched_policy_ids: Vec::new(),
                reason: format!(
                    "Governance policy `{}` only permits capabilities {:?}; `{capability}` denied.",
                    self.policy.policy_id, self.policy.allowed_capabilities
                ),
            };
        }

        // Fail-closed Layer 3: authoritative Cedar policy evaluation.
        let (cedar_label, matched_ids, permitted) =
            self.evaluate_cedar(scenario_id, tool_name, &capability, is_write);
        span.record(
            "governance.decision",
            if permitted { "permit" } else { "forbid_cedar" },
        );
        GovernanceDecisionReceipt {
            call_id: call_id.to_owned(),
            tool_name: tool_name.to_owned(),
            capability,
            is_write,
            permitted,
            cedar_decision: cedar_label,
            matched_policy_ids: matched_ids,
            reason: if permitted {
                format!(
                    "Permitted by Cedar policy `{}` (read-only search capability).",
                    self.policy.policy_id
                )
            } else {
                self.policy.denial_message.clone()
            },
        }
    }

    fn evaluate_cedar(
        &self,
        scenario_id: &str,
        tool_name: &str,
        capability: &str,
        is_write: bool,
    ) -> (&'static str, Vec<String>, bool) {
        if self.compiled_cedar.is_empty() {
            return ("ImplicitDeny", Vec::new(), false);
        }
        let mut context = BTreeMap::new();
        context.insert("is_write".to_owned(), CedarContextValue::Bool(is_write));
        context.insert(
            "tool_name".to_owned(),
            CedarContextValue::Str(tool_name.to_owned()),
        );
        context.insert(
            "scenario_id".to_owned(),
            CedarContextValue::Str(scenario_id.to_owned()),
        );
        let req = CedarEvaluationRequest {
            principal: CedarEntityRef::actor(format!("harness:{scenario_id}")),
            principal_roles: vec![CedarEntityRef::role("search_eval_agent")],
            action: CedarEntityRef::action(format!("tool.{capability}")),
            resource: CedarEntityRef::resource(tool_name),
            resource_groups: vec![CedarEntityRef::scope("workspace")],
            context,
        };
        match evaluate_compiled_cedar_policies(&self.compiled_cedar, &req) {
            CedarEvaluationDecision::Allow { matching_permits } => {
                let ids = matching_permits
                    .into_iter()
                    .map(|id| id.as_str().to_owned())
                    .collect();
                ("Allow", ids, true)
            }
            CedarEvaluationDecision::Deny { matching_forbids } => {
                let ids = matching_forbids
                    .into_iter()
                    .map(|id| id.as_str().to_owned())
                    .collect();
                ("Forbid", ids, false)
            }
            CedarEvaluationDecision::ImplicitDeny => ("ImplicitDeny", Vec::new(), false),
        }
    }
}

fn classify_capability(tool_name: &str, declared_capability: Option<&str>) -> String {
    if let Some(cap) = declared_capability {
        return cap.to_ascii_lowercase();
    }
    let lower = tool_name.to_ascii_lowercase();
    if lower.contains("search") {
        "search".to_owned()
    } else if lower == "bash" || lower.contains("shell") || lower.contains("exec") {
        "shell".to_owned()
    } else {
        // Fail-closed: any write/edit/delete or unclassified tool defaults to `write`.
        "write".to_owned()
    }
}

fn extract_read_file_target(raw_cmd: &str) -> Option<&str> {
    let mut tokens = raw_cmd.split_whitespace();
    while let Some(tok) = tokens.next() {
        if matches!(tok, "cat" | "head" | "tail" | "sed" | "nl" | "less") {
            for next in tokens.by_ref() {
                if next.starts_with('-') || matches!(next, "|" | "&&" | ";" | "2>/dev/null") {
                    continue;
                }
                let cleaned = next.trim_matches(|c| c == '"' || c == '\'');
                if cleaned.contains('.') || cleaned.contains('/') {
                    return Some(cleaned);
                }
            }
        }
    }
    None
}

/// Enrich a fail-closed Cedar governance denial with a concrete, zero-latency
/// read-only L0/L1 replacement query so LLMs recover immediately without stalling.
#[must_use]
pub fn format_actionable_denial_message(
    base_reason: &str,
    tool_name: &str,
    raw_argument: &str,
) -> String {
    if !tool_name.eq_ignore_ascii_case("bash") {
        return base_reason.to_owned();
    }
    let lower = raw_argument.to_ascii_lowercase();
    let replacement = if let Some(path) = extract_read_file_target(raw_argument) {
        format!(
            "workspace_search: (read-file \"{path}\" start: 1 end: 120) or (file-outline \"{path}\")"
        )
    } else if lower.contains("find ")
        || lower.starts_with("ls")
        || lower.contains(" ls ")
        || lower.contains("tree")
    {
        "workspace_search: (file-outline \"crates/\" lang: \"rust\" kind: \"struct\" exclude-tests?: #t limit: 80) or (brute-search \"\" path-prefix: \"crates/\" limit: 40)"
            .to_owned()
    } else if lower.contains("rg ") || lower.contains("grep ") {
        "workspace_search: (brute-search \"<pattern>\" exclude-tests?: #t limit: 40) or structural_query: (find-symbols name-contains: \"<stem>\" lang: \"any\" exclude-tests?: #t)"
            .to_owned()
    } else {
        "workspace_search: (brute-search \"...\" exclude-tests?: #t) / (file-outline \"...\") / (read-file \"...\" symbol: \"...\") or structural_query: (find-impact \"...\")"
            .to_owned()
    };
    format!("{base_reason}\nsuggested_replacement: {replacement}")
}

fn is_write_capable(tool_name: &str, capability: &str) -> bool {
    if matches!(capability, "write" | "edit" | "mutation" | "shell") {
        return true;
    }
    let lower = tool_name.to_ascii_lowercase();
    lower == "bash"
        || lower.contains("write")
        || lower.contains("edit")
        || lower.contains("patch")
        || lower.contains("delete")
        || lower.contains("mutate")
}

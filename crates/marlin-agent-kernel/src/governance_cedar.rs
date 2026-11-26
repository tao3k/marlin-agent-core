// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Zero-dependency Cedar policy compiler & authoritative fail-closed evaluator for
//! policies dynamically synthesized by Gerbil Scheme (`scheme/policy/governance.ss`).
//!
//! Enforces true Cedar authorization semantics:
//! - Structured parsing of `permit` / `forbid` scopes (`principal`, `action`, `resource`)
//!   with `==`, `in`, set membership `in [...]`, and `when { context.<k> == <v> (&& ...)* }`.
//! - Fail-closed rejection on malformed policy syntax (no substring heuristics).
//! - Deterministic Cedar precedence: **any matching `forbid` overrides every `permit`**;
//!   otherwise a matching `permit` yields `Allow`, and no match yields `ImplicitDeny`.

use std::collections::BTreeMap;

use crate::governance_admission::{
    CedarPolicyId, GovernedSubjectName, GovernanceAdmissionError, SynthesizedCedarPolicy,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CedarEffect {
    Permit,
    Forbid,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CedarEntityRef {
    pub entity_type: String,
    pub entity_id: String,
}

impl CedarEntityRef {
    pub fn new(entity_type: impl Into<String>, entity_id: impl Into<String>) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
        }
    }

    pub fn actor(id: impl Into<String>) -> Self {
        Self::new("Marlin::Actor", id)
    }

    pub fn role(id: impl Into<String>) -> Self {
        Self::new("Marlin::Role", id)
    }

    pub fn action(id: impl Into<String>) -> Self {
        Self::new("Marlin::Action", id)
    }

    pub fn resource(id: impl Into<String>) -> Self {
        Self::new("Marlin::Resource", id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CedarScopeMatcher {
    Any,
    Eq(CedarEntityRef),
    In(CedarEntityRef),
    InSet(Vec<CedarEntityRef>),
}

impl CedarScopeMatcher {
    fn matches(&self, target: &CedarEntityRef, groups: &[CedarEntityRef]) -> bool {
        match self {
            Self::Any => true,
            Self::Eq(expected) => target == expected,
            Self::In(group) => target == group || groups.iter().any(|g| g == group),
            Self::InSet(set) => set
                .iter()
                .any(|item| target == item || groups.iter().any(|g| g == item)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CedarContextValue {
    Bool(bool),
    Str(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CedarWhenCondition {
    pub key: String,
    pub expected: CedarContextValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledCedarPolicy {
    pub policy_id: CedarPolicyId,
    pub effect: CedarEffect,
    pub principal: CedarScopeMatcher,
    pub action: CedarScopeMatcher,
    pub resource: CedarScopeMatcher,
    pub when_clauses: Vec<CedarWhenCondition>,
}

impl CompiledCedarPolicy {
    pub fn matches(&self, req: &CedarEvaluationRequest) -> bool {
        if !self.principal.matches(&req.principal, &req.principal_roles) {
            return false;
        }
        if !self.action.matches(&req.action, &[]) {
            return false;
        }
        if !self.resource.matches(&req.resource, &req.resource_groups) {
            return false;
        }
        self.when_clauses
            .iter()
            .all(|clause| req.context.get(&clause.key) == Some(&clause.expected))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CedarEvaluationRequest {
    pub principal: CedarEntityRef,
    pub principal_roles: Vec<CedarEntityRef>,
    pub action: CedarEntityRef,
    pub resource: CedarEntityRef,
    pub resource_groups: Vec<CedarEntityRef>,
    pub context: BTreeMap<String, CedarContextValue>,
}

impl CedarEvaluationRequest {
    pub fn for_side_effect(
        actor: &GovernedSubjectName,
        tool: &GovernedSubjectName,
        resource: &GovernedSubjectName,
        tainted: bool,
        declassified: bool,
    ) -> Self {
        let mut context = BTreeMap::new();
        context.insert("tainted".to_owned(), CedarContextValue::Bool(tainted));
        context.insert(
            "declassified".to_owned(),
            CedarContextValue::Bool(declassified),
        );
        context.insert("step_up_fresh".to_owned(), CedarContextValue::Bool(false));
        Self {
            principal: CedarEntityRef::actor(actor.as_str()),
            principal_roles: infer_actor_roles(actor.as_str()),
            action: CedarEntityRef::action(tool.as_str()),
            resource: CedarEntityRef::resource(resource.as_str()),
            resource_groups: Vec::new(),
            context,
        }
    }
}

/// Classifies agent-like runtime identities into `Marlin::Role::"Agent"` so baseline
/// Cedar role `forbid` rules automatically prevent privilege escalation.
pub fn infer_actor_roles(actor: &str) -> Vec<CedarEntityRef> {
    let lower = actor.trim().to_ascii_lowercase();
    if lower.starts_with("agent")
        || lower.starts_with("bot")
        || lower.starts_with("llm")
        || lower.starts_with("worker")
        || lower.starts_with("scheme-sandbox")
    {
        vec![CedarEntityRef::role("Agent")]
    } else {
        Vec::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CedarEvaluationDecision {
    Allow { matching_permits: Vec<CedarPolicyId> },
    Deny { matching_forbids: Vec<CedarPolicyId> },
    ImplicitDeny,
}

/// Evaluates compiled Cedar policies under strict Cedar semantics:
/// any matching `forbid` overrides every `permit`.
pub fn evaluate_compiled_cedar_policies(
    policies: &[CompiledCedarPolicy],
    req: &CedarEvaluationRequest,
) -> CedarEvaluationDecision {
    let mut forbids = Vec::new();
    let mut permits = Vec::new();
    for policy in policies {
        if policy.matches(req) {
            match policy.effect {
                CedarEffect::Forbid => forbids.push(policy.policy_id.clone()),
                CedarEffect::Permit => permits.push(policy.policy_id.clone()),
            }
        }
    }
    if !forbids.is_empty() {
        CedarEvaluationDecision::Deny {
            matching_forbids: forbids,
        }
    } else if !permits.is_empty() {
        CedarEvaluationDecision::Allow {
            matching_permits: permits,
        }
    } else {
        CedarEvaluationDecision::ImplicitDeny
    }
}

/// Compiles one Scheme-synthesized Cedar policy into a verified `CompiledCedarPolicy`.
/// Any syntax error, trailing token, or malformed entity ref fails closed.
pub fn compile_cedar_policy(
    policy: &SynthesizedCedarPolicy,
) -> Result<CompiledCedarPolicy, GovernanceAdmissionError> {
    let mut parser = CedarParser::new(&policy.source);
    parser
        .parse_policy(&policy.policy_id)
        .map_err(|detail| GovernanceAdmissionError::InvalidCedarPolicy {
            policy_id: policy.policy_id.clone(),
            detail,
        })
}

struct CedarParser<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> CedarParser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() {
            if self.bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            } else if self.bytes[self.pos..].starts_with(b"//") {
                while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn consume_literal(&mut self, lit: &str) -> bool {
        self.skip_ws();
        if self.src[self.pos..].starts_with(lit) {
            self.pos += lit.len();
            true
        } else {
            false
        }
    }

    fn expect_literal(&mut self, lit: &str) -> Result<(), String> {
        if self.consume_literal(lit) {
            Ok(())
        } else {
            Err(format!("expected `{lit}` at byte {}", self.pos))
        }
    }

    fn peek_ident(&mut self) -> Option<&'a str> {
        self.skip_ws();
        let start = self.pos;
        if start >= self.bytes.len() {
            return None;
        }
        let b0 = self.bytes[start];
        if !(b0.is_ascii_alphabetic() || b0 == b'_') {
            return None;
        }
        let mut end = start + 1;
        while end < self.bytes.len() {
            let b = self.bytes[end];
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
                end += 1;
            } else {
                break;
            }
        }
        Some(&self.src[start..end])
    }

    fn read_ident(&mut self) -> Result<&'a str, String> {
        let Some(ident) = self.peek_ident() else {
            return Err(format!("expected identifier at byte {}", self.pos));
        };
        self.pos += ident.len();
        Ok(ident)
    }

    fn read_quoted_string(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.bytes.get(self.pos) != Some(&b'"') {
            return Err(format!("expected quoted string at byte {}", self.pos));
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let ch = self.bytes[self.pos];
            self.pos += 1;
            match ch {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(&esc) = self.bytes.get(self.pos) else {
                        return Err("unterminated escape sequence".to_owned());
                    };
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        other => return Err(format!("unsupported escape \\{}", other as char)),
                    }
                }
                _ => {
                    let start = self.pos - 1;
                    let mut end = self.pos;
                    while end < self.bytes.len()
                        && self.bytes[end] != b'"'
                        && self.bytes[end] != b'\\'
                    {
                        end += 1;
                    }
                    let chunk = std::str::from_utf8(&self.bytes[start..end])
                        .map_err(|e| format!("invalid utf8 in Cedar string: {e}"))?;
                    out.push_str(chunk);
                    self.pos = end;
                }
            }
        }
        Err("unterminated Cedar string literal".to_owned())
    }

    fn read_entity_ref(&mut self) -> Result<CedarEntityRef, String> {
        let mut segments = Vec::new();
        segments.push(self.read_ident()?.to_owned());
        loop {
            self.expect_literal("::")?;
            self.skip_ws();
            if self.bytes.get(self.pos) == Some(&b'"') {
                let entity_id = self.read_quoted_string()?;
                if entity_id.is_empty() {
                    return Err("empty Cedar entity identifier".to_owned());
                }
                return Ok(CedarEntityRef {
                    entity_type: segments.join("::"),
                    entity_id,
                });
            }
            segments.push(self.read_ident()?.to_owned());
        }
    }

    fn parse_scope_clause(&mut self, expected_var: &str) -> Result<CedarScopeMatcher, String> {
        let var = self.read_ident()?;
        if var != expected_var {
            return Err(format!("expected scope variable `{expected_var}`, found `{var}`"));
        }
        if self.consume_literal("==") {
            return Ok(CedarScopeMatcher::Eq(self.read_entity_ref()?));
        }
        if self.peek_ident() == Some("in") {
            let _ = self.read_ident()?;
            if self.consume_literal("[") {
                let mut set = Vec::new();
                if !self.consume_literal("]") {
                    loop {
                        set.push(self.read_entity_ref()?);
                        if self.consume_literal(",") {
                            if self.consume_literal("]") {
                                break;
                            }
                            continue;
                        }
                        self.expect_literal("]")?;
                        break;
                    }
                }
                if set.is_empty() {
                    return Err("empty Cedar `in [...]` entity set".to_owned());
                }
                return Ok(CedarScopeMatcher::InSet(set));
            }
            return Ok(CedarScopeMatcher::In(self.read_entity_ref()?));
        }
        Ok(CedarScopeMatcher::Any)
    }

    fn parse_when_clause(&mut self) -> Result<Vec<CedarWhenCondition>, String> {
        self.expect_literal("{")?;
        let mut conditions = Vec::new();
        loop {
            let root = self.read_ident()?;
            if root != "context" {
                return Err(format!(
                    "unsupported Cedar condition root `{root}` (expected `context`)"
                ));
            }
            self.expect_literal(".")?;
            let key = self.read_ident()?.to_owned();
            self.expect_literal("==")?;
            self.skip_ws();
            let expected = if self.bytes.get(self.pos) == Some(&b'"') {
                CedarContextValue::Str(self.read_quoted_string()?)
            } else {
                match self.read_ident()? {
                    "true" => CedarContextValue::Bool(true),
                    "false" => CedarContextValue::Bool(false),
                    other => {
                        return Err(format!(
                            "unsupported Cedar literal `{other}` in when clause"
                        ));
                    }
                }
            };
            conditions.push(CedarWhenCondition { key, expected });
            if !self.consume_literal("&&") {
                break;
            }
        }
        self.expect_literal("}")?;
        Ok(conditions)
    }

    fn parse_policy(&mut self, policy_id: &CedarPolicyId) -> Result<CompiledCedarPolicy, String> {
        let effect = match self.read_ident()? {
            "permit" => CedarEffect::Permit,
            "forbid" => CedarEffect::Forbid,
            other => return Err(format!("expected `permit` or `forbid`, found `{other}`")),
        };
        self.expect_literal("(")?;
        let principal = self.parse_scope_clause("principal")?;
        self.expect_literal(",")?;
        let action = self.parse_scope_clause("action")?;
        self.expect_literal(",")?;
        let resource = self.parse_scope_clause("resource")?;
        self.expect_literal(")")?;

        let mut when_clauses = Vec::new();
        while self.peek_ident() == Some("when") {
            let _ = self.read_ident()?;
            when_clauses.extend(self.parse_when_clause()?);
        }
        self.expect_literal(";")?;
        self.skip_ws();
        if self.pos != self.bytes.len() {
            return Err(format!("unexpected trailing tokens at byte {}", self.pos));
        }
        Ok(CompiledCedarPolicy {
            policy_id: policy_id.clone(),
            effect,
            principal,
            action,
            resource,
            when_clauses,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_policy(id: &str, src: &str) -> CompiledCedarPolicy {
        compile_cedar_policy(&SynthesizedCedarPolicy {
            policy_id: CedarPolicyId::new(id).unwrap(),
            source: src.to_owned(),
        })
        .expect("valid Cedar policy")
    }

    #[test]
    fn compiles_and_evaluates_scheme_synthesized_cedar_policies_with_forbid_override() {
        let policies = vec![
            mk_policy(
                "marlin.forbid.agent-privileged-governance",
                "forbid (\n  principal in Marlin::Role::\"Agent\",\n  action in [\n    Marlin::Action::\"rearm_breaker\",\n    Marlin::Action::\"promote_context_scope\",\n    Marlin::Action::\"resolve_human_gate\",\n    Marlin::Action::\"unblock_matter\"\n  ],\n  resource\n);",
            ),
            mk_policy(
                "marlin.permit.verified-human-governance",
                "permit (\n  principal in Marlin::Role::\"VerifiedHuman\",\n  action in [\n    Marlin::Action::\"rearm_breaker\",\n    Marlin::Action::\"promote_context_scope\",\n    Marlin::Action::\"resolve_human_gate\"\n  ],\n  resource\n) when { context.step_up_fresh == true };",
            ),
            mk_policy(
                "marlin.ascent.forbid.fenced.ops-compromised",
                "forbid (principal == Marlin::Actor::\"ops-compromised\", action, resource);",
            ),
            mk_policy(
                "marlin.ascent.forbid.exfil.secret-doc.ext-webhook",
                "forbid (principal, action, resource == Marlin::Resource::\"ext-webhook\");",
            ),
        ];

        // 1. Agent attempting privileged rearm_breaker -> DENY by role forbid.
        let agent_req = CedarEvaluationRequest {
            principal: CedarEntityRef::actor("agent-worker"),
            principal_roles: vec![CedarEntityRef::role("Agent")],
            action: CedarEntityRef::action("rearm_breaker"),
            resource: CedarEntityRef::resource("workspace"),
            resource_groups: Vec::new(),
            context: BTreeMap::from([(
                "step_up_fresh".to_owned(),
                CedarContextValue::Bool(true),
            )]),
        };
        assert_eq!(
            evaluate_compiled_cedar_policies(&policies, &agent_req),
            CedarEvaluationDecision::Deny {
                matching_forbids: vec![
                    CedarPolicyId::new("marlin.forbid.agent-privileged-governance").unwrap()
                ],
            }
        );

        // 2. VerifiedHuman WITHOUT fresh step-up -> IMPLICIT DENY.
        let human_stale = CedarEvaluationRequest {
            principal: CedarEntityRef::actor("ops-alice"),
            principal_roles: vec![CedarEntityRef::role("VerifiedHuman")],
            action: CedarEntityRef::action("rearm_breaker"),
            resource: CedarEntityRef::resource("workspace"),
            resource_groups: Vec::new(),
            context: BTreeMap::from([(
                "step_up_fresh".to_owned(),
                CedarContextValue::Bool(false),
            )]),
        };
        assert_eq!(
            evaluate_compiled_cedar_policies(&policies, &human_stale),
            CedarEvaluationDecision::ImplicitDeny
        );

        // 3. VerifiedHuman WITH fresh step-up -> ALLOW.
        let mut human_fresh = human_stale.clone();
        human_fresh
            .context
            .insert("step_up_fresh".to_owned(), CedarContextValue::Bool(true));
        assert_eq!(
            evaluate_compiled_cedar_policies(&policies, &human_fresh),
            CedarEvaluationDecision::Allow {
                matching_permits: vec![
                    CedarPolicyId::new("marlin.permit.verified-human-governance").unwrap()
                ],
            }
        );

        // 4. VerifiedHuman who was ASCENT-fenced (`ops-compromised`) -> FORBID OVERRIDES PERMIT!
        let mut fenced_human = human_fresh.clone();
        fenced_human.principal = CedarEntityRef::actor("ops-compromised");
        assert_eq!(
            evaluate_compiled_cedar_policies(&policies, &fenced_human),
            CedarEvaluationDecision::Deny {
                matching_forbids: vec![
                    CedarPolicyId::new("marlin.ascent.forbid.fenced.ops-compromised").unwrap()
                ],
            }
        );

        // 5. Resource-scoped exfiltration forbid blocks ANY actor targeting `ext-webhook`.
        let mut exfil_req = human_fresh;
        exfil_req.resource = CedarEntityRef::resource("ext-webhook");
        assert_eq!(
            evaluate_compiled_cedar_policies(&policies, &exfil_req),
            CedarEvaluationDecision::Deny {
                matching_forbids: vec![
                    CedarPolicyId::new("marlin.ascent.forbid.exfil.secret-doc.ext-webhook")
                        .unwrap()
                ],
            }
        );
    }

    #[test]
    fn malformed_cedar_policy_fails_closed() {
        for bad in [
            "allow (principal, action, resource);",
            "forbid (principal == Marlin::Actor::\"a\", action);",
            "forbid (principal == Marlin::Actor::\"a\", action, resource)",
            "forbid (principal == Marlin::Actor::\"a\", action, resource); trailing",
            "forbid (principal == Marlin::Actor::\"\", action, resource);",
        ] {
            let err = compile_cedar_policy(&SynthesizedCedarPolicy {
                policy_id: CedarPolicyId::new("bad.pol").unwrap(),
                source: bad.to_owned(),
            })
            .expect_err("malformed Cedar policy must fail closed");
            assert!(matches!(
                err,
                GovernanceAdmissionError::InvalidCedarPolicy { .. }
            ));
        }
    }
}

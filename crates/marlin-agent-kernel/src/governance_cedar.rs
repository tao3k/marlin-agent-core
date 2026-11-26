// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Zero-dependency Cedar policy compiler & authoritative fail-closed evaluator for
//! policies dynamically synthesized by Gerbil Scheme (`scheme/policy/governance.ss`).
//!
//! Enforces true Cedar authorization semantics:
//! - Structured parsing of `permit` / `forbid` scopes (`principal`, `action`, `resource`)
//!   with `==`, `in`, set membership `in [...]`, and `when { ... }` / `unless { ... }`
//!   condition blocks supporting `==`, `!=`, bare boolean `context.<k>`, and `!context.<k>`.
//! - Fail-closed rejection on malformed policy syntax or missing context keys in conditions.
//! - Deterministic Cedar precedence: **any matching `forbid` overrides every `permit`**;
//!   otherwise a matching `permit` yields `Allow`, and no match yields `ImplicitDeny`.

use std::collections::BTreeMap;

use crate::governance_admission::{
    CedarPolicyId, GovernanceAdmissionError, GovernedSubjectName, SynthesizedCedarPolicy,
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

    pub fn scope(id: impl Into<String>) -> Self {
        Self::new("Marlin::Scope", id)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CedarConditionOp {
    Eq,
    NotEq,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CedarWhenCondition {
    pub key: String,
    pub op: CedarConditionOp,
    pub expected: CedarContextValue,
}

impl CedarWhenCondition {
    /// Evaluates one condition predicate against `context`.
    /// Missing context keys return `None` so both `when` and `unless` fail closed.
    fn eval(&self, context: &BTreeMap<String, CedarContextValue>) -> Option<bool> {
        let actual = context.get(&self.key)?;
        Some(match self.op {
            CedarConditionOp::Eq => actual == &self.expected,
            CedarConditionOp::NotEq => actual != &self.expected,
        })
    }
}

fn eval_condition_block(
    conditions: &[CedarWhenCondition],
    context: &BTreeMap<String, CedarContextValue>,
) -> Option<bool> {
    let mut all_true = true;
    for cond in conditions {
        if !cond.eval(context)? {
            all_true = false;
        }
    }
    Some(all_true)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledCedarPolicy {
    pub policy_id: CedarPolicyId,
    pub effect: CedarEffect,
    pub principal: CedarScopeMatcher,
    pub action: CedarScopeMatcher,
    pub resource: CedarScopeMatcher,
    pub when_clauses: Vec<CedarWhenCondition>,
    pub unless_clauses: Vec<Vec<CedarWhenCondition>>,
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
        if !self.when_clauses.is_empty()
            && eval_condition_block(&self.when_clauses, &req.context) != Some(true)
        {
            return false;
        }
        for block in &self.unless_clauses {
            if eval_condition_block(block, &req.context) != Some(false) {
                return false;
            }
        }
        true
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
    pub fn with_resource_scope(mut self, scope: impl Into<String>) -> Self {
        let group = CedarEntityRef::scope(scope);
        if !self.resource_groups.contains(&group) {
            self.resource_groups.push(group);
        }
        self
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
    Allow {
        matching_permits: Vec<CedarPolicyId>,
    },
    Deny {
        matching_forbids: Vec<CedarPolicyId>,
    },
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
    parser.parse_policy(&policy.policy_id).map_err(|detail| {
        GovernanceAdmissionError::InvalidCedarPolicy {
            policy_id: policy.policy_id.clone(),
            detail,
        }
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
            return Err(format!(
                "expected scope variable `{expected_var}`, found `{var}`"
            ));
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

    fn parse_condition_block(&mut self) -> Result<Vec<CedarWhenCondition>, String> {
        self.expect_literal("{")?;
        let mut conditions = Vec::new();
        loop {
            let negated = self.consume_literal("!");
            let root = self.read_ident()?;
            if root != "context" {
                return Err(format!(
                    "unsupported Cedar condition root `{root}` (expected `context`)"
                ));
            }
            self.expect_literal(".")?;
            let key = self.read_ident()?.to_owned();
            let (op, expected) = if negated {
                (CedarConditionOp::Eq, CedarContextValue::Bool(false))
            } else if self.consume_literal("==") {
                (CedarConditionOp::Eq, self.read_context_literal()?)
            } else if self.consume_literal("!=") {
                (CedarConditionOp::NotEq, self.read_context_literal()?)
            } else {
                (CedarConditionOp::Eq, CedarContextValue::Bool(true))
            };
            conditions.push(CedarWhenCondition { key, op, expected });
            if !self.consume_literal("&&") {
                break;
            }
        }
        self.expect_literal("}")?;
        Ok(conditions)
    }

    fn read_context_literal(&mut self) -> Result<CedarContextValue, String> {
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b'"') {
            Ok(CedarContextValue::Str(self.read_quoted_string()?))
        } else {
            match self.read_ident()? {
                "true" => Ok(CedarContextValue::Bool(true)),
                "false" => Ok(CedarContextValue::Bool(false)),
                other => Err(format!(
                    "unsupported Cedar literal `{other}` in condition block"
                )),
            }
        }
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
        let mut unless_clauses = Vec::new();
        loop {
            match self.peek_ident() {
                Some("when") => {
                    let _ = self.read_ident()?;
                    when_clauses.extend(self.parse_condition_block()?);
                }
                Some("unless") => {
                    let _ = self.read_ident()?;
                    unless_clauses.push(self.parse_condition_block()?);
                }
                _ => break,
            }
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
            unless_clauses,
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
            context: BTreeMap::from([("step_up_fresh".to_owned(), CedarContextValue::Bool(true))]),
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
            context: BTreeMap::from([("step_up_fresh".to_owned(), CedarContextValue::Bool(false))]),
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
    fn supports_unless_not_equal_boolean_predicates_and_scope_groups_fail_closed() {
        let policies = vec![
            mk_policy(
                "marlin.permit.ops-export",
                "permit (principal in Marlin::Role::\"VerifiedHuman\", action == Marlin::Action::\"export\", resource) when { context.step_up_fresh && !context.tainted && context.channel_scope != \"untrusted\" };",
            ),
            mk_policy(
                "marlin.forbid.external-scope-unless-declassified",
                "forbid (principal, action, resource in Marlin::Scope::\"external\") unless { context.declassified == true };",
            ),
        ];

        let mut req = CedarEvaluationRequest {
            principal: CedarEntityRef::actor("ops-alice"),
            principal_roles: vec![CedarEntityRef::role("VerifiedHuman")],
            action: CedarEntityRef::action("export"),
            resource: CedarEntityRef::resource("ext-webhook"),
            resource_groups: vec![CedarEntityRef::scope("external")],
            context: BTreeMap::from([
                ("step_up_fresh".to_owned(), CedarContextValue::Bool(true)),
                ("tainted".to_owned(), CedarContextValue::Bool(false)),
                ("declassified".to_owned(), CedarContextValue::Bool(false)),
                (
                    "channel_scope".to_owned(),
                    CedarContextValue::Str("workspace".to_owned()),
                ),
            ]),
        };

        // 1. Not declassified on external scope -> forbid overrides permit!
        assert_eq!(
            evaluate_compiled_cedar_policies(&policies, &req),
            CedarEvaluationDecision::Deny {
                matching_forbids: vec![
                    CedarPolicyId::new("marlin.forbid.external-scope-unless-declassified").unwrap()
                ],
            }
        );

        // 2. Declassified == true -> unless suppresses forbid -> permit allows!
        req.context
            .insert("declassified".to_owned(), CedarContextValue::Bool(true));
        assert_eq!(
            evaluate_compiled_cedar_policies(&policies, &req),
            CedarEvaluationDecision::Allow {
                matching_permits: vec![CedarPolicyId::new("marlin.permit.ops-export").unwrap()],
            }
        );

        // 3. Tainted == true violates `!context.tainted` -> ImplicitDeny!
        req.context
            .insert("tainted".to_owned(), CedarContextValue::Bool(true));
        assert_eq!(
            evaluate_compiled_cedar_policies(&policies, &req),
            CedarEvaluationDecision::ImplicitDeny
        );

        // 4. Untrusted channel violates `!= "untrusted"` -> ImplicitDeny!
        req.context
            .insert("tainted".to_owned(), CedarContextValue::Bool(false));
        req.context.insert(
            "channel_scope".to_owned(),
            CedarContextValue::Str("untrusted".to_owned()),
        );
        assert_eq!(
            evaluate_compiled_cedar_policies(&policies, &req),
            CedarEvaluationDecision::ImplicitDeny
        );

        // 5. Missing context key fails closed (neither `!=` nor `unless` silently matches!).
        req.context.remove("channel_scope");
        assert_eq!(
            evaluate_compiled_cedar_policies(&policies, &req),
            CedarEvaluationDecision::ImplicitDeny
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
            "forbid (principal, action, resource) unless {};",
            "permit (principal, action, resource) when { context.a === true };",
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

    #[test]
    fn adversarial_cedar_mutations_never_panic_and_forbid_always_overrides_permit() {
        let mut state: u64 = 0x5EED_CEDA_1234_5678;
        let mut next_u64 = || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };

        let seeds = [
            "forbid (principal in Marlin::Role::\"Agent\", action in [Marlin::Action::\"rearm_breaker\"], resource);",
            "permit (principal in Marlin::Role::\"VerifiedHuman\", action == Marlin::Action::\"export\", resource) when { context.step_up_fresh && !context.tainted && context.channel_scope != \"untrusted\" };",
            "forbid (principal, action, resource in Marlin::Scope::\"external\") unless { context.declassified == true };",
            "forbid (principal == Marlin::Actor::\"ops-bad\", action, resource);",
        ];
        let pid = CedarPolicyId::new("fuzz.cedar").unwrap();

        // 1. 2,000 deterministic adversarial mutations against the Cedar compiler -> zero panics!
        for i in 0..2000usize {
            let base = seeds[i % seeds.len()].as_bytes();
            let mut mutated = base.to_vec();
            match next_u64() % 5 {
                0 => {
                    let cut = (next_u64() as usize) % (mutated.len() + 1);
                    mutated.truncate(cut);
                }
                1 => {
                    if !mutated.is_empty() {
                        let idx = (next_u64() as usize) % mutated.len();
                        mutated[idx] = (next_u64() & 0x7F) as u8;
                    }
                }
                2 => {
                    let idx = (next_u64() as usize) % (mutated.len() + 1);
                    mutated.insert(idx, b"\"\\{}[]()=!;:\0\n"[(next_u64() as usize) % 14]);
                }
                3 => {
                    mutated.extend_from_slice(b" unless { context.x == ");
                }
                _ => {
                    let len = (next_u64() as usize) % 96;
                    mutated = (0..len).map(|_| (0x20 + (next_u64() % 95)) as u8).collect();
                }
            }
            if let Ok(src) = String::from_utf8(mutated) {
                let _ = compile_cedar_policy(&SynthesizedCedarPolicy {
                    policy_id: pid.clone(),
                    source: src,
                });
            }
        }

        // 2. 512 randomized Cedar policy-set evaluations verifying strict forbid-overrides-permit!
        let compiled_pool = [
            mk_policy("p.permit.human", seeds[1]),
            mk_policy("p.forbid.agent", seeds[0]),
            mk_policy("p.forbid.ext", seeds[2]),
            mk_policy("p.forbid.actor", seeds[3]),
        ];
        let actors = ["ops-alice", "ops-bad", "agent-worker"];
        let actions = ["export", "rearm_breaker", "read"];
        let scopes = ["workspace", "external", "untrusted"];

        for _ in 0..512 {
            let mask = (next_u64() & 0xF) as usize;
            let active: Vec<CompiledCedarPolicy> = compiled_pool
                .iter()
                .enumerate()
                .filter(|(idx, _)| (mask & (1 << idx)) != 0)
                .map(|(_, p)| p.clone())
                .collect();

            let actor = actors[(next_u64() as usize) % actors.len()];
            let mut roles = infer_actor_roles(actor);
            if next_u64() % 2 == 0 {
                roles.push(CedarEntityRef::role("VerifiedHuman"));
            }
            let action = actions[(next_u64() as usize) % actions.len()];
            let scope = scopes[(next_u64() as usize) % scopes.len()];
            let mut context = BTreeMap::new();
            if next_u64() % 3 != 0 {
                context.insert(
                    "step_up_fresh".to_owned(),
                    CedarContextValue::Bool(next_u64() % 2 == 0),
                );
            }
            if next_u64() % 3 != 0 {
                context.insert(
                    "tainted".to_owned(),
                    CedarContextValue::Bool(next_u64() % 2 == 0),
                );
            }
            if next_u64() % 3 != 0 {
                context.insert(
                    "declassified".to_owned(),
                    CedarContextValue::Bool(next_u64() % 2 == 0),
                );
            }
            if next_u64() % 3 != 0 {
                context.insert(
                    "channel_scope".to_owned(),
                    CedarContextValue::Str(scopes[(next_u64() as usize) % scopes.len()].to_owned()),
                );
            }

            let req = CedarEvaluationRequest {
                principal: CedarEntityRef::actor(actor),
                principal_roles: roles,
                action: CedarEntityRef::action(action),
                resource: CedarEntityRef::resource("res-1"),
                resource_groups: vec![CedarEntityRef::scope(scope)],
                context,
            };

            let any_forbid = active
                .iter()
                .any(|p| p.effect == CedarEffect::Forbid && p.matches(&req));
            let any_permit = active
                .iter()
                .any(|p| p.effect == CedarEffect::Permit && p.matches(&req));

            let decision = evaluate_compiled_cedar_policies(&active, &req);
            if any_forbid {
                assert!(
                    matches!(decision, CedarEvaluationDecision::Deny { .. }),
                    "matching forbid MUST always yield Deny, got {decision:?}"
                );
            } else if any_permit {
                assert!(
                    matches!(decision, CedarEvaluationDecision::Allow { .. }),
                    "matching permit without forbid MUST yield Allow, got {decision:?}"
                );
            } else {
                assert_eq!(decision, CedarEvaluationDecision::ImplicitDeny);
            }
        }
    }
}

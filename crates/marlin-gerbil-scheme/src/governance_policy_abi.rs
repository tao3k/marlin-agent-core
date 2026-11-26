// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Zero-JSON rooted Gambit Scheme vector C-ABI codec for Marlin's Governance Policy
//! Engine (`scheme/policy/governance.ss`).
//!
//! Encodes 15-slot rooted Scheme request vectors and decodes 14-slot rooted Scheme
//! proof vectors directly across the embedded Gambit heap boundary without text or
//! JSON serialization.

use std::fmt;

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, RootedSchemeCollection, RootedSchemeObject, SchemeScalar,
};

pub const MARLIN_GOVERNANCE_ADMISSION_SCHEMA_VERSION: u32 = 1;
pub const MARLIN_GOVERNANCE_PROJECT_SYMBOL: &str =
    "marlin-governance-project-vector";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernancePolicyAbiError(pub String);

impl fmt::Display for GovernancePolicyAbiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GovernancePolicyAbiError {}

/// One dynamic Bi-Temporal Truth Maintenance derivation node registered at runtime.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GovernanceDerivationSpec {
    pub derivation_id: String,
    pub policy_id: String,
    pub subject_premises: Vec<String>,
    pub parent_derivations: Vec<String>,
}

/// Zero-JSON request payload encoded into a 16-slot rooted Gambit Scheme vector.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GovernancePolicyVectorRequest {
    pub delegation_edges: Vec<(String, String)>,
    pub revoked_actors: Vec<String>,
    pub taint_edges: Vec<(String, String)>,
    pub taint_sources: Vec<String>,
    pub taint_sinks: Vec<String>,
    pub taint_sanitizers: Vec<String>,
    pub context_entries: Vec<(String, String)>,
    pub context_flows: Vec<(String, String)>,
    pub target_scopes: Vec<(String, String)>,
    pub declassified_entries: Vec<String>,
    pub tripped_breakers: Vec<String>,
    pub breaker_tools: Vec<(String, String)>,
    pub retracted_subjects: Vec<String>,
    pub revocation_epoch: u64,
    pub policy_revision: u64,
    pub derivation_specs: Vec<GovernanceDerivationSpec>,
}

/// Decoded 14-slot rooted Gambit Scheme governance proof projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernancePolicyVectorProjection {
    pub schema_version: u32,
    pub admitted: bool,
    pub revocation_epoch: u64,
    pub policy_revision: u64,
    pub assessment_digest: String,
    pub taint_proof_digest: String,
    pub fixpoint_proof_digest: String,
    pub temporal_proof_digest: String,
    pub invalidation_digest: String,
    pub rejection_reasons: Vec<String>,
    pub fenced_actors: Vec<String>,
    pub blocked_tools: Vec<String>,
    pub retracted_conclusions: Vec<String>,
    pub cedar_policies: Vec<(String, String)>,
}

fn encode_str<'vm>(
    runtime: &'vm GerbilRuntime,
    value: &str,
) -> Result<RootedSchemeObject<'vm>, GovernancePolicyAbiError> {
    runtime
        .string_from_utf8(value)
        .map(|s| s.into_object())
        .map_err(|e| GovernancePolicyAbiError(format!("encode string '{value}': {e}")))
}

fn encode_vec<'vm>(
    runtime: &'vm GerbilRuntime,
    items: &[RootedSchemeObject<'vm>],
) -> Result<RootedSchemeObject<'vm>, GovernancePolicyAbiError> {
    let refs: Vec<&RootedSchemeObject<'vm>> = items.iter().collect();
    runtime
        .collection_from_objects(CollectionKind::Vector, &refs)
        .map(|c| c.into_object())
        .map_err(|e| GovernancePolicyAbiError(format!("encode vector: {e}")))
}

fn encode_string_vec<'vm>(
    runtime: &'vm GerbilRuntime,
    items: &[String],
) -> Result<RootedSchemeObject<'vm>, GovernancePolicyAbiError> {
    let mut rooted = Vec::with_capacity(items.len());
    for item in items {
        rooted.push(encode_str(runtime, item)?);
    }
    encode_vec(runtime, &rooted)
}

fn encode_pair_vec<'vm>(
    runtime: &'vm GerbilRuntime,
    items: &[(String, String)],
) -> Result<RootedSchemeObject<'vm>, GovernancePolicyAbiError> {
    let mut rooted = Vec::with_capacity(items.len());
    for (left, right) in items {
        let pair = [encode_str(runtime, left)?, encode_str(runtime, right)?];
        rooted.push(encode_vec(runtime, &pair)?);
    }
    encode_vec(runtime, &rooted)
}

fn encode_derivation_specs<'vm>(
    runtime: &'vm GerbilRuntime,
    items: &[GovernanceDerivationSpec],
) -> Result<RootedSchemeObject<'vm>, GovernancePolicyAbiError> {
    let mut rooted = Vec::with_capacity(items.len());
    for spec in items {
        let entry = [
            encode_str(runtime, &spec.derivation_id)?,
            encode_str(runtime, &spec.policy_id)?,
            encode_string_vec(runtime, &spec.subject_premises)?,
            encode_string_vec(runtime, &spec.parent_derivations)?,
        ];
        rooted.push(encode_vec(runtime, &entry)?);
    }
    encode_vec(runtime, &rooted)
}

/// Encodes `GovernancePolicyVectorRequest` into a 16-slot rooted Gambit heap vector.
pub fn encode_governance_policy_request<'vm>(
    runtime: &'vm GerbilRuntime,
    req: &GovernancePolicyVectorRequest,
) -> Result<RootedSchemeObject<'vm>, GovernancePolicyAbiError> {
    let slots = [
        encode_pair_vec(runtime, &req.delegation_edges)?,
        encode_string_vec(runtime, &req.revoked_actors)?,
        encode_pair_vec(runtime, &req.taint_edges)?,
        encode_string_vec(runtime, &req.taint_sources)?,
        encode_string_vec(runtime, &req.taint_sinks)?,
        encode_string_vec(runtime, &req.taint_sanitizers)?,
        encode_pair_vec(runtime, &req.context_entries)?,
        encode_pair_vec(runtime, &req.context_flows)?,
        encode_pair_vec(runtime, &req.target_scopes)?,
        encode_string_vec(runtime, &req.declassified_entries)?,
        encode_string_vec(runtime, &req.tripped_breakers)?,
        encode_pair_vec(runtime, &req.breaker_tools)?,
        encode_string_vec(runtime, &req.retracted_subjects)?,
        runtime
            .scalar_object(SchemeScalar::Fixnum(req.revocation_epoch as isize))
            .map_err(|e| GovernancePolicyAbiError(format!("encode revocation_epoch: {e}")))?,
        runtime
            .scalar_object(SchemeScalar::Fixnum(req.policy_revision as isize))
            .map_err(|e| GovernancePolicyAbiError(format!("encode policy_revision: {e}")))?,
        encode_derivation_specs(runtime, &req.derivation_specs)?,
    ];
    encode_vec(runtime, &slots)
}

fn read_u64_slot(
    vec: &RootedSchemeCollection<'_>,
    idx: usize,
    label: &str,
) -> Result<u64, GovernancePolicyAbiError> {
    let obj = vec
        .element(idx)
        .map_err(|e| GovernancePolicyAbiError(format!("missing slot {idx} ({label}): {e}")))?;
    match obj.as_ref().scalar() {
        Ok(SchemeScalar::Fixnum(n)) if n >= 0 => Ok(n as u64),
        other => Err(GovernancePolicyAbiError(format!(
            "slot {idx} ({label}) expected non-negative fixnum, got {other:?}"
        ))),
    }
}

fn read_bool_slot(
    vec: &RootedSchemeCollection<'_>,
    idx: usize,
    label: &str,
) -> Result<bool, GovernancePolicyAbiError> {
    let obj = vec
        .element(idx)
        .map_err(|e| GovernancePolicyAbiError(format!("missing slot {idx} ({label}): {e}")))?;
    match obj.as_ref().scalar() {
        Ok(SchemeScalar::Boolean(b)) => Ok(b),
        other => Err(GovernancePolicyAbiError(format!(
            "slot {idx} ({label}) expected boolean, got {other:?}"
        ))),
    }
}

fn read_string_obj(
    obj: RootedSchemeObject<'_>,
    label: &str,
) -> Result<String, GovernancePolicyAbiError> {
    obj.into_string()
        .map_err(|e| GovernancePolicyAbiError(format!("expected string for {label}: {e}")))?
        .to_string()
        .into_result()
        .map_err(|e| GovernancePolicyAbiError(format!("decode utf8 string for {label}: {e}")))
}

fn read_string_slot(
    vec: &RootedSchemeCollection<'_>,
    idx: usize,
    label: &str,
) -> Result<String, GovernancePolicyAbiError> {
    let obj = vec
        .element(idx)
        .map_err(|e| GovernancePolicyAbiError(format!("missing slot {idx} ({label}): {e}")))?;
    read_string_obj(obj, label)
}

fn read_string_vec_slot(
    vec: &RootedSchemeCollection<'_>,
    idx: usize,
    label: &str,
) -> Result<Vec<String>, GovernancePolicyAbiError> {
    let sub = vec
        .element(idx)
        .map_err(|e| GovernancePolicyAbiError(format!("missing slot {idx} ({label}): {e}")))?
        .into_collection(CollectionKind::Vector)
        .map_err(|e| {
            GovernancePolicyAbiError(format!("slot {idx} ({label}) expected vector: {e}"))
        })?;
    let mut out = Vec::with_capacity(sub.len());
    for i in 0..sub.len() {
        let item = sub
            .element(i)
            .map_err(|e| GovernancePolicyAbiError(format!("{label}[{i}]: {e}")))?;
        out.push(read_string_obj(item, label)?);
    }
    Ok(out)
}

fn read_pair_vec_slot(
    vec: &RootedSchemeCollection<'_>,
    idx: usize,
    label: &str,
) -> Result<Vec<(String, String)>, GovernancePolicyAbiError> {
    let sub = vec
        .element(idx)
        .map_err(|e| GovernancePolicyAbiError(format!("missing slot {idx} ({label}): {e}")))?
        .into_collection(CollectionKind::Vector)
        .map_err(|e| {
            GovernancePolicyAbiError(format!("slot {idx} ({label}) expected vector: {e}"))
        })?;
    let mut out = Vec::with_capacity(sub.len());
    for i in 0..sub.len() {
        let pair = sub
            .element(i)
            .map_err(|e| GovernancePolicyAbiError(format!("{label}[{i}]: {e}")))?
            .into_collection(CollectionKind::Vector)
            .map_err(|e| {
                GovernancePolicyAbiError(format!("{label}[{i}] expected 2-slot vector: {e}"))
            })?;
        if pair.len() != 2 {
            return Err(GovernancePolicyAbiError(format!(
                "{label}[{i}] expected length 2, got {}",
                pair.len()
            )));
        }
        let left = read_string_slot(&pair, 0, label)?;
        let right = read_string_slot(&pair, 1, label)?;
        out.push((left, right));
    }
    Ok(out)
}

/// Decodes a 14-slot rooted Gambit heap governance proof vector with fail-closed arity & version validation.
pub fn decode_governance_policy_vector(
    object: RootedSchemeObject<'_>,
) -> Result<GovernancePolicyVectorProjection, GovernancePolicyAbiError> {
    let vec = object
        .into_collection(CollectionKind::Vector)
        .map_err(|e| {
            GovernancePolicyAbiError(format!("expected 14-slot rooted vector: {e}"))
        })?;
    if vec.len() != 14 {
        return Err(GovernancePolicyAbiError(format!(
            "expected 14-slot governance vector, got {}",
            vec.len()
        )));
    }
    let schema_version = read_u64_slot(&vec, 0, "schema_version")? as u32;
    if schema_version != MARLIN_GOVERNANCE_ADMISSION_SCHEMA_VERSION {
        return Err(GovernancePolicyAbiError(format!(
            "unsupported schema_version {schema_version}"
        )));
    }
    Ok(GovernancePolicyVectorProjection {
        schema_version,
        admitted: read_bool_slot(&vec, 1, "admitted")?,
        revocation_epoch: read_u64_slot(&vec, 2, "revocation_epoch")?,
        policy_revision: read_u64_slot(&vec, 3, "policy_revision")?,
        assessment_digest: read_string_slot(&vec, 4, "assessment_digest")?,
        taint_proof_digest: read_string_slot(&vec, 5, "taint_proof_digest")?,
        fixpoint_proof_digest: read_string_slot(&vec, 6, "fixpoint_proof_digest")?,
        temporal_proof_digest: read_string_slot(&vec, 7, "temporal_proof_digest")?,
        invalidation_digest: read_string_slot(&vec, 8, "invalidation_digest")?,
        rejection_reasons: read_string_vec_slot(&vec, 9, "rejection_reasons")?,
        fenced_actors: read_string_vec_slot(&vec, 10, "fenced_actors")?,
        blocked_tools: read_string_vec_slot(&vec, 11, "blocked_tools")?,
        retracted_conclusions: read_string_vec_slot(&vec, 12, "retracted_conclusions")?,
        cedar_policies: read_pair_vec_slot(&vec, 13, "cedar_policies")?,
    })
}

//! Gerbil `POO` AgentGraph policy routing projection into Rust receipts.

use marlin_agent_graph::{
    AgentCoordinationEvidenceKind, AgentCoordinationEvidenceRef, AgentEdgeId, AgentEvidenceId,
    AgentGraphId, AgentGraphValidationError, AgentNodeId, AgentPolicyRoutingDecision,
    AgentPolicyRoutingReceipt, GerbilPolicyScopeRef,
};
use serde::{Deserialize, Serialize};

use crate::{
    GerbilSchemeFieldName, GerbilSchemeNativeAbiContract, GerbilSchemeNativeAbiId,
    GerbilSchemeNativeAbiReadinessPlan, GerbilSchemeNativeProjectionReceipt,
    GerbilSchemeNativeProjectionRequest, GerbilSchemeNativeSymbol, GerbilSchemePackageId,
    GerbilSchemePackageManifest, GerbilSchemeProjectionContract, GerbilSchemeSchemaId,
    GerbilSchemeTypeDecodeError, GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId,
    GerbilSchemeTypeManifest, GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec,
    GerbilSchemeTypedProjection, GerbilSchemeTypedValue, decode_gerbil_scheme_native_projection,
};

/// Native ABI id for AgentGraph policy routing typed projections.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_ID: &str =
    "marlin.agent.policy-routing.native-projection";

/// Package id for the AgentGraph policy routing typed projection ABI manifest.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_PACKAGE_ID: &str =
    "marlin.agent.policy-routing.native-projection";

/// Native ABI version for AgentGraph policy routing typed projections.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_VERSION: u32 = 1;

/// Scheme type identifier expected for AgentGraph policy routing decisions.
pub const GERBIL_AGENT_POLICY_ROUTING_TYPE_ID: &str = "marlin.agent.policy-routing";

/// Schema id returned by the AgentGraph policy routing projection.
pub const GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID: &str = "marlin.agent.policy-routing.v1";

/// Native symbol expected to project a Gerbil/POO policy scope route decision.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_SYMBOL: &str =
    "marlin_agent_policy_routing_select_edges";

/// Rust projection for a Gerbil-built AgentGraph policy routing receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilAgentPolicyRoutingProjection {
    pub schema_id: String,
    pub graph_id: String,
    pub policy_scope: String,
    pub root_node: String,
    pub decision: GerbilAgentPolicyRoutingDecision,
    pub candidate_edges: Vec<String>,
    pub evidence: Vec<GerbilAgentPolicyRoutingEvidence>,
}

impl GerbilAgentPolicyRoutingProjection {
    /// Returns whether the embedded payload schema matches the current Rust projection.
    pub fn has_current_schema(&self) -> bool {
        self.schema_id == GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID
    }

    fn ensure_current_schema(&self) -> Result<(), GerbilSchemeTypeDecodeError> {
        if self.has_current_schema() {
            Ok(())
        } else {
            Err(GerbilSchemeTypeDecodeError::RustProjection {
                message: format!(
                    "AgentGraph policy routing payload schema {} does not match {}",
                    self.schema_id, GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID
                ),
            })
        }
    }
}

impl GerbilSchemeTypedProjection for GerbilAgentPolicyRoutingProjection {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_agent_policy_routing_projection_contract()
    }
}

/// Routing decision returned by the Gerbil AgentGraph policy routing projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GerbilAgentPolicyRoutingDecision {
    SelectEdges,
    Deny,
    Defer,
}

/// Evidence reference returned by the Gerbil AgentGraph policy routing projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilAgentPolicyRoutingEvidence {
    pub kind: GerbilAgentPolicyRoutingEvidenceKind,
    pub evidence_id: String,
}

/// Evidence kind returned by the Gerbil AgentGraph policy routing projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GerbilAgentPolicyRoutingEvidenceKind {
    LoopReceipt,
    OrgMemoryReceipt,
    GerbilPolicyReceipt,
    HookReceipt,
    RuntimeReceipt,
}

/// Contract expected for the Gerbil AgentGraph policy routing projection.
pub fn gerbil_agent_policy_routing_projection_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_AGENT_POLICY_ROUTING_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID,
    ))
}

/// Scheme type manifest for AgentGraph policy routing projections.
pub fn gerbil_agent_policy_routing_type_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_AGENT_POLICY_ROUTING_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID,
            )),
            fields: vec![
                required_policy_routing_field("schema_id", "string", None),
                required_policy_routing_field("graph_id", "string", None),
                required_policy_routing_field("policy_scope", "string", None),
                required_policy_routing_field("root_node", "string", None),
                required_policy_routing_field("decision", "string", None),
                required_policy_routing_field("candidate_edges", "array", Some("string")),
                required_policy_routing_field("evidence", "array", Some("object")),
            ],
        }],
    }
}

/// Native ABI projection request for the Gerbil AgentGraph policy routing projection.
pub fn gerbil_agent_policy_routing_native_projection_request() -> GerbilSchemeNativeProjectionRequest
{
    GerbilSchemeNativeProjectionRequest::new(
        GerbilSchemeNativeAbiId::new(GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_ID),
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_VERSION,
        GerbilSchemeNativeSymbol::new(GERBIL_AGENT_POLICY_ROUTING_NATIVE_SYMBOL),
        gerbil_agent_policy_routing_projection_contract(),
    )
}

/// Native ABI contract declared by the AgentGraph policy routing projection package.
pub fn gerbil_agent_policy_routing_native_projection_abi_contract() -> GerbilSchemeNativeAbiContract
{
    GerbilSchemeNativeAbiContract::new(
        GerbilSchemeNativeAbiId::new(GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_ID),
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_VERSION,
    )
    .with_exported_symbols([GerbilSchemeNativeSymbol::new(
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_SYMBOL,
    )])
}

/// Readiness plan expected before calling the AgentGraph policy routing native projection ABI.
pub fn gerbil_agent_policy_routing_native_projection_readiness_plan()
-> GerbilSchemeNativeAbiReadinessPlan {
    GerbilSchemeNativeAbiReadinessPlan::new(
        GerbilSchemeNativeAbiId::new(GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_ID),
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_ABI_VERSION,
    )
    .with_exported_symbols([GerbilSchemeNativeSymbol::new(
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_SYMBOL,
    )])
}

/// Package manifest for the AgentGraph policy routing typed projection ABI.
pub fn gerbil_agent_policy_routing_native_projection_package_manifest()
-> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new(GERBIL_AGENT_POLICY_ROUTING_NATIVE_PROJECTION_PACKAGE_ID),
        gerbil_agent_policy_routing_type_manifest(),
    )
    .with_projection_contracts([gerbil_agent_policy_routing_projection_contract()])
    .with_native_abi(gerbil_agent_policy_routing_native_projection_abi_contract())
}

/// Decode the typed Gerbil AgentGraph policy routing projection.
pub fn decode_gerbil_agent_policy_routing_projection(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilAgentPolicyRoutingProjection, GerbilSchemeTypeDecodeError> {
    let projection: GerbilAgentPolicyRoutingProjection = registry.decode_projection(typed_value)?;
    projection.ensure_current_schema()?;
    Ok(projection)
}

/// Decode an AgentGraph policy routing projection returned by the native ABI.
pub fn decode_gerbil_agent_policy_routing_native_projection(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<
    (
        GerbilSchemeNativeProjectionReceipt,
        GerbilAgentPolicyRoutingProjection,
    ),
    GerbilSchemeTypeDecodeError,
> {
    let (receipt, projection) =
        decode_gerbil_scheme_native_projection::<GerbilAgentPolicyRoutingProjection>(
            registry,
            &gerbil_agent_policy_routing_native_projection_readiness_plan(),
            &gerbil_agent_policy_routing_native_projection_request(),
            typed_value,
        )?;
    projection.ensure_current_schema()?;
    Ok((receipt, projection))
}

/// Project a typed Gerbil AgentGraph policy routing value into a Rust policy receipt.
pub fn project_gerbil_agent_policy_routing_receipt(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<AgentPolicyRoutingReceipt, GerbilSchemeTypeDecodeError> {
    project_agent_policy_routing_receipt(decode_gerbil_agent_policy_routing_projection(
        registry,
        typed_value,
    )?)
}

/// Project a native ABI Gerbil AgentGraph policy routing value into a Rust policy receipt.
pub fn project_gerbil_agent_policy_routing_native_receipt(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<
    (
        GerbilSchemeNativeProjectionReceipt,
        AgentPolicyRoutingReceipt,
    ),
    GerbilSchemeTypeDecodeError,
> {
    let (receipt, projection) =
        decode_gerbil_agent_policy_routing_native_projection(registry, typed_value)?;
    let policy_receipt = project_agent_policy_routing_receipt(projection)?;
    Ok((receipt, policy_receipt))
}

fn project_agent_policy_routing_receipt(
    projection: GerbilAgentPolicyRoutingProjection,
) -> Result<AgentPolicyRoutingReceipt, GerbilSchemeTypeDecodeError> {
    Ok(AgentPolicyRoutingReceipt {
        graph_id: project_id("AgentGraphId", projection.graph_id, AgentGraphId::new)?,
        policy_scope: project_id(
            "GerbilPolicyScopeRef",
            projection.policy_scope,
            GerbilPolicyScopeRef::new,
        )?,
        root_node: project_id("AgentNodeId", projection.root_node, AgentNodeId::new)?,
        decision: project_policy_routing_decision(projection.decision),
        candidate_edges: projection
            .candidate_edges
            .into_iter()
            .map(|edge_id| project_id("AgentEdgeId", edge_id, AgentEdgeId::new))
            .collect::<Result<Vec<_>, _>>()?,
        evidence: projection
            .evidence
            .into_iter()
            .map(project_policy_routing_evidence)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn project_policy_routing_decision(
    decision: GerbilAgentPolicyRoutingDecision,
) -> AgentPolicyRoutingDecision {
    match decision {
        GerbilAgentPolicyRoutingDecision::SelectEdges => AgentPolicyRoutingDecision::SelectEdges,
        GerbilAgentPolicyRoutingDecision::Deny => AgentPolicyRoutingDecision::Deny,
        GerbilAgentPolicyRoutingDecision::Defer => AgentPolicyRoutingDecision::Defer,
    }
}

fn project_policy_routing_evidence(
    evidence: GerbilAgentPolicyRoutingEvidence,
) -> Result<AgentCoordinationEvidenceRef, GerbilSchemeTypeDecodeError> {
    Ok(AgentCoordinationEvidenceRef {
        kind: match evidence.kind {
            GerbilAgentPolicyRoutingEvidenceKind::LoopReceipt => {
                AgentCoordinationEvidenceKind::LoopReceipt
            }
            GerbilAgentPolicyRoutingEvidenceKind::OrgMemoryReceipt => {
                AgentCoordinationEvidenceKind::OrgMemoryReceipt
            }
            GerbilAgentPolicyRoutingEvidenceKind::GerbilPolicyReceipt => {
                AgentCoordinationEvidenceKind::GerbilPolicyReceipt
            }
            GerbilAgentPolicyRoutingEvidenceKind::HookReceipt => {
                AgentCoordinationEvidenceKind::HookReceipt
            }
            GerbilAgentPolicyRoutingEvidenceKind::RuntimeReceipt => {
                AgentCoordinationEvidenceKind::RuntimeReceipt
            }
        },
        evidence_id: project_id(
            "AgentEvidenceId",
            evidence.evidence_id,
            AgentEvidenceId::new,
        )?,
    })
}

fn project_id<T>(
    type_name: &'static str,
    value: String,
    constructor: impl FnOnce(String) -> Result<T, AgentGraphValidationError>,
) -> Result<T, GerbilSchemeTypeDecodeError> {
    constructor(value).map_err(|error| GerbilSchemeTypeDecodeError::RustProjection {
        message: format!("AgentGraph policy routing {type_name} projection failed: {error}"),
    })
}

fn required_policy_routing_field(
    name: &str,
    type_id: &str,
    element_type_id: Option<&str>,
) -> GerbilSchemeTypeFieldSpec {
    GerbilSchemeTypeFieldSpec {
        name: GerbilSchemeFieldName::new(name),
        type_id: GerbilSchemeTypeId::new(type_id),
        element_type_id: element_type_id.map(GerbilSchemeTypeId::new),
        required: true,
        description: None,
    }
}

//! Rust request and backing storage for Gerbil AgentGraph policy routing.

use crate::agent_policy_routing::GerbilAgentPolicyRoutingEvidenceKind;

use super::abi::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION, GerbilAgentPolicyRoutingAbiEvidence,
    GerbilAgentPolicyRoutingAbiEvidenceList, GerbilAgentPolicyRoutingAbiRequest,
    GerbilAgentPolicyRoutingAbiUtf8, GerbilAgentPolicyRoutingAbiUtf8List,
};

/// Rust request used to build the native policy-routing ABI input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingAbiSelectEdgesRequest {
    pub graph_id: String,
    pub policy_scope: String,
    pub root_node: String,
    pub candidate_edges: Vec<String>,
    pub routing_evidence: Vec<GerbilAgentPolicyRoutingAbiEvidenceRef>,
}

/// Stable match key for AgentGraph policy-routing native selectors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingAbiMatchKey {
    graph_id: String,
    policy_scope: String,
    root_node: String,
}

impl GerbilAgentPolicyRoutingAbiMatchKey {
    /// Builds a policy-routing match key for graph, policy scope, and root node.
    pub fn new(
        graph_id: impl Into<String>,
        policy_scope: impl Into<String>,
        root_node: impl Into<String>,
    ) -> Self {
        Self {
            graph_id: graph_id.into(),
            policy_scope: policy_scope.into(),
            root_node: root_node.into(),
        }
    }

    /// Returns the agent graph identifier.
    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    /// Returns the Gerbil policy scope identifier.
    pub fn policy_scope(&self) -> &str {
        &self.policy_scope
    }

    /// Returns the root agent node identifier.
    pub fn root_node(&self) -> &str {
        &self.root_node
    }
}

/// Per-decision payload for AgentGraph policy-routing native selectors.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingAbiPayload {
    candidate_edges: Vec<String>,
    routing_evidence: Vec<GerbilAgentPolicyRoutingAbiEvidenceRef>,
}

impl GerbilAgentPolicyRoutingAbiPayload {
    /// Builds an empty policy-routing payload.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a candidate edge identifier.
    pub fn with_candidate_edge(mut self, edge_id: impl Into<String>) -> Self {
        self.candidate_edges.push(edge_id.into());
        self
    }

    /// Adds an evidence reference.
    pub fn with_evidence(
        mut self,
        evidence_kind: GerbilAgentPolicyRoutingEvidenceKind,
        evidence_id: impl Into<String>,
    ) -> Self {
        self.routing_evidence
            .push(GerbilAgentPolicyRoutingAbiEvidenceRef::new(
                evidence_kind,
                evidence_id,
            ));
        self
    }

    /// Returns candidate edge identifiers.
    pub fn candidate_edges(&self) -> &[String] {
        &self.candidate_edges
    }

    /// Returns routing evidence references.
    pub fn routing_evidence(&self) -> &[GerbilAgentPolicyRoutingAbiEvidenceRef] {
        &self.routing_evidence
    }
}

impl GerbilAgentPolicyRoutingAbiSelectEdgesRequest {
    /// Builds a policy-routing native ABI request.
    pub fn new(
        graph_id: impl Into<String>,
        policy_scope: impl Into<String>,
        root_node: impl Into<String>,
    ) -> Self {
        Self::from_parts(
            GerbilAgentPolicyRoutingAbiMatchKey::new(graph_id, policy_scope, root_node),
            GerbilAgentPolicyRoutingAbiPayload::new(),
        )
    }

    /// Builds a policy-routing native ABI request from split match key and payload.
    pub fn from_parts(
        match_key: GerbilAgentPolicyRoutingAbiMatchKey,
        payload: GerbilAgentPolicyRoutingAbiPayload,
    ) -> Self {
        Self {
            graph_id: match_key.graph_id,
            policy_scope: match_key.policy_scope,
            root_node: match_key.root_node,
            candidate_edges: payload.candidate_edges,
            routing_evidence: payload.routing_evidence,
        }
    }

    /// Adds a candidate edge identifier.
    pub fn with_candidate_edge(mut self, edge_id: impl Into<String>) -> Self {
        self.candidate_edges.push(edge_id.into());
        self
    }

    /// Adds an evidence reference.
    pub fn with_evidence(
        mut self,
        evidence_kind: GerbilAgentPolicyRoutingEvidenceKind,
        evidence_id: impl Into<String>,
    ) -> Self {
        self.routing_evidence
            .push(GerbilAgentPolicyRoutingAbiEvidenceRef::new(
                evidence_kind,
                evidence_id,
            ));
        self
    }

    /// Profiles the Rust-side native request conversion cost for this request.
    pub fn native_conversion_profile(&self) -> GerbilAgentPolicyRoutingAbiRequestConversionProfile {
        gerbil_agent_policy_routing_abi_request_conversion_profile(self)
    }

    /// Returns the stable match key portion of this request.
    pub fn match_key(&self) -> GerbilAgentPolicyRoutingAbiMatchKey {
        GerbilAgentPolicyRoutingAbiMatchKey::new(
            self.graph_id.clone(),
            self.policy_scope.clone(),
            self.root_node.clone(),
        )
    }

    /// Returns the per-decision payload portion of this request.
    pub fn payload(&self) -> GerbilAgentPolicyRoutingAbiPayload {
        GerbilAgentPolicyRoutingAbiPayload {
            candidate_edges: self.candidate_edges.clone(),
            routing_evidence: self.routing_evidence.clone(),
        }
    }

    pub(super) fn view(&self) -> GerbilAgentPolicyRoutingAbiRequestView<'_> {
        GerbilAgentPolicyRoutingAbiRequestView {
            graph_id: &self.graph_id,
            policy_scope: &self.policy_scope,
            root_node: &self.root_node,
            candidate_edges: &self.candidate_edges,
            routing_evidence: &self.routing_evidence,
        }
    }
}

/// Rust-side conversion profile for one AgentGraph policy-routing native request.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingAbiRequestConversionProfile {
    /// Number of scalar request strings copied before entering native code.
    pub scalar_string_count: usize,
    /// Number of candidate edge strings copied before entering native code.
    pub candidate_edge_count: usize,
    /// Number of evidence records copied before entering native code.
    pub evidence_count: usize,
    /// Total number of individual string values copied before entering native code.
    pub copied_string_count: usize,
    /// Total number of UTF-8 bytes copied into Rust-owned backing storage.
    pub copied_utf8_bytes: usize,
    /// Number of raw UTF-8 descriptors built for the native request.
    pub raw_utf8_descriptor_count: usize,
    /// Number of raw evidence descriptors built for the native request.
    pub raw_evidence_descriptor_count: usize,
    /// Number of raw list descriptors built for the native request.
    pub raw_list_descriptor_count: usize,
    /// Number of Rust backing vectors allocated by the current marshalling path.
    pub backing_vector_count: usize,
    /// Number of scalar strings reused from epoch-owned backing instead of copied per call.
    pub reused_epoch_scalar_string_count: usize,
}

/// Builds the current Rust-side conversion profile by constructing native backing storage.
pub fn gerbil_agent_policy_routing_abi_request_conversion_profile(
    request: &GerbilAgentPolicyRoutingAbiSelectEdgesRequest,
) -> GerbilAgentPolicyRoutingAbiRequestConversionProfile {
    GerbilAgentPolicyRoutingAbiRequestBacking::new(request).conversion_profile()
}

/// Epoch-owned backing for stable AgentGraph policy-routing match keys.
///
/// This keeps graph, policy scope, and root-node UTF-8 backing stable across
/// repeated native selector calls in the same policy epoch.
pub struct GerbilAgentPolicyRoutingAbiEpochBacking {
    match_key: GerbilAgentPolicyRoutingAbiMatchKey,
    backing: GerbilAgentPolicyRoutingAbiMatchKeyBacking,
}

impl GerbilAgentPolicyRoutingAbiEpochBacking {
    /// Builds epoch-owned backing for a stable match key.
    pub fn new(match_key: GerbilAgentPolicyRoutingAbiMatchKey) -> Self {
        let backing = GerbilAgentPolicyRoutingAbiMatchKeyBacking::new(&match_key);
        Self { match_key, backing }
    }

    /// Builds epoch-owned backing from an existing full request.
    pub fn from_request(request: &GerbilAgentPolicyRoutingAbiSelectEdgesRequest) -> Self {
        Self::new(request.match_key())
    }

    /// Returns the stable match key associated with this backing.
    pub fn match_key(&self) -> &GerbilAgentPolicyRoutingAbiMatchKey {
        &self.match_key
    }

    /// Profiles payload-only conversion when scalar match-key backing is reused from the epoch.
    pub fn native_conversion_profile_for_payload(
        &self,
        payload: &GerbilAgentPolicyRoutingAbiPayload,
    ) -> GerbilAgentPolicyRoutingAbiRequestConversionProfile {
        GerbilAgentPolicyRoutingAbiPayloadBacking::new(payload).conversion_profile()
    }

    pub(super) fn raw_request(
        &self,
        payload_backing: &GerbilAgentPolicyRoutingAbiPayloadBacking,
    ) -> GerbilAgentPolicyRoutingAbiRequest {
        GerbilAgentPolicyRoutingAbiRequest {
            abi_version: GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
            graph_id: self.backing.graph_id.raw(),
            policy_scope: self.backing.policy_scope.raw(),
            root_node: self.backing.root_node.raw(),
            candidate_edges: payload_backing.candidate_edges.raw(),
            routing_evidence: payload_backing.routing_evidence.raw(),
        }
    }
}

pub(super) struct GerbilAgentPolicyRoutingAbiRequestView<'a> {
    pub(super) graph_id: &'a str,
    pub(super) policy_scope: &'a str,
    pub(super) root_node: &'a str,
    pub(super) candidate_edges: &'a [String],
    pub(super) routing_evidence: &'a [GerbilAgentPolicyRoutingAbiEvidenceRef],
}

impl<'a> GerbilAgentPolicyRoutingAbiRequestView<'a> {
    pub(super) fn from_parts(
        match_key: &'a GerbilAgentPolicyRoutingAbiMatchKey,
        payload: &'a GerbilAgentPolicyRoutingAbiPayload,
    ) -> Self {
        Self {
            graph_id: match_key.graph_id(),
            policy_scope: match_key.policy_scope(),
            root_node: match_key.root_node(),
            candidate_edges: payload.candidate_edges(),
            routing_evidence: payload.routing_evidence(),
        }
    }
}

/// Rust-owned evidence reference used to build native ABI inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingAbiEvidenceRef {
    pub evidence_kind: GerbilAgentPolicyRoutingEvidenceKind,
    pub evidence_id: String,
}

impl GerbilAgentPolicyRoutingAbiEvidenceRef {
    /// Builds an evidence reference used by policy-routing native ABI requests.
    pub fn new(
        evidence_kind: GerbilAgentPolicyRoutingEvidenceKind,
        evidence_id: impl Into<String>,
    ) -> Self {
        Self {
            evidence_kind,
            evidence_id: evidence_id.into(),
        }
    }
}

pub(super) fn agent_policy_routing_evidence_kind_as_str(
    evidence_kind: &GerbilAgentPolicyRoutingEvidenceKind,
) -> &'static str {
    match evidence_kind {
        GerbilAgentPolicyRoutingEvidenceKind::LoopReceipt => "loop_receipt",
        GerbilAgentPolicyRoutingEvidenceKind::OrgMemoryReceipt => "org_memory_receipt",
        GerbilAgentPolicyRoutingEvidenceKind::GerbilPolicyReceipt => "gerbil_policy_receipt",
        GerbilAgentPolicyRoutingEvidenceKind::HookReceipt => "hook_receipt",
        GerbilAgentPolicyRoutingEvidenceKind::RuntimeReceipt => "runtime_receipt",
    }
}

struct GerbilAgentPolicyRoutingAbiStringBacking {
    bytes: Vec<u8>,
}

impl GerbilAgentPolicyRoutingAbiStringBacking {
    fn new(value: &str) -> Self {
        Self {
            bytes: value.as_bytes().to_vec(),
        }
    }

    fn raw(&self) -> GerbilAgentPolicyRoutingAbiUtf8 {
        GerbilAgentPolicyRoutingAbiUtf8::from_bytes(&self.bytes)
    }
}

pub(super) struct GerbilAgentPolicyRoutingAbiStringListBacking {
    values: Vec<GerbilAgentPolicyRoutingAbiStringBacking>,
    raw_items: Vec<GerbilAgentPolicyRoutingAbiUtf8>,
}

impl GerbilAgentPolicyRoutingAbiStringListBacking {
    fn new(values: &[String]) -> Self {
        let values = values
            .iter()
            .map(|value| GerbilAgentPolicyRoutingAbiStringBacking::new(value))
            .collect::<Vec<_>>();
        let raw_items = values.iter().map(|value| value.raw()).collect::<Vec<_>>();
        Self { values, raw_items }
    }

    fn raw(&self) -> GerbilAgentPolicyRoutingAbiUtf8List {
        let _keep_values_alive = self.values.len();
        GerbilAgentPolicyRoutingAbiUtf8List::from_items(&self.raw_items)
    }
}

struct GerbilAgentPolicyRoutingAbiEvidenceBacking {
    evidence_kind: GerbilAgentPolicyRoutingAbiStringBacking,
    evidence_id: GerbilAgentPolicyRoutingAbiStringBacking,
}

impl GerbilAgentPolicyRoutingAbiEvidenceBacking {
    fn new(evidence: &GerbilAgentPolicyRoutingAbiEvidenceRef) -> Self {
        Self {
            evidence_kind: GerbilAgentPolicyRoutingAbiStringBacking::new(
                agent_policy_routing_evidence_kind_as_str(&evidence.evidence_kind),
            ),
            evidence_id: GerbilAgentPolicyRoutingAbiStringBacking::new(&evidence.evidence_id),
        }
    }

    fn raw(&self) -> GerbilAgentPolicyRoutingAbiEvidence {
        GerbilAgentPolicyRoutingAbiEvidence {
            evidence_kind: self.evidence_kind.raw(),
            evidence_id: self.evidence_id.raw(),
        }
    }
}

pub(super) struct GerbilAgentPolicyRoutingAbiEvidenceListBacking {
    values: Vec<GerbilAgentPolicyRoutingAbiEvidenceBacking>,
    raw_items: Vec<GerbilAgentPolicyRoutingAbiEvidence>,
}

impl GerbilAgentPolicyRoutingAbiEvidenceListBacking {
    fn new(values: &[GerbilAgentPolicyRoutingAbiEvidenceRef]) -> Self {
        let values = values
            .iter()
            .map(GerbilAgentPolicyRoutingAbiEvidenceBacking::new)
            .collect::<Vec<_>>();
        let raw_items = values.iter().map(|value| value.raw()).collect::<Vec<_>>();
        Self { values, raw_items }
    }

    fn raw(&self) -> GerbilAgentPolicyRoutingAbiEvidenceList {
        let _keep_values_alive = self.values.len();
        GerbilAgentPolicyRoutingAbiEvidenceList::from_items(&self.raw_items)
    }
}

struct GerbilAgentPolicyRoutingAbiMatchKeyBacking {
    graph_id: GerbilAgentPolicyRoutingAbiStringBacking,
    policy_scope: GerbilAgentPolicyRoutingAbiStringBacking,
    root_node: GerbilAgentPolicyRoutingAbiStringBacking,
}

impl GerbilAgentPolicyRoutingAbiMatchKeyBacking {
    fn new(match_key: &GerbilAgentPolicyRoutingAbiMatchKey) -> Self {
        Self {
            graph_id: GerbilAgentPolicyRoutingAbiStringBacking::new(match_key.graph_id()),
            policy_scope: GerbilAgentPolicyRoutingAbiStringBacking::new(match_key.policy_scope()),
            root_node: GerbilAgentPolicyRoutingAbiStringBacking::new(match_key.root_node()),
        }
    }
}

pub(super) struct GerbilAgentPolicyRoutingAbiPayloadBacking {
    candidate_edges: GerbilAgentPolicyRoutingAbiStringListBacking,
    routing_evidence: GerbilAgentPolicyRoutingAbiEvidenceListBacking,
}

impl GerbilAgentPolicyRoutingAbiPayloadBacking {
    pub(super) fn new(payload: &GerbilAgentPolicyRoutingAbiPayload) -> Self {
        Self {
            candidate_edges: GerbilAgentPolicyRoutingAbiStringListBacking::new(
                payload.candidate_edges(),
            ),
            routing_evidence: GerbilAgentPolicyRoutingAbiEvidenceListBacking::new(
                payload.routing_evidence(),
            ),
        }
    }

    fn conversion_profile(&self) -> GerbilAgentPolicyRoutingAbiRequestConversionProfile {
        let candidate_edge_count = self.candidate_edges.values.len();
        let evidence_count = self.routing_evidence.values.len();
        let copied_string_count = candidate_edge_count + evidence_count * 2;
        let copied_utf8_bytes = self
            .candidate_edges
            .values
            .iter()
            .map(|value| value.bytes.len())
            .sum::<usize>()
            + self
                .routing_evidence
                .values
                .iter()
                .map(|value| value.evidence_kind.bytes.len() + value.evidence_id.bytes.len())
                .sum::<usize>();

        GerbilAgentPolicyRoutingAbiRequestConversionProfile {
            scalar_string_count: 0,
            candidate_edge_count,
            evidence_count,
            copied_string_count,
            copied_utf8_bytes,
            raw_utf8_descriptor_count: copied_string_count,
            raw_evidence_descriptor_count: evidence_count,
            raw_list_descriptor_count: 2,
            backing_vector_count: 4,
            reused_epoch_scalar_string_count: 3,
        }
    }
}

pub(super) struct GerbilAgentPolicyRoutingAbiRequestBacking {
    graph_id: GerbilAgentPolicyRoutingAbiStringBacking,
    policy_scope: GerbilAgentPolicyRoutingAbiStringBacking,
    root_node: GerbilAgentPolicyRoutingAbiStringBacking,
    candidate_edges: GerbilAgentPolicyRoutingAbiStringListBacking,
    routing_evidence: GerbilAgentPolicyRoutingAbiEvidenceListBacking,
}

impl GerbilAgentPolicyRoutingAbiRequestBacking {
    pub(super) fn new(request: &GerbilAgentPolicyRoutingAbiSelectEdgesRequest) -> Self {
        Self {
            graph_id: GerbilAgentPolicyRoutingAbiStringBacking::new(&request.graph_id),
            policy_scope: GerbilAgentPolicyRoutingAbiStringBacking::new(&request.policy_scope),
            root_node: GerbilAgentPolicyRoutingAbiStringBacking::new(&request.root_node),
            candidate_edges: GerbilAgentPolicyRoutingAbiStringListBacking::new(
                &request.candidate_edges,
            ),
            routing_evidence: GerbilAgentPolicyRoutingAbiEvidenceListBacking::new(
                &request.routing_evidence,
            ),
        }
    }

    pub(super) fn raw_request(&self) -> GerbilAgentPolicyRoutingAbiRequest {
        GerbilAgentPolicyRoutingAbiRequest {
            abi_version: GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
            graph_id: self.graph_id.raw(),
            policy_scope: self.policy_scope.raw(),
            root_node: self.root_node.raw(),
            candidate_edges: self.candidate_edges.raw(),
            routing_evidence: self.routing_evidence.raw(),
        }
    }

    fn conversion_profile(&self) -> GerbilAgentPolicyRoutingAbiRequestConversionProfile {
        let scalar_string_count = 3;
        let candidate_edge_count = self.candidate_edges.values.len();
        let evidence_count = self.routing_evidence.values.len();
        let copied_string_count = scalar_string_count + candidate_edge_count + evidence_count * 2;
        let copied_utf8_bytes = self.graph_id.bytes.len()
            + self.policy_scope.bytes.len()
            + self.root_node.bytes.len()
            + self
                .candidate_edges
                .values
                .iter()
                .map(|value| value.bytes.len())
                .sum::<usize>()
            + self
                .routing_evidence
                .values
                .iter()
                .map(|value| value.evidence_kind.bytes.len() + value.evidence_id.bytes.len())
                .sum::<usize>();

        GerbilAgentPolicyRoutingAbiRequestConversionProfile {
            scalar_string_count,
            candidate_edge_count,
            evidence_count,
            copied_string_count,
            copied_utf8_bytes,
            raw_utf8_descriptor_count: copied_string_count,
            raw_evidence_descriptor_count: evidence_count,
            raw_list_descriptor_count: 2,
            backing_vector_count: 4,
            reused_epoch_scalar_string_count: 0,
        }
    }
}

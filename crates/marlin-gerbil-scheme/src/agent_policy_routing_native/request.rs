//! Rust request and backing storage for Gerbil AgentGraph policy routing.

use crate::agent_policy_routing::GerbilAgentPolicyRoutingEvidenceKind;

use super::abi::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION, GerbilAgentPolicyRoutingNativeEvidence,
    GerbilAgentPolicyRoutingNativeEvidenceList, GerbilAgentPolicyRoutingNativeRequest,
    GerbilAgentPolicyRoutingNativeUtf8, GerbilAgentPolicyRoutingNativeUtf8List,
};

/// Rust request used to build the native policy-routing ABI input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingNativeSelectEdgesRequest {
    pub graph_id: String,
    pub policy_scope: String,
    pub root_node: String,
    pub candidate_edges: Vec<String>,
    pub routing_evidence: Vec<GerbilAgentPolicyRoutingNativeEvidenceRef>,
}

/// Stable match key for AgentGraph policy-routing native selectors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingNativeMatchKey {
    graph_id: String,
    policy_scope: String,
    root_node: String,
}

impl GerbilAgentPolicyRoutingNativeMatchKey {
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
pub struct GerbilAgentPolicyRoutingNativePayload {
    candidate_edges: Vec<String>,
    routing_evidence: Vec<GerbilAgentPolicyRoutingNativeEvidenceRef>,
}

impl GerbilAgentPolicyRoutingNativePayload {
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
            .push(GerbilAgentPolicyRoutingNativeEvidenceRef::new(
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
    pub fn routing_evidence(&self) -> &[GerbilAgentPolicyRoutingNativeEvidenceRef] {
        &self.routing_evidence
    }
}

impl GerbilAgentPolicyRoutingNativeSelectEdgesRequest {
    /// Builds a policy-routing native ABI request.
    pub fn new(
        graph_id: impl Into<String>,
        policy_scope: impl Into<String>,
        root_node: impl Into<String>,
    ) -> Self {
        Self::from_parts(
            GerbilAgentPolicyRoutingNativeMatchKey::new(graph_id, policy_scope, root_node),
            GerbilAgentPolicyRoutingNativePayload::new(),
        )
    }

    /// Builds a policy-routing native ABI request from split match key and payload.
    pub fn from_parts(
        match_key: GerbilAgentPolicyRoutingNativeMatchKey,
        payload: GerbilAgentPolicyRoutingNativePayload,
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
            .push(GerbilAgentPolicyRoutingNativeEvidenceRef::new(
                evidence_kind,
                evidence_id,
            ));
        self
    }

    /// Profiles the Rust-side native request conversion cost for this request.
    pub fn native_conversion_profile(
        &self,
    ) -> GerbilAgentPolicyRoutingNativeRequestConversionProfile {
        gerbil_agent_policy_routing_native_request_conversion_profile(self)
    }

    /// Returns the stable match key portion of this request.
    pub fn match_key(&self) -> GerbilAgentPolicyRoutingNativeMatchKey {
        GerbilAgentPolicyRoutingNativeMatchKey::new(
            self.graph_id.clone(),
            self.policy_scope.clone(),
            self.root_node.clone(),
        )
    }

    /// Returns the per-decision payload portion of this request.
    pub fn payload(&self) -> GerbilAgentPolicyRoutingNativePayload {
        GerbilAgentPolicyRoutingNativePayload {
            candidate_edges: self.candidate_edges.clone(),
            routing_evidence: self.routing_evidence.clone(),
        }
    }

    pub(super) fn view(&self) -> GerbilAgentPolicyRoutingNativeRequestView<'_> {
        GerbilAgentPolicyRoutingNativeRequestView {
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
pub struct GerbilAgentPolicyRoutingNativeRequestConversionProfile {
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
pub fn gerbil_agent_policy_routing_native_request_conversion_profile(
    request: &GerbilAgentPolicyRoutingNativeSelectEdgesRequest,
) -> GerbilAgentPolicyRoutingNativeRequestConversionProfile {
    GerbilAgentPolicyRoutingNativeRequestBacking::new(request).conversion_profile()
}

/// Epoch-owned backing for stable AgentGraph policy-routing match keys.
///
/// This keeps graph, policy scope, and root-node UTF-8 backing stable across
/// repeated native selector calls in the same policy epoch.
pub struct GerbilAgentPolicyRoutingNativeEpochBacking {
    match_key: GerbilAgentPolicyRoutingNativeMatchKey,
    backing: GerbilAgentPolicyRoutingNativeMatchKeyBacking,
}

impl GerbilAgentPolicyRoutingNativeEpochBacking {
    /// Builds epoch-owned backing for a stable match key.
    pub fn new(match_key: GerbilAgentPolicyRoutingNativeMatchKey) -> Self {
        let backing = GerbilAgentPolicyRoutingNativeMatchKeyBacking::new(&match_key);
        Self { match_key, backing }
    }

    /// Builds epoch-owned backing from an existing full request.
    pub fn from_request(request: &GerbilAgentPolicyRoutingNativeSelectEdgesRequest) -> Self {
        Self::new(request.match_key())
    }

    /// Returns the stable match key associated with this backing.
    pub fn match_key(&self) -> &GerbilAgentPolicyRoutingNativeMatchKey {
        &self.match_key
    }

    /// Profiles payload-only conversion when scalar match-key backing is reused from the epoch.
    pub fn native_conversion_profile_for_payload(
        &self,
        payload: &GerbilAgentPolicyRoutingNativePayload,
    ) -> GerbilAgentPolicyRoutingNativeRequestConversionProfile {
        GerbilAgentPolicyRoutingNativePayloadBacking::new(payload).conversion_profile()
    }

    pub(super) fn raw_request(
        &self,
        payload_backing: &GerbilAgentPolicyRoutingNativePayloadBacking,
    ) -> GerbilAgentPolicyRoutingNativeRequest {
        GerbilAgentPolicyRoutingNativeRequest {
            abi_version: GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
            graph_id: self.backing.graph_id.raw(),
            policy_scope: self.backing.policy_scope.raw(),
            root_node: self.backing.root_node.raw(),
            candidate_edges: payload_backing.candidate_edges.raw(),
            routing_evidence: payload_backing.routing_evidence.raw(),
        }
    }
}

pub(super) struct GerbilAgentPolicyRoutingNativeRequestView<'a> {
    pub(super) graph_id: &'a str,
    pub(super) policy_scope: &'a str,
    pub(super) root_node: &'a str,
    pub(super) candidate_edges: &'a [String],
    pub(super) routing_evidence: &'a [GerbilAgentPolicyRoutingNativeEvidenceRef],
}

impl<'a> GerbilAgentPolicyRoutingNativeRequestView<'a> {
    pub(super) fn from_parts(
        match_key: &'a GerbilAgentPolicyRoutingNativeMatchKey,
        payload: &'a GerbilAgentPolicyRoutingNativePayload,
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
pub struct GerbilAgentPolicyRoutingNativeEvidenceRef {
    pub evidence_kind: GerbilAgentPolicyRoutingEvidenceKind,
    pub evidence_id: String,
}

impl GerbilAgentPolicyRoutingNativeEvidenceRef {
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

struct GerbilAgentPolicyRoutingNativeStringBacking {
    bytes: Vec<u8>,
}

impl GerbilAgentPolicyRoutingNativeStringBacking {
    fn new(value: &str) -> Self {
        Self {
            bytes: value.as_bytes().to_vec(),
        }
    }

    fn raw(&self) -> GerbilAgentPolicyRoutingNativeUtf8 {
        GerbilAgentPolicyRoutingNativeUtf8::from_bytes(&self.bytes)
    }
}

pub(super) struct GerbilAgentPolicyRoutingNativeStringListBacking {
    values: Vec<GerbilAgentPolicyRoutingNativeStringBacking>,
    raw_items: Vec<GerbilAgentPolicyRoutingNativeUtf8>,
}

impl GerbilAgentPolicyRoutingNativeStringListBacking {
    fn new(values: &[String]) -> Self {
        let values = values
            .iter()
            .map(|value| GerbilAgentPolicyRoutingNativeStringBacking::new(value))
            .collect::<Vec<_>>();
        let raw_items = values.iter().map(|value| value.raw()).collect::<Vec<_>>();
        Self { values, raw_items }
    }

    fn raw(&self) -> GerbilAgentPolicyRoutingNativeUtf8List {
        let _keep_values_alive = self.values.len();
        GerbilAgentPolicyRoutingNativeUtf8List::from_items(&self.raw_items)
    }
}

struct GerbilAgentPolicyRoutingNativeEvidenceBacking {
    evidence_kind: GerbilAgentPolicyRoutingNativeStringBacking,
    evidence_id: GerbilAgentPolicyRoutingNativeStringBacking,
}

impl GerbilAgentPolicyRoutingNativeEvidenceBacking {
    fn new(evidence: &GerbilAgentPolicyRoutingNativeEvidenceRef) -> Self {
        Self {
            evidence_kind: GerbilAgentPolicyRoutingNativeStringBacking::new(
                agent_policy_routing_evidence_kind_as_str(&evidence.evidence_kind),
            ),
            evidence_id: GerbilAgentPolicyRoutingNativeStringBacking::new(&evidence.evidence_id),
        }
    }

    fn raw(&self) -> GerbilAgentPolicyRoutingNativeEvidence {
        GerbilAgentPolicyRoutingNativeEvidence {
            evidence_kind: self.evidence_kind.raw(),
            evidence_id: self.evidence_id.raw(),
        }
    }
}

pub(super) struct GerbilAgentPolicyRoutingNativeEvidenceListBacking {
    values: Vec<GerbilAgentPolicyRoutingNativeEvidenceBacking>,
    raw_items: Vec<GerbilAgentPolicyRoutingNativeEvidence>,
}

impl GerbilAgentPolicyRoutingNativeEvidenceListBacking {
    fn new(values: &[GerbilAgentPolicyRoutingNativeEvidenceRef]) -> Self {
        let values = values
            .iter()
            .map(GerbilAgentPolicyRoutingNativeEvidenceBacking::new)
            .collect::<Vec<_>>();
        let raw_items = values.iter().map(|value| value.raw()).collect::<Vec<_>>();
        Self { values, raw_items }
    }

    fn raw(&self) -> GerbilAgentPolicyRoutingNativeEvidenceList {
        let _keep_values_alive = self.values.len();
        GerbilAgentPolicyRoutingNativeEvidenceList::from_items(&self.raw_items)
    }
}

struct GerbilAgentPolicyRoutingNativeMatchKeyBacking {
    graph_id: GerbilAgentPolicyRoutingNativeStringBacking,
    policy_scope: GerbilAgentPolicyRoutingNativeStringBacking,
    root_node: GerbilAgentPolicyRoutingNativeStringBacking,
}

impl GerbilAgentPolicyRoutingNativeMatchKeyBacking {
    fn new(match_key: &GerbilAgentPolicyRoutingNativeMatchKey) -> Self {
        Self {
            graph_id: GerbilAgentPolicyRoutingNativeStringBacking::new(match_key.graph_id()),
            policy_scope: GerbilAgentPolicyRoutingNativeStringBacking::new(
                match_key.policy_scope(),
            ),
            root_node: GerbilAgentPolicyRoutingNativeStringBacking::new(match_key.root_node()),
        }
    }
}

pub(super) struct GerbilAgentPolicyRoutingNativePayloadBacking {
    candidate_edges: GerbilAgentPolicyRoutingNativeStringListBacking,
    routing_evidence: GerbilAgentPolicyRoutingNativeEvidenceListBacking,
}

impl GerbilAgentPolicyRoutingNativePayloadBacking {
    pub(super) fn new(payload: &GerbilAgentPolicyRoutingNativePayload) -> Self {
        Self {
            candidate_edges: GerbilAgentPolicyRoutingNativeStringListBacking::new(
                payload.candidate_edges(),
            ),
            routing_evidence: GerbilAgentPolicyRoutingNativeEvidenceListBacking::new(
                payload.routing_evidence(),
            ),
        }
    }

    fn conversion_profile(&self) -> GerbilAgentPolicyRoutingNativeRequestConversionProfile {
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

        GerbilAgentPolicyRoutingNativeRequestConversionProfile {
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

pub(super) struct GerbilAgentPolicyRoutingNativeRequestBacking {
    graph_id: GerbilAgentPolicyRoutingNativeStringBacking,
    policy_scope: GerbilAgentPolicyRoutingNativeStringBacking,
    root_node: GerbilAgentPolicyRoutingNativeStringBacking,
    candidate_edges: GerbilAgentPolicyRoutingNativeStringListBacking,
    routing_evidence: GerbilAgentPolicyRoutingNativeEvidenceListBacking,
}

impl GerbilAgentPolicyRoutingNativeRequestBacking {
    pub(super) fn new(request: &GerbilAgentPolicyRoutingNativeSelectEdgesRequest) -> Self {
        Self {
            graph_id: GerbilAgentPolicyRoutingNativeStringBacking::new(&request.graph_id),
            policy_scope: GerbilAgentPolicyRoutingNativeStringBacking::new(&request.policy_scope),
            root_node: GerbilAgentPolicyRoutingNativeStringBacking::new(&request.root_node),
            candidate_edges: GerbilAgentPolicyRoutingNativeStringListBacking::new(
                &request.candidate_edges,
            ),
            routing_evidence: GerbilAgentPolicyRoutingNativeEvidenceListBacking::new(
                &request.routing_evidence,
            ),
        }
    }

    pub(super) fn raw_request(&self) -> GerbilAgentPolicyRoutingNativeRequest {
        GerbilAgentPolicyRoutingNativeRequest {
            abi_version: GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
            graph_id: self.graph_id.raw(),
            policy_scope: self.policy_scope.raw(),
            root_node: self.root_node.raw(),
            candidate_edges: self.candidate_edges.raw(),
            routing_evidence: self.routing_evidence.raw(),
        }
    }

    fn conversion_profile(&self) -> GerbilAgentPolicyRoutingNativeRequestConversionProfile {
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

        GerbilAgentPolicyRoutingNativeRequestConversionProfile {
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

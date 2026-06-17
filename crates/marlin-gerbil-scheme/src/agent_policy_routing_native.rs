//! Native C ABI boundary for Gerbil AgentGraph policy routing projections.
//!
//! Gerbil owns policy semantics. Rust owns the ABI wrapper, the typed Scheme
//! value envelope, and projection into `marlin-agent-graph` receipts.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    slice, str,
};

use marlin_agent_graph::AgentPolicyRoutingReceipt;

use crate::{
    GerbilSchemeNativeProjectionReceipt, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    agent_policy_routing::{
        GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID, GERBIL_AGENT_POLICY_ROUTING_TYPE_ID,
        GerbilAgentPolicyRoutingEvidenceKind, gerbil_agent_policy_routing_type_manifest,
        project_gerbil_agent_policy_routing_native_receipt,
    },
};

/// Current native ABI version accepted by the Rust AgentGraph policy routing wrapper.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION: u32 = 1;
/// Stable native ABI id for Gerbil AgentGraph policy routing.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_ID: &str = "marlin.agent.policy-routing.native";
/// Native policy-routing status code for success.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_OK: i32 = 0;
/// Native policy-routing status code for null pointer inputs.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_NULL_POINTER: i32 = 2;
/// Native policy-routing status code for request ABI version mismatch.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_ABI_MISMATCH: i32 = 3;
/// Native policy-routing status code for invalid output projection data.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_INVALID_PROJECTION: i32 = 4;
/// Relative path of the C header that defines the native AgentGraph policy routing ABI.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_PATH: &str =
    "include/marlin_agent_policy_routing_native.h";
/// Source text of the C header that defines the native AgentGraph policy routing ABI.
pub const GERBIL_AGENT_POLICY_ROUTING_NATIVE_HEADER_SOURCE: &str =
    include_str!("../include/marlin_agent_policy_routing_native.h");

/// Borrowed UTF-8 bytes passed across the AgentGraph policy routing C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingNativeUtf8 {
    /// Pointer to the first UTF-8 byte, or null when `len` is zero.
    pub ptr: *const u8,
    /// Number of UTF-8 bytes available at `ptr`.
    pub len: usize,
}

impl GerbilAgentPolicyRoutingNativeUtf8 {
    /// Builds an empty borrowed UTF-8 slice for native ABI calls.
    pub const fn empty() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
        }
    }

    /// Builds a borrowed UTF-8 slice from a process-static string.
    pub fn from_static(value: &'static str) -> Self {
        Self {
            ptr: value.as_ptr(),
            len: value.len(),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            Self::empty()
        } else {
            Self {
                ptr: bytes.as_ptr(),
                len: bytes.len(),
            }
        }
    }
}

/// Borrowed list of UTF-8 byte slices passed across the native C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingNativeUtf8List {
    /// Pointer to the first UTF-8 slice descriptor, or null when `len` is zero.
    pub items: *const GerbilAgentPolicyRoutingNativeUtf8,
    /// Number of UTF-8 slice descriptors available at `items`.
    pub len: usize,
}

impl GerbilAgentPolicyRoutingNativeUtf8List {
    /// Builds an empty borrowed UTF-8 slice list for native ABI calls.
    pub const fn empty() -> Self {
        Self {
            items: std::ptr::null(),
            len: 0,
        }
    }

    fn from_items(items: &[GerbilAgentPolicyRoutingNativeUtf8]) -> Self {
        if items.is_empty() {
            Self::empty()
        } else {
            Self {
                items: items.as_ptr(),
                len: items.len(),
            }
        }
    }
}

/// Evidence reference passed across the native AgentGraph policy routing ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingNativeEvidence {
    /// Evidence kind string, such as `gerbil_policy_receipt`.
    pub evidence_kind: GerbilAgentPolicyRoutingNativeUtf8,
    /// Stable evidence identifier.
    pub evidence_id: GerbilAgentPolicyRoutingNativeUtf8,
}

/// Borrowed list of evidence descriptors passed across the native C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingNativeEvidenceList {
    /// Pointer to the first evidence descriptor, or null when `len` is zero.
    pub items: *const GerbilAgentPolicyRoutingNativeEvidence,
    /// Number of evidence descriptors available at `items`.
    pub len: usize,
}

impl GerbilAgentPolicyRoutingNativeEvidenceList {
    /// Builds an empty borrowed evidence list for native ABI calls.
    pub const fn empty() -> Self {
        Self {
            items: std::ptr::null(),
            len: 0,
        }
    }

    fn from_items(items: &[GerbilAgentPolicyRoutingNativeEvidence]) -> Self {
        if items.is_empty() {
            Self::empty()
        } else {
            Self {
                items: items.as_ptr(),
                len: items.len(),
            }
        }
    }
}

/// Native policy-routing request passed from Rust to linked Gerbil code.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingNativeRequest {
    /// ABI version expected by the Rust wrapper.
    pub abi_version: u32,
    /// Agent graph identifier being routed.
    pub graph_id: GerbilAgentPolicyRoutingNativeUtf8,
    /// Gerbil policy scope being evaluated.
    pub policy_scope: GerbilAgentPolicyRoutingNativeUtf8,
    /// Root agent node for the routing decision.
    pub root_node: GerbilAgentPolicyRoutingNativeUtf8,
    /// Candidate edge identifiers available to the policy.
    pub candidate_edges: GerbilAgentPolicyRoutingNativeUtf8List,
    /// Evidence references available to the policy.
    pub routing_evidence: GerbilAgentPolicyRoutingNativeEvidenceList,
}

/// Typed policy-routing projection returned by linked Gerbil code.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingNativeProjection {
    /// ABI version written by the native runtime.
    pub abi_version: u32,
    /// Routing decision string, such as `select_edges`, `deny`, or `defer`.
    pub routing_decision: GerbilAgentPolicyRoutingNativeUtf8,
}

impl GerbilAgentPolicyRoutingNativeProjection {
    /// Builds an empty projection descriptor.
    pub const fn empty() -> Self {
        Self {
            abi_version: GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
            routing_decision: GerbilAgentPolicyRoutingNativeUtf8::empty(),
        }
    }

    /// Builds a projection descriptor with a concrete routing decision.
    pub const fn with_routing_decision(
        routing_decision: GerbilAgentPolicyRoutingNativeUtf8,
    ) -> Self {
        Self {
            abi_version: GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
            routing_decision,
        }
    }
}

/// Status code returned by the native Gerbil policy-routing selector.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingNativeStatus(i32);

impl GerbilAgentPolicyRoutingNativeStatus {
    /// Successful native selector status code.
    pub const OK: Self = Self(GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_OK);
    /// Native selector status code for null pointer inputs.
    pub const NULL_POINTER: Self = Self(GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_NULL_POINTER);
    /// Native selector status code for request ABI version mismatch.
    pub const ABI_MISMATCH: Self = Self(GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_ABI_MISMATCH);
    /// Native selector status code for invalid output projection data.
    pub const INVALID_PROJECTION: Self =
        Self(GERBIL_AGENT_POLICY_ROUTING_NATIVE_STATUS_INVALID_PROJECTION);

    /// Builds a status code from a raw native integer.
    pub const fn new(code: i32) -> Self {
        Self(code)
    }

    /// Returns the raw native status code.
    pub const fn code(self) -> i32 {
        self.0
    }

    /// Returns true when the status code is successful.
    pub const fn is_ok(self) -> bool {
        self.0 == Self::OK.0
    }
}

/// Native selector function exported by linked Gerbil policy code.
pub type GerbilAgentPolicyRoutingNativeSelectEdgesFn =
    unsafe extern "C" fn(
        request: *const GerbilAgentPolicyRoutingNativeRequest,
        projection: *mut GerbilAgentPolicyRoutingNativeProjection,
    ) -> GerbilAgentPolicyRoutingNativeStatus;

/// Native function used to initialize the linked Scheme runtime once per process.
pub type GerbilAgentPolicyRoutingNativeInitializeFn = unsafe extern "C" fn() -> i32;

/// Rust request used to build the native policy-routing ABI input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAgentPolicyRoutingNativeSelectEdgesRequest {
    pub graph_id: String,
    pub policy_scope: String,
    pub root_node: String,
    pub candidate_edges: Vec<String>,
    pub routing_evidence: Vec<GerbilAgentPolicyRoutingNativeEvidenceRef>,
}

impl GerbilAgentPolicyRoutingNativeSelectEdgesRequest {
    /// Builds a policy-routing native ABI request.
    pub fn new(
        graph_id: impl Into<String>,
        policy_scope: impl Into<String>,
        root_node: impl Into<String>,
    ) -> Self {
        Self {
            graph_id: graph_id.into(),
            policy_scope: policy_scope.into(),
            root_node: root_node.into(),
            candidate_edges: Vec::new(),
            routing_evidence: Vec::new(),
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

/// Safe Rust wrapper around a Gerbil-owned native AgentGraph policy selector.
#[derive(Clone, Copy)]
pub struct GerbilAgentPolicyRoutingNativeSelector {
    initialize: Option<GerbilAgentPolicyRoutingNativeInitializeFn>,
    select_edges: GerbilAgentPolicyRoutingNativeSelectEdgesFn,
}

impl GerbilAgentPolicyRoutingNativeSelector {
    /// Builds a selector from the native function exported by linked Gerbil code.
    pub const fn new(select_edges: GerbilAgentPolicyRoutingNativeSelectEdgesFn) -> Self {
        Self {
            initialize: None,
            select_edges,
        }
    }

    /// Builds a selector that initializes a linked native Scheme runtime before use.
    pub const fn with_initializer(
        initialize: GerbilAgentPolicyRoutingNativeInitializeFn,
        select_edges: GerbilAgentPolicyRoutingNativeSelectEdgesFn,
    ) -> Self {
        Self {
            initialize: Some(initialize),
            select_edges,
        }
    }

    /// Calls the linked native selector and returns the Rust-owned typed Scheme projection.
    pub fn project_typed_value(
        &self,
        request: &GerbilAgentPolicyRoutingNativeSelectEdgesRequest,
    ) -> Result<GerbilSchemeTypedValue, GerbilAgentPolicyRoutingNativeAbiError> {
        self.initialize_runtime()?;

        let request_backing = GerbilAgentPolicyRoutingNativeRequestBacking::new(request);
        let raw_request = request_backing.raw_request();
        let mut projection = GerbilAgentPolicyRoutingNativeProjection::empty();
        let status = unsafe { (self.select_edges)(&raw_request, &mut projection as *mut _) };

        if !status.is_ok() {
            return Err(GerbilAgentPolicyRoutingNativeAbiError::RuntimeStatus {
                code: status.code(),
            });
        }

        self.typed_value_from_native_projection(request, projection)
    }

    /// Calls the native selector and projects the result into a typed Rust receipt.
    pub fn project_policy_receipt(
        &self,
        request: &GerbilAgentPolicyRoutingNativeSelectEdgesRequest,
    ) -> Result<
        (
            GerbilSchemeNativeProjectionReceipt,
            AgentPolicyRoutingReceipt,
        ),
        GerbilAgentPolicyRoutingNativeAbiError,
    > {
        let typed_value = self.project_typed_value(request)?;
        let registry = GerbilSchemeTypeRegistry::new(gerbil_agent_policy_routing_type_manifest())
            .map_err(GerbilAgentPolicyRoutingNativeAbiError::from_scheme_decode)?;
        project_gerbil_agent_policy_routing_native_receipt(&registry, &typed_value)
            .map_err(GerbilAgentPolicyRoutingNativeAbiError::from_scheme_decode)
    }

    fn initialize_runtime(&self) -> Result<(), GerbilAgentPolicyRoutingNativeAbiError> {
        let Some(initialize) = self.initialize else {
            return Ok(());
        };

        let code = unsafe { initialize() };
        if code == 0 {
            Ok(())
        } else {
            Err(GerbilAgentPolicyRoutingNativeAbiError::RuntimeInit { code })
        }
    }

    fn typed_value_from_native_projection(
        &self,
        request: &GerbilAgentPolicyRoutingNativeSelectEdgesRequest,
        projection: GerbilAgentPolicyRoutingNativeProjection,
    ) -> Result<GerbilSchemeTypedValue, GerbilAgentPolicyRoutingNativeAbiError> {
        if projection.abi_version != GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION {
            return Err(GerbilAgentPolicyRoutingNativeAbiError::OutputAbiVersion {
                expected: GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
                actual: projection.abi_version,
            });
        }

        let routing_decision =
            native_utf8_to_string("routing_decision", projection.routing_decision)?;

        Ok(GerbilSchemeTypedValue::new(
            GerbilSchemeTypeId::new(GERBIL_AGENT_POLICY_ROUTING_TYPE_ID),
            GerbilSchemeValue::record([
                (
                    "schema_id",
                    GerbilSchemeValue::from(GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID),
                ),
                (
                    "graph_id",
                    GerbilSchemeValue::from(request.graph_id.clone()),
                ),
                (
                    "policy_scope",
                    GerbilSchemeValue::from(request.policy_scope.clone()),
                ),
                (
                    "root_node",
                    GerbilSchemeValue::from(request.root_node.clone()),
                ),
                (
                    "routing_decision",
                    GerbilSchemeValue::from(routing_decision),
                ),
                (
                    "candidate_edges",
                    GerbilSchemeValue::vector(
                        request
                            .candidate_edges
                            .iter()
                            .cloned()
                            .map(GerbilSchemeValue::from),
                    ),
                ),
                (
                    "routing_evidence",
                    GerbilSchemeValue::vector(request.routing_evidence.iter().map(|evidence| {
                        GerbilSchemeValue::record([
                            (
                                "evidence_kind",
                                GerbilSchemeValue::from(agent_policy_routing_evidence_kind_as_str(
                                    &evidence.evidence_kind,
                                )),
                            ),
                            (
                                "evidence_id",
                                GerbilSchemeValue::from(evidence.evidence_id.clone()),
                            ),
                        ])
                    })),
                ),
            ]),
        )
        .with_schema_id(GerbilSchemeSchemaId::new(
            GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID,
        )))
    }
}

/// Error raised while invoking the native AgentGraph policy routing ABI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GerbilAgentPolicyRoutingNativeAbiError {
    /// The linked Scheme runtime could not be initialized.
    RuntimeInit {
        /// Native initialization status code.
        code: i32,
    },
    /// The Scheme runtime returned a non-zero native status code.
    RuntimeStatus {
        /// Native status code returned by the selector.
        code: i32,
    },
    /// The native runtime wrote an unexpected output ABI version.
    OutputAbiVersion {
        /// ABI version expected by Rust.
        expected: u32,
        /// ABI version written by the native runtime.
        actual: u32,
    },
    /// The native runtime returned invalid UTF-8 for a typed projection field.
    InvalidUtf8 {
        /// Field whose UTF-8 bytes were invalid.
        field: &'static str,
        /// UTF-8 validation failure detail.
        message: String,
    },
    /// Rust rejected the typed Scheme projection.
    TypeProjection {
        /// Projection failure detail.
        message: String,
    },
}

impl GerbilAgentPolicyRoutingNativeAbiError {
    fn from_scheme_decode(error: GerbilSchemeTypeDecodeError) -> Self {
        Self::TypeProjection {
            message: error.to_string(),
        }
    }
}

impl Display for GerbilAgentPolicyRoutingNativeAbiError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeInit { code } => write!(
                formatter,
                "AgentGraph policy routing native init failed with status {code}"
            ),
            Self::RuntimeStatus { code } => write!(
                formatter,
                "AgentGraph policy routing native selector failed with status {code}"
            ),
            Self::OutputAbiVersion { expected, actual } => write!(
                formatter,
                "AgentGraph policy routing native selector wrote ABI version {actual}, expected {expected}"
            ),
            Self::InvalidUtf8 { field, message } => write!(
                formatter,
                "AgentGraph policy routing native selector returned invalid UTF-8 for {field}: {message}"
            ),
            Self::TypeProjection { message } => write!(
                formatter,
                "AgentGraph policy routing native selector returned an invalid typed projection: {message}"
            ),
        }
    }
}

impl Error for GerbilAgentPolicyRoutingNativeAbiError {}

fn native_utf8_to_string(
    field: &'static str,
    value: GerbilAgentPolicyRoutingNativeUtf8,
) -> Result<String, GerbilAgentPolicyRoutingNativeAbiError> {
    if value.ptr.is_null() {
        if value.len == 0 {
            return Ok(String::new());
        }
        return Err(GerbilAgentPolicyRoutingNativeAbiError::InvalidUtf8 {
            field,
            message: "null pointer with non-zero length".to_string(),
        });
    }

    let bytes = unsafe { slice::from_raw_parts(value.ptr, value.len) };
    str::from_utf8(bytes).map(str::to_owned).map_err(|error| {
        GerbilAgentPolicyRoutingNativeAbiError::InvalidUtf8 {
            field,
            message: error.to_string(),
        }
    })
}

fn agent_policy_routing_evidence_kind_as_str(
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

struct GerbilAgentPolicyRoutingNativeStringListBacking {
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

struct GerbilAgentPolicyRoutingNativeEvidenceListBacking {
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

struct GerbilAgentPolicyRoutingNativeRequestBacking {
    graph_id: GerbilAgentPolicyRoutingNativeStringBacking,
    policy_scope: GerbilAgentPolicyRoutingNativeStringBacking,
    root_node: GerbilAgentPolicyRoutingNativeStringBacking,
    candidate_edges: GerbilAgentPolicyRoutingNativeStringListBacking,
    routing_evidence: GerbilAgentPolicyRoutingNativeEvidenceListBacking,
}

impl GerbilAgentPolicyRoutingNativeRequestBacking {
    fn new(request: &GerbilAgentPolicyRoutingNativeSelectEdgesRequest) -> Self {
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

    fn raw_request(&self) -> GerbilAgentPolicyRoutingNativeRequest {
        GerbilAgentPolicyRoutingNativeRequest {
            abi_version: GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
            graph_id: self.graph_id.raw(),
            policy_scope: self.policy_scope.raw(),
            root_node: self.root_node.raw(),
            candidate_edges: self.candidate_edges.raw(),
            routing_evidence: self.routing_evidence.raw(),
        }
    }
}

//! C ABI shapes for Gerbil AgentGraph policy routing.

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
    include_str!("../../include/marlin_agent_policy_routing_native.h");

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

    pub(super) fn from_bytes(bytes: &[u8]) -> Self {
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

    pub(super) fn from_items(items: &[GerbilAgentPolicyRoutingNativeUtf8]) -> Self {
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

    pub(super) fn from_items(items: &[GerbilAgentPolicyRoutingNativeEvidence]) -> Self {
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

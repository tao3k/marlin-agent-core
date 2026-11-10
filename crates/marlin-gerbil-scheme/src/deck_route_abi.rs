//! Native C ABI boundary for the Deck runtime model-route selector.
//!
//! The Scheme runtime owns policy selection semantics. Rust owns only the C ABI
//! safety wrapper and projects the typed native result into Rust protocol types.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::{
    GERBIL_DECK_RUNTIME_MODEL_ROUTE_SELECTION_SCHEMA_ID, GerbilDeckRouteAbiReceipt,
    GerbilDeckRuntimeModelRoutePolicyRequest, GerbilDeckRuntimeModelRouteSelectedPolicy,
    GerbilDeckRuntimeModelRouteSelectionReceipt, GerbilDeckRuntimeSelectedPolicyKind,
};

/// Current `Deck` runtime native ABI version accepted by Rust.
pub const GERBIL_DECK_ROUTE_ABI_VERSION: u32 = 1;
/// Stable Scheme package ABI id for the native `Deck` runtime.
pub const GERBIL_DECK_ROUTE_ABI_ID: &str = "marlin.deck-runtime.native";
/// Native selector status code for success.
pub const GERBIL_DECK_ROUTE_ABI_STATUS_OK: i32 = 0;
/// Native selector status code for null pointer inputs.
pub const GERBIL_DECK_ROUTE_ABI_STATUS_NULL_POINTER: i32 = 2;
/// Native selector status code for request ABI version mismatch.
pub const GERBIL_DECK_ROUTE_ABI_STATUS_ABI_MISMATCH: i32 = 3;
/// Native selector status code for an invalid output selection.
pub const GERBIL_DECK_ROUTE_ABI_STATUS_INVALID_SELECTION: i32 = 4;
/// Native selector policy index sentinel used when no policy matched.
pub const GERBIL_DECK_ROUTE_ABI_NO_POLICY_INDEX: usize = usize::MAX;
/// Relative path of the C header that defines the native `Deck` runtime ABI.
pub const GERBIL_DECK_ROUTE_ABI_HEADER_PATH: &str = "include/marlin_deck_route_abi.h";
/// Source text of the C header that defines the native `Deck` runtime ABI.
pub const GERBIL_DECK_ROUTE_ABI_HEADER_SOURCE: &str =
    include_str!("../include/marlin_deck_route_abi.h");

/// Borrowed UTF-8 bytes passed across the native C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRouteAbiUtf8 {
    /// Pointer to the first UTF-8 byte, or null when `len` is zero.
    pub ptr: *const u8,
    /// Number of UTF-8 bytes available at `ptr`.
    pub len: usize,
}

impl GerbilDeckRouteAbiUtf8 {
    /// Builds an empty borrowed UTF-8 slice for C ABI calls.
    pub const fn empty() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            ptr: bytes.as_ptr(),
            len: bytes.len(),
        }
    }
}

/// Borrowed list of UTF-8 byte slices passed across the native C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRouteAbiUtf8List {
    /// Pointer to the first UTF-8 slice descriptor, or null when `len` is zero.
    pub items: *const GerbilDeckRouteAbiUtf8,
    /// Number of UTF-8 slice descriptors available at `items`.
    pub len: usize,
}

impl GerbilDeckRouteAbiUtf8List {
    /// Builds an empty borrowed UTF-8 slice list for C ABI calls.
    pub const fn empty() -> Self {
        Self {
            items: std::ptr::null(),
            len: 0,
        }
    }

    fn from_items(items: &[GerbilDeckRouteAbiUtf8]) -> Self {
        Self {
            items: items.as_ptr(),
            len: items.len(),
        }
    }
}

/// Native model-route policy shape consumed by the Scheme `Deck` runtime.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRouteAbiModelRoutePolicy {
    /// Stable policy name.
    pub name: GerbilDeckRouteAbiUtf8,
    /// Provider identity, such as `openai` or `anthropic`.
    pub provider: GerbilDeckRouteAbiUtf8,
    /// Provider-owned model name.
    pub model: GerbilDeckRouteAbiUtf8,
    /// Command prefixes matched by the Scheme policy selector.
    pub command_prefixes: GerbilDeckRouteAbiUtf8List,
    /// Agent scopes matched by the Scheme policy selector.
    pub agent_scopes: GerbilDeckRouteAbiUtf8List,
    /// Context propagation mode selected by the Scheme policy.
    pub context_mode: GerbilDeckRouteAbiUtf8,
    /// Runtime isolation mode selected by the Scheme policy.
    pub isolation_mode: GerbilDeckRouteAbiUtf8,
}

/// Native model-route request shape passed from Rust to the Scheme `Deck` runtime.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRouteAbiModelRouteRequest {
    /// ABI version expected by the Rust wrapper.
    pub abi_version: u32,
    /// Command line being classified.
    pub command: GerbilDeckRouteAbiUtf8,
    /// Agent scope being classified.
    pub agent_scope: GerbilDeckRouteAbiUtf8,
    /// Pointer to policy descriptors owned by the caller for the duration of the call.
    pub policies: *const GerbilDeckRouteAbiModelRoutePolicy,
    /// Number of policy descriptors available at `policies`.
    pub policies_len: usize,
}

/// Typed model-route selection returned from the native Scheme selector.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRouteAbiModelRouteSelection {
    /// ABI version written by the native runtime.
    pub abi_version: u32,
    /// Non-zero when a policy matched.
    pub matched: u8,
    /// Reserved padding for stable C layout.
    pub reserved: [u8; 3],
    /// Index into the request policy array when `matched` is non-zero.
    pub policy_index: usize,
}

impl GerbilDeckRouteAbiModelRouteSelection {
    /// Sentinel policy index used when no policy matched.
    pub const NO_POLICY_INDEX: usize = GERBIL_DECK_ROUTE_ABI_NO_POLICY_INDEX;

    /// Builds an empty unmatched selection descriptor.
    pub const fn empty() -> Self {
        Self {
            abi_version: GERBIL_DECK_ROUTE_ABI_VERSION,
            matched: 0,
            reserved: [0; 3],
            policy_index: Self::NO_POLICY_INDEX,
        }
    }

    /// Builds a matched selection descriptor.
    pub const fn matched(policy_index: usize) -> Self {
        Self {
            abi_version: GERBIL_DECK_ROUTE_ABI_VERSION,
            matched: 1,
            reserved: [0; 3],
            policy_index,
        }
    }

    fn matched_index(self) -> Option<usize> {
        if self.matched == 0 {
            None
        } else {
            Some(self.policy_index)
        }
    }

    fn validate(self) -> Result<(), GerbilDeckRouteAbiError> {
        if self.matched == 0 && self.policy_index != Self::NO_POLICY_INDEX {
            return Err(GerbilDeckRouteAbiError::InvalidUnmatchedPolicyIndex {
                policy_index: self.policy_index,
            });
        }
        Ok(())
    }
}

/// Status code returned by the native Scheme `Deck` runtime selector.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRouteAbiStatus(i32);

impl GerbilDeckRouteAbiStatus {
    /// Successful native selector status code.
    pub const OK: Self = Self(GERBIL_DECK_ROUTE_ABI_STATUS_OK);
    /// Native selector status code for null pointer inputs.
    pub const NULL_POINTER: Self = Self(GERBIL_DECK_ROUTE_ABI_STATUS_NULL_POINTER);
    /// Native selector status code for request ABI version mismatch.
    pub const ABI_MISMATCH: Self = Self(GERBIL_DECK_ROUTE_ABI_STATUS_ABI_MISMATCH);
    /// Native selector status code for an invalid output selection.
    pub const INVALID_SELECTION: Self = Self(GERBIL_DECK_ROUTE_ABI_STATUS_INVALID_SELECTION);

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

/// Native selector function exported by the Scheme `Deck` runtime.
pub type GerbilDeckRouteAbiSelectModelRouteFn = unsafe extern "C" fn(
    request: *const GerbilDeckRouteAbiModelRouteRequest,
    selection: *mut GerbilDeckRouteAbiModelRouteSelection,
) -> GerbilDeckRouteAbiStatus;

/// Native function used to initialize the linked Scheme runtime once per process.
pub type GerbilDeckRouteAbiInitializeFn = unsafe extern "C" fn() -> i32;

/// Safe Rust wrapper around a Scheme-owned native model-route selector.
#[derive(Clone, Copy)]
pub struct GerbilDeckRouteAbiModelRouteSelector {
    initialize: Option<GerbilDeckRouteAbiInitializeFn>,
    select_model_route: GerbilDeckRouteAbiSelectModelRouteFn,
}

impl GerbilDeckRouteAbiModelRouteSelector {
    /// Builds a selector from native function pointers exported by the runtime.
    pub const fn new(select_model_route: GerbilDeckRouteAbiSelectModelRouteFn) -> Self {
        Self {
            initialize: None,
            select_model_route,
        }
    }

    /// Builds a selector that initializes a linked native Scheme runtime before use.
    pub const fn with_initializer(
        initialize: GerbilDeckRouteAbiInitializeFn,
        select_model_route: GerbilDeckRouteAbiSelectModelRouteFn,
    ) -> Self {
        Self {
            initialize: Some(initialize),
            select_model_route,
        }
    }

    /// Evaluates a model-route request through the native Scheme selector.
    pub fn evaluate(
        &self,
        request: &GerbilDeckRuntimeModelRoutePolicyRequest,
    ) -> Result<GerbilDeckRuntimeModelRouteSelectionReceipt, GerbilDeckRouteAbiError> {
        let policy_index = self.select_policy_index(request)?;
        let policy = policy_index
            .map(|index| {
                let policy = request.policies.get(index).ok_or(
                    GerbilDeckRouteAbiError::InvalidPolicyIndex {
                        index,
                        policies_len: request.policies.len(),
                    },
                )?;
                Ok(GerbilDeckRuntimeModelRouteSelectedPolicy {
                    kind: GerbilDeckRuntimeSelectedPolicyKind::from(
                        "marlin-deck-runtime.model-route-policy.v1",
                    ),
                    name: policy.name.clone(),
                    provider: policy.provider.clone(),
                    model: policy.model.clone(),
                    command_prefixes: policy.command_prefixes.clone(),
                    agent_scopes: policy.agent_scopes.clone(),
                    context_mode: policy.context_mode.clone(),
                    isolation_mode: policy.isolation_mode.clone(),
                })
            })
            .transpose()?;

        Ok(GerbilDeckRuntimeModelRouteSelectionReceipt {
            schema_id: GERBIL_DECK_RUNTIME_MODEL_ROUTE_SELECTION_SCHEMA_ID.to_string(),
            command: request.command.clone(),
            agent_scope: request.agent_scope.clone(),
            matched: policy.is_some(),
            route_abi: GerbilDeckRouteAbiReceipt::ready(
                GERBIL_DECK_ROUTE_ABI_ID,
                GERBIL_DECK_ROUTE_ABI_VERSION,
            ),
            policy,
        })
    }

    /// Selects the matching model-route policy index through the typed native ABI.
    pub fn select_policy_index(
        &self,
        request: &GerbilDeckRuntimeModelRoutePolicyRequest,
    ) -> Result<Option<usize>, GerbilDeckRouteAbiError> {
        self.initialize_runtime()?;
        let request_backing = GerbilDeckRouteAbiModelRouteRequestBacking::new(request);
        let raw_request = request_backing.raw_request();
        let mut selection = GerbilDeckRouteAbiModelRouteSelection::empty();
        let status = unsafe { (self.select_model_route)(&raw_request, &mut selection as *mut _) };

        if !status.is_ok() {
            return Err(GerbilDeckRouteAbiError::RuntimeStatus {
                code: status.code(),
            });
        }

        Self::policy_index_from_native_selection(request, selection)
    }

    fn initialize_runtime(&self) -> Result<(), GerbilDeckRouteAbiError> {
        let Some(initialize) = self.initialize else {
            return Ok(());
        };

        let code = unsafe { initialize() };
        if code == 0 {
            Ok(())
        } else {
            Err(GerbilDeckRouteAbiError::RuntimeInit { code })
        }
    }

    fn policy_index_from_native_selection(
        request: &GerbilDeckRuntimeModelRoutePolicyRequest,
        selection: GerbilDeckRouteAbiModelRouteSelection,
    ) -> Result<Option<usize>, GerbilDeckRouteAbiError> {
        if selection.abi_version != GERBIL_DECK_ROUTE_ABI_VERSION {
            return Err(GerbilDeckRouteAbiError::OutputAbiVersion {
                expected: GERBIL_DECK_ROUTE_ABI_VERSION,
                actual: selection.abi_version,
            });
        }
        selection.validate()?;

        if let Some(index) = selection.matched_index()
            && index >= request.policies.len()
        {
            return Err(GerbilDeckRouteAbiError::InvalidPolicyIndex {
                index,
                policies_len: request.policies.len(),
            });
        }

        Ok(selection.matched_index())
    }
}

/// Error raised while invoking the native `Deck` runtime ABI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GerbilDeckRouteAbiError {
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
    /// The native runtime selected a policy index outside the request policy array.
    InvalidPolicyIndex {
        /// Selected policy index.
        index: usize,
        /// Number of policies in the request.
        policies_len: usize,
    },
    /// The native runtime reported no match but wrote a concrete policy index.
    InvalidUnmatchedPolicyIndex {
        /// Policy index written alongside an unmatched result.
        policy_index: usize,
    },
}

impl Display for GerbilDeckRouteAbiError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeInit { code } => {
                write!(
                    formatter,
                    "deck runtime native init failed with status {code}"
                )
            }
            Self::RuntimeStatus { code } => write!(
                formatter,
                "deck runtime native selector failed with status {code}"
            ),
            Self::OutputAbiVersion { expected, actual } => write!(
                formatter,
                "deck runtime native selector wrote ABI version {actual}, expected {expected}"
            ),
            Self::InvalidPolicyIndex {
                index,
                policies_len,
            } => write!(
                formatter,
                "deck runtime native selector returned policy index {index}, but request has {policies_len} policies"
            ),
            Self::InvalidUnmatchedPolicyIndex { policy_index } => write!(
                formatter,
                "deck runtime native selector returned unmatched result with policy index {policy_index}"
            ),
        }
    }
}

impl Error for GerbilDeckRouteAbiError {}

struct GerbilDeckRouteAbiStringBacking {
    bytes: Vec<u8>,
}

impl GerbilDeckRouteAbiStringBacking {
    fn new(value: &str) -> Self {
        Self {
            bytes: value.as_bytes().to_vec(),
        }
    }

    fn raw(&self) -> GerbilDeckRouteAbiUtf8 {
        GerbilDeckRouteAbiUtf8::from_bytes(&self.bytes)
    }
}

struct GerbilDeckRouteAbiStringListBacking {
    values: Vec<GerbilDeckRouteAbiStringBacking>,
    raw_items: Vec<GerbilDeckRouteAbiUtf8>,
}

impl GerbilDeckRouteAbiStringListBacking {
    fn new(values: &[String]) -> Self {
        let values = values
            .iter()
            .map(|value| GerbilDeckRouteAbiStringBacking::new(value))
            .collect::<Vec<_>>();
        let raw_items = values.iter().map(|value| value.raw()).collect::<Vec<_>>();
        Self { values, raw_items }
    }

    fn raw(&self) -> GerbilDeckRouteAbiUtf8List {
        let _keep_values_alive = self.values.len();
        GerbilDeckRouteAbiUtf8List::from_items(&self.raw_items)
    }
}

struct GerbilDeckRouteAbiPolicyBacking {
    name: GerbilDeckRouteAbiStringBacking,
    provider: GerbilDeckRouteAbiStringBacking,
    model: GerbilDeckRouteAbiStringBacking,
    command_prefixes: GerbilDeckRouteAbiStringListBacking,
    agent_scopes: GerbilDeckRouteAbiStringListBacking,
    context_mode: GerbilDeckRouteAbiStringBacking,
    isolation_mode: GerbilDeckRouteAbiStringBacking,
}

impl GerbilDeckRouteAbiPolicyBacking {
    fn new(policy: &crate::GerbilDeckRuntimeModelRoutePolicy) -> Self {
        Self {
            name: GerbilDeckRouteAbiStringBacking::new(&policy.name),
            provider: GerbilDeckRouteAbiStringBacking::new(&policy.provider),
            model: GerbilDeckRouteAbiStringBacking::new(&policy.model),
            command_prefixes: GerbilDeckRouteAbiStringListBacking::new(&policy.command_prefixes),
            agent_scopes: GerbilDeckRouteAbiStringListBacking::new(&policy.agent_scopes),
            context_mode: GerbilDeckRouteAbiStringBacking::new(policy.context_mode.as_str()),
            isolation_mode: GerbilDeckRouteAbiStringBacking::new(policy.isolation_mode.as_str()),
        }
    }

    fn raw(&self) -> GerbilDeckRouteAbiModelRoutePolicy {
        GerbilDeckRouteAbiModelRoutePolicy {
            name: self.name.raw(),
            provider: self.provider.raw(),
            model: self.model.raw(),
            command_prefixes: self.command_prefixes.raw(),
            agent_scopes: self.agent_scopes.raw(),
            context_mode: self.context_mode.raw(),
            isolation_mode: self.isolation_mode.raw(),
        }
    }
}

struct GerbilDeckRouteAbiModelRouteRequestBacking {
    command: GerbilDeckRouteAbiStringBacking,
    agent_scope: GerbilDeckRouteAbiStringBacking,
    policies: Vec<GerbilDeckRouteAbiPolicyBacking>,
    raw_policies: Vec<GerbilDeckRouteAbiModelRoutePolicy>,
}

impl GerbilDeckRouteAbiModelRouteRequestBacking {
    fn new(request: &GerbilDeckRuntimeModelRoutePolicyRequest) -> Self {
        let policies = request
            .policies
            .iter()
            .map(GerbilDeckRouteAbiPolicyBacking::new)
            .collect::<Vec<_>>();
        let raw_policies = policies
            .iter()
            .map(|policy| policy.raw())
            .collect::<Vec<_>>();
        Self {
            command: GerbilDeckRouteAbiStringBacking::new(&request.command),
            agent_scope: GerbilDeckRouteAbiStringBacking::new(&request.agent_scope),
            policies,
            raw_policies,
        }
    }

    fn raw_request(&self) -> GerbilDeckRouteAbiModelRouteRequest {
        let _keep_policies_alive = self.policies.len();
        GerbilDeckRouteAbiModelRouteRequest {
            abi_version: GERBIL_DECK_ROUTE_ABI_VERSION,
            command: self.command.raw(),
            agent_scope: self.agent_scope.raw(),
            policies: self.raw_policies.as_ptr(),
            policies_len: self.raw_policies.len(),
        }
    }
}

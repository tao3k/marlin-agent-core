//! Native C ABI boundary for the Deck runtime model-route selector.
//!
//! The Scheme runtime owns policy selection semantics. Rust owns only the C ABI
//! safety wrapper and projects the typed native result into Rust protocol types.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::{
    GerbilDeckRuntimeModelRoutePolicyRequest, GerbilDeckRuntimeModelRouteSelectedPolicy,
    GerbilDeckRuntimeModelRouteSelectionReceipt, GerbilDeckRuntimeSelectedPolicyKind,
};

/// Current `Deck` runtime native ABI version accepted by Rust.
pub const GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION: u32 = 1;
/// Native selector status code for success.
pub const GERBIL_DECK_RUNTIME_NATIVE_STATUS_OK: i32 = 0;
/// Native selector status code for null pointer inputs.
pub const GERBIL_DECK_RUNTIME_NATIVE_STATUS_NULL_POINTER: i32 = 2;
/// Native selector status code for request ABI version mismatch.
pub const GERBIL_DECK_RUNTIME_NATIVE_STATUS_ABI_MISMATCH: i32 = 3;
/// Native selector status code for an invalid output selection.
pub const GERBIL_DECK_RUNTIME_NATIVE_STATUS_INVALID_SELECTION: i32 = 4;
/// Native selector policy index sentinel used when no policy matched.
pub const GERBIL_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX: usize = usize::MAX;
/// Relative path of the C header that defines the native `Deck` runtime ABI.
pub const GERBIL_DECK_RUNTIME_NATIVE_HEADER_PATH: &str = "include/marlin_deck_runtime_native.h";
/// Source text of the C header that defines the native `Deck` runtime ABI.
pub const GERBIL_DECK_RUNTIME_NATIVE_HEADER_SOURCE: &str =
    include_str!("../include/marlin_deck_runtime_native.h");

/// Borrowed UTF-8 bytes passed across the native C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeUtf8 {
    /// Pointer to the first UTF-8 byte, or null when `len` is zero.
    pub ptr: *const u8,
    /// Number of UTF-8 bytes available at `ptr`.
    pub len: usize,
}

impl GerbilDeckRuntimeNativeUtf8 {
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
pub struct GerbilDeckRuntimeNativeUtf8List {
    /// Pointer to the first UTF-8 slice descriptor, or null when `len` is zero.
    pub items: *const GerbilDeckRuntimeNativeUtf8,
    /// Number of UTF-8 slice descriptors available at `items`.
    pub len: usize,
}

impl GerbilDeckRuntimeNativeUtf8List {
    /// Builds an empty borrowed UTF-8 slice list for C ABI calls.
    pub const fn empty() -> Self {
        Self {
            items: std::ptr::null(),
            len: 0,
        }
    }

    fn from_items(items: &[GerbilDeckRuntimeNativeUtf8]) -> Self {
        Self {
            items: items.as_ptr(),
            len: items.len(),
        }
    }
}

/// Native model-route policy shape consumed by the Scheme `Deck` runtime.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeModelRoutePolicy {
    /// Stable policy name.
    pub name: GerbilDeckRuntimeNativeUtf8,
    /// Provider identity, such as `openai` or `anthropic`.
    pub provider: GerbilDeckRuntimeNativeUtf8,
    /// Provider-owned model name.
    pub model: GerbilDeckRuntimeNativeUtf8,
    /// Command prefixes matched by the Scheme policy selector.
    pub command_prefixes: GerbilDeckRuntimeNativeUtf8List,
    /// Agent scopes matched by the Scheme policy selector.
    pub agent_scopes: GerbilDeckRuntimeNativeUtf8List,
    /// Context propagation mode selected by the Scheme policy.
    pub context_mode: GerbilDeckRuntimeNativeUtf8,
    /// Runtime isolation mode selected by the Scheme policy.
    pub isolation_mode: GerbilDeckRuntimeNativeUtf8,
}

/// Native model-route request shape passed from Rust to the Scheme `Deck` runtime.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeModelRouteRequest {
    /// ABI version expected by the Rust wrapper.
    pub abi_version: u32,
    /// Command line being classified.
    pub command: GerbilDeckRuntimeNativeUtf8,
    /// Agent scope being classified.
    pub agent_scope: GerbilDeckRuntimeNativeUtf8,
    /// Pointer to policy descriptors owned by the caller for the duration of the call.
    pub policies: *const GerbilDeckRuntimeNativeModelRoutePolicy,
    /// Number of policy descriptors available at `policies`.
    pub policies_len: usize,
}

/// Typed model-route selection returned from the native Scheme selector.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeModelRouteSelection {
    /// ABI version written by the native runtime.
    pub abi_version: u32,
    /// Non-zero when a policy matched.
    pub matched: u8,
    /// Reserved padding for stable C layout.
    pub reserved: [u8; 3],
    /// Index into the request policy array when `matched` is non-zero.
    pub policy_index: usize,
}

impl GerbilDeckRuntimeNativeModelRouteSelection {
    /// Sentinel policy index used when no policy matched.
    pub const NO_POLICY_INDEX: usize = GERBIL_DECK_RUNTIME_NATIVE_NO_POLICY_INDEX;

    /// Builds an empty unmatched selection descriptor.
    pub const fn empty() -> Self {
        Self {
            abi_version: GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
            matched: 0,
            reserved: [0; 3],
            policy_index: Self::NO_POLICY_INDEX,
        }
    }

    /// Builds a matched selection descriptor.
    pub const fn matched(policy_index: usize) -> Self {
        Self {
            abi_version: GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
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

    fn validate(self) -> Result<(), GerbilDeckRuntimeNativeAbiError> {
        if self.matched == 0 && self.policy_index != Self::NO_POLICY_INDEX {
            return Err(
                GerbilDeckRuntimeNativeAbiError::InvalidUnmatchedPolicyIndex {
                    policy_index: self.policy_index,
                },
            );
        }
        Ok(())
    }
}

/// Status code returned by the native Scheme `Deck` runtime selector.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GerbilDeckRuntimeNativeStatus(i32);

impl GerbilDeckRuntimeNativeStatus {
    /// Successful native selector status code.
    pub const OK: Self = Self(GERBIL_DECK_RUNTIME_NATIVE_STATUS_OK);
    /// Native selector status code for null pointer inputs.
    pub const NULL_POINTER: Self = Self(GERBIL_DECK_RUNTIME_NATIVE_STATUS_NULL_POINTER);
    /// Native selector status code for request ABI version mismatch.
    pub const ABI_MISMATCH: Self = Self(GERBIL_DECK_RUNTIME_NATIVE_STATUS_ABI_MISMATCH);
    /// Native selector status code for an invalid output selection.
    pub const INVALID_SELECTION: Self = Self(GERBIL_DECK_RUNTIME_NATIVE_STATUS_INVALID_SELECTION);

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
pub type GerbilDeckRuntimeNativeSelectModelRouteFn =
    unsafe extern "C" fn(
        request: *const GerbilDeckRuntimeNativeModelRouteRequest,
        selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
    ) -> GerbilDeckRuntimeNativeStatus;

/// Native function used to initialize the linked Scheme runtime once per process.
pub type GerbilDeckRuntimeNativeInitializeFn = unsafe extern "C" fn() -> i32;

/// Safe Rust wrapper around a Scheme-owned native model-route selector.
#[derive(Clone, Copy)]
pub struct GerbilDeckRuntimeNativeModelRouteSelector {
    initialize: Option<GerbilDeckRuntimeNativeInitializeFn>,
    select_model_route: GerbilDeckRuntimeNativeSelectModelRouteFn,
}

impl GerbilDeckRuntimeNativeModelRouteSelector {
    /// Builds a selector from native function pointers exported by the runtime.
    pub const fn new(select_model_route: GerbilDeckRuntimeNativeSelectModelRouteFn) -> Self {
        Self {
            initialize: None,
            select_model_route,
        }
    }

    /// Builds a selector that initializes a linked native Scheme runtime before use.
    pub const fn with_initializer(
        initialize: GerbilDeckRuntimeNativeInitializeFn,
        select_model_route: GerbilDeckRuntimeNativeSelectModelRouteFn,
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
    ) -> Result<GerbilDeckRuntimeModelRouteSelectionReceipt, GerbilDeckRuntimeNativeAbiError> {
        let policy_index = self.select_policy_index(request)?;
        let policy = policy_index
            .map(|index| {
                let policy = request.policies.get(index).ok_or(
                    GerbilDeckRuntimeNativeAbiError::InvalidPolicyIndex {
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
            schema_id: "marlin-deck-runtime.model-route-selection.v1".to_string(),
            command: request.command.clone(),
            agent_scope: request.agent_scope.clone(),
            matched: policy.is_some(),
            policy,
        })
    }

    /// Selects the matching model-route policy index through the typed native ABI.
    pub fn select_policy_index(
        &self,
        request: &GerbilDeckRuntimeModelRoutePolicyRequest,
    ) -> Result<Option<usize>, GerbilDeckRuntimeNativeAbiError> {
        self.initialize_runtime()?;
        let request_backing = GerbilDeckRuntimeNativeModelRouteRequestBacking::new(request);
        let raw_request = request_backing.raw_request();
        let mut selection = GerbilDeckRuntimeNativeModelRouteSelection::empty();
        let status = unsafe { (self.select_model_route)(&raw_request, &mut selection as *mut _) };

        if !status.is_ok() {
            return Err(GerbilDeckRuntimeNativeAbiError::RuntimeStatus {
                code: status.code(),
            });
        }

        Self::policy_index_from_native_selection(request, selection)
    }

    fn initialize_runtime(&self) -> Result<(), GerbilDeckRuntimeNativeAbiError> {
        let Some(initialize) = self.initialize else {
            return Ok(());
        };

        let code = unsafe { initialize() };
        if code == 0 {
            Ok(())
        } else {
            Err(GerbilDeckRuntimeNativeAbiError::RuntimeInit { code })
        }
    }

    fn policy_index_from_native_selection(
        request: &GerbilDeckRuntimeModelRoutePolicyRequest,
        selection: GerbilDeckRuntimeNativeModelRouteSelection,
    ) -> Result<Option<usize>, GerbilDeckRuntimeNativeAbiError> {
        if selection.abi_version != GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION {
            return Err(GerbilDeckRuntimeNativeAbiError::OutputAbiVersion {
                expected: GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
                actual: selection.abi_version,
            });
        }
        selection.validate()?;

        if let Some(index) = selection.matched_index()
            && index >= request.policies.len()
        {
            return Err(GerbilDeckRuntimeNativeAbiError::InvalidPolicyIndex {
                index,
                policies_len: request.policies.len(),
            });
        }

        Ok(selection.matched_index())
    }
}

/// Error raised while invoking the native `Deck` runtime ABI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GerbilDeckRuntimeNativeAbiError {
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

impl Display for GerbilDeckRuntimeNativeAbiError {
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

impl Error for GerbilDeckRuntimeNativeAbiError {}

struct GerbilDeckRuntimeNativeStringBacking {
    bytes: Vec<u8>,
}

impl GerbilDeckRuntimeNativeStringBacking {
    fn new(value: &str) -> Self {
        Self {
            bytes: value.as_bytes().to_vec(),
        }
    }

    fn raw(&self) -> GerbilDeckRuntimeNativeUtf8 {
        GerbilDeckRuntimeNativeUtf8::from_bytes(&self.bytes)
    }
}

struct GerbilDeckRuntimeNativeStringListBacking {
    values: Vec<GerbilDeckRuntimeNativeStringBacking>,
    raw_items: Vec<GerbilDeckRuntimeNativeUtf8>,
}

impl GerbilDeckRuntimeNativeStringListBacking {
    fn new(values: &[String]) -> Self {
        let values = values
            .iter()
            .map(|value| GerbilDeckRuntimeNativeStringBacking::new(value))
            .collect::<Vec<_>>();
        let raw_items = values.iter().map(|value| value.raw()).collect::<Vec<_>>();
        Self { values, raw_items }
    }

    fn raw(&self) -> GerbilDeckRuntimeNativeUtf8List {
        let _keep_values_alive = self.values.len();
        GerbilDeckRuntimeNativeUtf8List::from_items(&self.raw_items)
    }
}

struct GerbilDeckRuntimeNativePolicyBacking {
    name: GerbilDeckRuntimeNativeStringBacking,
    provider: GerbilDeckRuntimeNativeStringBacking,
    model: GerbilDeckRuntimeNativeStringBacking,
    command_prefixes: GerbilDeckRuntimeNativeStringListBacking,
    agent_scopes: GerbilDeckRuntimeNativeStringListBacking,
    context_mode: GerbilDeckRuntimeNativeStringBacking,
    isolation_mode: GerbilDeckRuntimeNativeStringBacking,
}

impl GerbilDeckRuntimeNativePolicyBacking {
    fn new(policy: &crate::GerbilDeckRuntimeModelRoutePolicy) -> Self {
        Self {
            name: GerbilDeckRuntimeNativeStringBacking::new(&policy.name),
            provider: GerbilDeckRuntimeNativeStringBacking::new(&policy.provider),
            model: GerbilDeckRuntimeNativeStringBacking::new(&policy.model),
            command_prefixes: GerbilDeckRuntimeNativeStringListBacking::new(
                &policy.command_prefixes,
            ),
            agent_scopes: GerbilDeckRuntimeNativeStringListBacking::new(&policy.agent_scopes),
            context_mode: GerbilDeckRuntimeNativeStringBacking::new(policy.context_mode.as_str()),
            isolation_mode: GerbilDeckRuntimeNativeStringBacking::new(
                policy.isolation_mode.as_str(),
            ),
        }
    }

    fn raw(&self) -> GerbilDeckRuntimeNativeModelRoutePolicy {
        GerbilDeckRuntimeNativeModelRoutePolicy {
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

struct GerbilDeckRuntimeNativeModelRouteRequestBacking {
    command: GerbilDeckRuntimeNativeStringBacking,
    agent_scope: GerbilDeckRuntimeNativeStringBacking,
    policies: Vec<GerbilDeckRuntimeNativePolicyBacking>,
    raw_policies: Vec<GerbilDeckRuntimeNativeModelRoutePolicy>,
}

impl GerbilDeckRuntimeNativeModelRouteRequestBacking {
    fn new(request: &GerbilDeckRuntimeModelRoutePolicyRequest) -> Self {
        let policies = request
            .policies
            .iter()
            .map(GerbilDeckRuntimeNativePolicyBacking::new)
            .collect::<Vec<_>>();
        let raw_policies = policies
            .iter()
            .map(|policy| policy.raw())
            .collect::<Vec<_>>();
        Self {
            command: GerbilDeckRuntimeNativeStringBacking::new(&request.command),
            agent_scope: GerbilDeckRuntimeNativeStringBacking::new(&request.agent_scope),
            policies,
            raw_policies,
        }
    }

    fn raw_request(&self) -> GerbilDeckRuntimeNativeModelRouteRequest {
        let _keep_policies_alive = self.policies.len();
        GerbilDeckRuntimeNativeModelRouteRequest {
            abi_version: GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
            command: self.command.raw(),
            agent_scope: self.agent_scope.raw(),
            policies: self.raw_policies.as_ptr(),
            policies_len: self.raw_policies.len(),
        }
    }
}

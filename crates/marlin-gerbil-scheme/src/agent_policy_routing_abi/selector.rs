//! Safe selector wrapper for linked Gerbil AgentGraph policy routing code.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    slice, str,
};

use marlin_agent_graph::AgentPolicyRoutingReceipt;

use crate::{
    GerbilSchemeAbiProjectionReceipt, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeId, GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    agent_policy_routing::{
        GERBIL_AGENT_POLICY_ROUTING_SCHEMA_ID, GERBIL_AGENT_POLICY_ROUTING_TYPE_ID,
        gerbil_agent_policy_routing_type_manifest, project_gerbil_agent_policy_routing_abi_receipt,
    },
};

use super::{
    abi::{
        GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION, GerbilAgentPolicyRoutingAbiInitializeFn,
        GerbilAgentPolicyRoutingAbiProjection, GerbilAgentPolicyRoutingAbiSelectEdgesFn,
        GerbilAgentPolicyRoutingAbiUtf8,
    },
    request::{
        GerbilAgentPolicyRoutingAbiEpochBacking, GerbilAgentPolicyRoutingAbiPayload,
        GerbilAgentPolicyRoutingAbiPayloadBacking, GerbilAgentPolicyRoutingAbiRequestBacking,
        GerbilAgentPolicyRoutingAbiRequestView, GerbilAgentPolicyRoutingAbiSelectEdgesRequest,
        agent_policy_routing_evidence_kind_as_str,
    },
};

/// Safe Rust wrapper around a Gerbil-owned native AgentGraph policy selector.
#[derive(Clone, Copy)]
pub struct GerbilAgentPolicyRoutingAbiSelector {
    initialize: Option<GerbilAgentPolicyRoutingAbiInitializeFn>,
    select_edges: GerbilAgentPolicyRoutingAbiSelectEdgesFn,
}

impl GerbilAgentPolicyRoutingAbiSelector {
    /// Builds a selector from the native function exported by linked Gerbil code.
    pub const fn new(select_edges: GerbilAgentPolicyRoutingAbiSelectEdgesFn) -> Self {
        Self {
            initialize: None,
            select_edges,
        }
    }

    /// Builds a selector that initializes a linked native Scheme runtime before use.
    pub const fn with_initializer(
        initialize: GerbilAgentPolicyRoutingAbiInitializeFn,
        select_edges: GerbilAgentPolicyRoutingAbiSelectEdgesFn,
    ) -> Self {
        Self {
            initialize: Some(initialize),
            select_edges,
        }
    }

    /// Calls the linked native selector and returns the Rust-owned typed Scheme projection.
    pub fn project_typed_value(
        &self,
        request: &GerbilAgentPolicyRoutingAbiSelectEdgesRequest,
    ) -> Result<GerbilSchemeTypedValue, GerbilAgentPolicyRoutingAbiError> {
        self.initialize_runtime()?;

        let request_backing = GerbilAgentPolicyRoutingAbiRequestBacking::new(request);
        let raw_request = request_backing.raw_request();
        let mut projection = GerbilAgentPolicyRoutingAbiProjection::empty();
        let status = unsafe { (self.select_edges)(&raw_request, &mut projection as *mut _) };

        if !status.is_ok() {
            return Err(GerbilAgentPolicyRoutingAbiError::RuntimeStatus {
                code: status.code(),
            });
        }

        self.typed_value_from_abi_projection(request.view(), projection)
    }

    /// Calls the linked native selector using epoch-owned match-key backing.
    pub fn project_typed_value_with_epoch_backing(
        &self,
        epoch_backing: &GerbilAgentPolicyRoutingAbiEpochBacking,
        payload: &GerbilAgentPolicyRoutingAbiPayload,
    ) -> Result<GerbilSchemeTypedValue, GerbilAgentPolicyRoutingAbiError> {
        self.initialize_runtime()?;

        let payload_backing = GerbilAgentPolicyRoutingAbiPayloadBacking::new(payload);
        let raw_request = epoch_backing.raw_request(&payload_backing);
        let mut projection = GerbilAgentPolicyRoutingAbiProjection::empty();
        let status = unsafe { (self.select_edges)(&raw_request, &mut projection as *mut _) };

        if !status.is_ok() {
            return Err(GerbilAgentPolicyRoutingAbiError::RuntimeStatus {
                code: status.code(),
            });
        }

        self.typed_value_from_abi_projection(
            GerbilAgentPolicyRoutingAbiRequestView::from_parts(epoch_backing.match_key(), payload),
            projection,
        )
    }

    /// Calls the native selector and projects the result into a typed Rust receipt.
    pub fn project_policy_receipt(
        &self,
        request: &GerbilAgentPolicyRoutingAbiSelectEdgesRequest,
    ) -> Result<
        (GerbilSchemeAbiProjectionReceipt, AgentPolicyRoutingReceipt),
        GerbilAgentPolicyRoutingAbiError,
    > {
        let typed_value = self.project_typed_value(request)?;
        let registry = GerbilSchemeTypeRegistry::new(gerbil_agent_policy_routing_type_manifest())
            .map_err(GerbilAgentPolicyRoutingAbiError::from_scheme_decode)?;
        project_gerbil_agent_policy_routing_abi_receipt(&registry, &typed_value)
            .map_err(GerbilAgentPolicyRoutingAbiError::from_scheme_decode)
    }

    /// Calls the native selector with epoch-owned backing and projects a typed Rust receipt.
    pub fn project_policy_receipt_with_epoch_backing(
        &self,
        epoch_backing: &GerbilAgentPolicyRoutingAbiEpochBacking,
        payload: &GerbilAgentPolicyRoutingAbiPayload,
    ) -> Result<
        (GerbilSchemeAbiProjectionReceipt, AgentPolicyRoutingReceipt),
        GerbilAgentPolicyRoutingAbiError,
    > {
        let typed_value = self.project_typed_value_with_epoch_backing(epoch_backing, payload)?;
        let registry = GerbilSchemeTypeRegistry::new(gerbil_agent_policy_routing_type_manifest())
            .map_err(GerbilAgentPolicyRoutingAbiError::from_scheme_decode)?;
        project_gerbil_agent_policy_routing_abi_receipt(&registry, &typed_value)
            .map_err(GerbilAgentPolicyRoutingAbiError::from_scheme_decode)
    }

    fn initialize_runtime(&self) -> Result<(), GerbilAgentPolicyRoutingAbiError> {
        let Some(initialize) = self.initialize else {
            return Ok(());
        };

        let code = unsafe { initialize() };
        if code == 0 {
            Ok(())
        } else {
            Err(GerbilAgentPolicyRoutingAbiError::RuntimeInit { code })
        }
    }

    fn typed_value_from_abi_projection(
        &self,
        request: GerbilAgentPolicyRoutingAbiRequestView<'_>,
        projection: GerbilAgentPolicyRoutingAbiProjection,
    ) -> Result<GerbilSchemeTypedValue, GerbilAgentPolicyRoutingAbiError> {
        if projection.abi_version != GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION {
            return Err(GerbilAgentPolicyRoutingAbiError::OutputAbiVersion {
                expected: GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
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
                ("graph_id", GerbilSchemeValue::from(request.graph_id)),
                (
                    "policy_scope",
                    GerbilSchemeValue::from(request.policy_scope),
                ),
                ("root_node", GerbilSchemeValue::from(request.root_node)),
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
                            .map(|edge| GerbilSchemeValue::from(edge.as_str())),
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
                                GerbilSchemeValue::from(evidence.evidence_id.as_str()),
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
pub enum GerbilAgentPolicyRoutingAbiError {
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

impl GerbilAgentPolicyRoutingAbiError {
    fn from_scheme_decode(error: GerbilSchemeTypeDecodeError) -> Self {
        Self::TypeProjection {
            message: error.to_string(),
        }
    }
}

impl Display for GerbilAgentPolicyRoutingAbiError {
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

impl Error for GerbilAgentPolicyRoutingAbiError {}

fn native_utf8_to_string(
    field: &'static str,
    value: GerbilAgentPolicyRoutingAbiUtf8,
) -> Result<String, GerbilAgentPolicyRoutingAbiError> {
    if value.ptr.is_null() {
        if value.len == 0 {
            return Ok(String::new());
        }
        return Err(GerbilAgentPolicyRoutingAbiError::InvalidUtf8 {
            field,
            message: "null pointer with non-zero length".to_string(),
        });
    }

    let bytes = unsafe { slice::from_raw_parts(value.ptr, value.len) };
    str::from_utf8(bytes).map(str::to_owned).map_err(|error| {
        GerbilAgentPolicyRoutingAbiError::InvalidUtf8 {
            field,
            message: error.to_string(),
        }
    })
}

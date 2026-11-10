//! Gerbil `POO` graph-loop continuation projection into Rust controller actions.

use marlin_agent_protocol::{
    GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID, GerbilLoopGraphContinuationRequest,
    GraphLoopNextAction, compile_gerbil_loop_graph_continuation,
};

use crate::{
    GerbilSchemeAbiProjectionReceipt, GerbilSchemeAbiProjectionRequest, GerbilSchemeAbiSymbol,
    GerbilSchemeCAbiContract, GerbilSchemeCAbiId, GerbilSchemeCAbiReadinessPlan,
    GerbilSchemeFieldName, GerbilSchemePackageId, GerbilSchemePackageManifest,
    GerbilSchemeProjectionContract, GerbilSchemeSchemaId, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId, GerbilSchemeTypeManifest,
    GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec, GerbilSchemeTypedProjection,
    GerbilSchemeTypedValue, decode_gerbil_scheme_abi_projection,
};

/// Native ABI id for graph-loop continuation typed projections.
pub const GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_ID: &str =
    "marlin.agent.gerbil-loop-continuation.native-projection";

/// Package id for the graph-loop continuation typed projection ABI manifest.
pub const GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_PACKAGE_ID: &str =
    "marlin.agent.gerbil-loop-continuation.native-projection";

/// Native ABI version for graph-loop continuation typed projections.
pub const GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_VERSION: u32 = 1;

/// Scheme type identifier expected for graph-loop continuation decisions.
pub const GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID: &str = "marlin.agent.gerbil-loop-continuation";

/// Native symbol expected to project a Gerbil/POO continuation decision.
pub const GERBIL_LOOP_GRAPH_CONTINUATION_ABI_SYMBOL: &str =
    "marlin_graph_loop_continuation_next_action";

/// Projection contract for a Gerbil/POO graph-loop continuation decision.
pub fn gerbil_loop_graph_continuation_projection_contract() -> GerbilSchemeProjectionContract {
    GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
        GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID,
    ))
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID,
    ))
}

impl GerbilSchemeTypedProjection for GerbilLoopGraphContinuationRequest {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        gerbil_loop_graph_continuation_projection_contract()
    }
}

/// Scheme type manifest for graph-loop continuation projections.
pub fn gerbil_loop_graph_continuation_type_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![GerbilSchemeTypeSpec {
            type_id: GerbilSchemeTypeId::new(GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID),
            schema_id: Some(GerbilSchemeSchemaId::new(
                GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID,
            )),
            fields: vec![
                required_continuation_field("schema_id", "string", None),
                required_continuation_field("action", "object", None),
                optional_continuation_field("diagnostics", "array", Some("string")),
            ],
        }],
    }
}

/// Native ABI projection request for the Gerbil/POO continuation projection.
pub fn gerbil_loop_graph_continuation_abi_projection_request() -> GerbilSchemeAbiProjectionRequest {
    GerbilSchemeAbiProjectionRequest::new(
        GerbilSchemeCAbiId::new(GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_ID),
        GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_VERSION,
        GerbilSchemeAbiSymbol::new(GERBIL_LOOP_GRAPH_CONTINUATION_ABI_SYMBOL),
        gerbil_loop_graph_continuation_projection_contract(),
    )
}

/// Native ABI contract declared by the graph-loop continuation projection package.
pub fn gerbil_loop_graph_continuation_abi_projection_abi_contract() -> GerbilSchemeCAbiContract {
    GerbilSchemeCAbiContract::new(
        GerbilSchemeCAbiId::new(GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_ID),
        GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_VERSION,
    )
    .with_exported_symbols([GerbilSchemeAbiSymbol::new(
        GERBIL_LOOP_GRAPH_CONTINUATION_ABI_SYMBOL,
    )])
}

/// Readiness plan expected before calling the continuation native projection ABI.
pub fn gerbil_loop_graph_continuation_abi_projection_readiness_plan()
-> GerbilSchemeCAbiReadinessPlan {
    GerbilSchemeCAbiReadinessPlan::new(
        GerbilSchemeCAbiId::new(GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_ID),
        GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_ABI_VERSION,
    )
    .with_exported_symbols([GerbilSchemeAbiSymbol::new(
        GERBIL_LOOP_GRAPH_CONTINUATION_ABI_SYMBOL,
    )])
}

/// Package manifest for the graph-loop continuation typed projection ABI.
pub fn gerbil_loop_graph_continuation_abi_projection_package_manifest()
-> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new(GERBIL_LOOP_GRAPH_CONTINUATION_ABI_PROJECTION_PACKAGE_ID),
        gerbil_loop_graph_continuation_type_manifest(),
    )
    .with_projection_contracts([gerbil_loop_graph_continuation_projection_contract()])
    .with_c_abi(gerbil_loop_graph_continuation_abi_projection_abi_contract())
}

/// Decode the typed Gerbil continuation projection without compiling the graph action.
pub fn decode_gerbil_loop_graph_continuation_projection(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GerbilLoopGraphContinuationRequest, GerbilSchemeTypeDecodeError> {
    registry.decode_projection(typed_value)
}

/// Decode a Gerbil/POO continuation projection returned by the native ABI.
pub fn decode_gerbil_loop_graph_continuation_abi_projection(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<
    (
        GerbilSchemeAbiProjectionReceipt,
        GerbilLoopGraphContinuationRequest,
    ),
    GerbilSchemeTypeDecodeError,
> {
    decode_gerbil_scheme_abi_projection(
        registry,
        &gerbil_loop_graph_continuation_abi_projection_readiness_plan(),
        &gerbil_loop_graph_continuation_abi_projection_request(),
        typed_value,
    )
}

/// Project a typed Gerbil continuation value into the controller next-action protocol.
pub fn project_gerbil_loop_graph_continuation_action(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<GraphLoopNextAction, GerbilSchemeTypeDecodeError> {
    let request = decode_gerbil_loop_graph_continuation_projection(registry, typed_value)?;
    compile_gerbil_loop_graph_continuation(request).map_err(|error| {
        GerbilSchemeTypeDecodeError::RustProjection {
            message: format!("Gerbil continuation graph failed Rust compilation: {error:?}"),
        }
    })
}

/// Project a native ABI Gerbil continuation value into the controller next-action protocol.
pub fn project_gerbil_loop_graph_continuation_abi_action(
    registry: &GerbilSchemeTypeRegistry,
    typed_value: &GerbilSchemeTypedValue,
) -> Result<(GerbilSchemeAbiProjectionReceipt, GraphLoopNextAction), GerbilSchemeTypeDecodeError> {
    let (receipt, request) =
        decode_gerbil_loop_graph_continuation_abi_projection(registry, typed_value)?;
    let next_action = compile_gerbil_loop_graph_continuation(request).map_err(|error| {
        GerbilSchemeTypeDecodeError::RustProjection {
            message: format!("Gerbil continuation graph failed Rust compilation: {error:?}"),
        }
    })?;
    Ok((receipt, next_action))
}

fn required_continuation_field(
    name: &str,
    type_id: &str,
    element_type_id: Option<&str>,
) -> GerbilSchemeTypeFieldSpec {
    continuation_field(name, type_id, element_type_id, true)
}

fn optional_continuation_field(
    name: &str,
    type_id: &str,
    element_type_id: Option<&str>,
) -> GerbilSchemeTypeFieldSpec {
    continuation_field(name, type_id, element_type_id, false)
}

fn continuation_field(
    name: &str,
    type_id: &str,
    element_type_id: Option<&str>,
    required: bool,
) -> GerbilSchemeTypeFieldSpec {
    GerbilSchemeTypeFieldSpec {
        name: GerbilSchemeFieldName::new(name),
        type_id: GerbilSchemeTypeId::new(type_id),
        element_type_id: element_type_id.map(GerbilSchemeTypeId::new),
        required,
        description: None,
    }
}

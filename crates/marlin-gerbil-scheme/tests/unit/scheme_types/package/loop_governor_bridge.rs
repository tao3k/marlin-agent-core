use marlin_gerbil_scheme::{
    GerbilSchemeFieldName, GerbilSchemePackageId, GerbilSchemePackageManifest,
    GerbilSchemeProjectionContract, GerbilSchemeSchemaId, GerbilSchemeTypeFieldSpec,
    GerbilSchemeTypeId, GerbilSchemeTypeManifest, GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec,
    GerbilSchemeTypedValue, GerbilSchemeValue, validate_gerbil_scheme_package_manifest,
};

const POO_FLOW_LOOP_GOVERNOR_MARLIN_ABI_SCHEMA: &str = "poo-flow.loop-governor.marlin-abi.v1";
const POO_FLOW_LOOP_GOVERNOR_MARLIN_REQUEST_SCHEMA: &str =
    "poo-flow.loop-governor.marlin-request.v1";
const POO_FLOW_LOOP_GOVERNOR_RUNTIME_MANIFEST_SCHEMA: &str =
    "poo-flow.loop-governor.marlin-runtime-manifest.v1";
const POO_FLOW_LOOP_ENGINE_DISCOVERY_SCHEMA: &str = "poo-flow.loop-engine.marlin-discovery.v1";

#[test]
fn loop_governor_bridge_package_manifest_declares_marlin_consumer_shape() {
    let manifest = loop_governor_bridge_package_manifest();

    let receipt = validate_gerbil_scheme_package_manifest(&manifest)
        .expect("loop governor bridge manifest should validate");

    assert_eq!(
        receipt.package_id,
        GerbilSchemePackageId::new("marlin.deck-runtime.loop-governor-bridge")
    );
    assert_eq!(receipt.type_count, 4);
    assert_eq!(receipt.projection_contract_count, 1);
    assert_eq!(receipt.native_abi_version, None);
    assert_eq!(receipt.native_symbol_count, 0);

    let runtime_manifest = manifest
        .type_manifest
        .type_spec(&runtime_manifest_type_id())
        .expect("runtime manifest type is registered");
    assert_eq!(
        runtime_manifest
            .schema_id
            .as_ref()
            .map(GerbilSchemeSchemaId::as_str),
        Some(POO_FLOW_LOOP_GOVERNOR_RUNTIME_MANIFEST_SCHEMA)
    );
    assert!(
        runtime_manifest
            .field(&GerbilSchemeFieldName::new("request_envelope"))
            .expect("request envelope field")
            .required
    );
    assert_eq!(
        runtime_manifest
            .field(&GerbilSchemeFieldName::new("request_envelope"))
            .expect("request envelope field")
            .type_id,
        request_envelope_type_id()
    );
}

#[test]
fn loop_governor_bridge_decodes_runtime_manifest_without_scheme_execution() {
    let registry = GerbilSchemeTypeRegistry::new(loop_governor_bridge_type_manifest())
        .expect("loop governor bridge registry should build");
    let envelope = GerbilSchemeTypedValue::new(
        runtime_manifest_type_id(),
        GerbilSchemeValue::record([
            (
                "schema",
                POO_FLOW_LOOP_GOVERNOR_RUNTIME_MANIFEST_SCHEMA.into(),
            ),
            ("kind", "loop-governor-runtime-manifest".into()),
            ("bridge", "runtime-manifest".into()),
            ("producer", "poo-flow".into()),
            ("consumer", "marlin-agent-core".into()),
            ("operation", "govern-loop".into()),
            (
                "request_schema",
                POO_FLOW_LOOP_GOVERNOR_MARLIN_REQUEST_SCHEMA.into(),
            ),
            ("request_envelope", request_envelope_value()),
            ("abi_manifest", abi_manifest_value()),
            ("loop_engine_discovery", loop_engine_discovery_value()),
            ("control_owner", "gerbil".into()),
            ("execution_owner", "marlin-agent-core".into()),
        ]),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        POO_FLOW_LOOP_GOVERNOR_RUNTIME_MANIFEST_SCHEMA,
    ));

    let projection = registry
        .decode_dynamic_typed_value(&envelope)
        .expect("Marlin should consume the loop governor runtime manifest shape");

    assert_eq!(
        projection
            .get("operation")
            .and_then(GerbilSchemeValue::as_text),
        Some("govern-loop")
    );
    assert_eq!(
        projection
            .get("request_envelope")
            .and_then(|request| request.get("schema"))
            .and_then(GerbilSchemeValue::as_text),
        Some(POO_FLOW_LOOP_GOVERNOR_MARLIN_REQUEST_SCHEMA)
    );
    assert_eq!(
        projection
            .get("abi_manifest")
            .and_then(|abi| abi.get("schema"))
            .and_then(GerbilSchemeValue::as_text),
        Some(POO_FLOW_LOOP_GOVERNOR_MARLIN_ABI_SCHEMA)
    );
    assert_eq!(
        projection
            .get("loop_engine_discovery")
            .and_then(|discovery| discovery.get("runtime_executed"))
            .and_then(GerbilSchemeValue::as_bool),
        Some(false)
    );
    assert_eq!(
        projection
            .get("control_owner")
            .and_then(GerbilSchemeValue::as_text),
        Some("gerbil")
    );
    assert_eq!(
        projection
            .get("execution_owner")
            .and_then(GerbilSchemeValue::as_text),
        Some("marlin-agent-core")
    );
}

fn loop_governor_bridge_package_manifest() -> GerbilSchemePackageManifest {
    GerbilSchemePackageManifest::new(
        GerbilSchemePackageId::new("marlin.deck-runtime.loop-governor-bridge"),
        loop_governor_bridge_type_manifest(),
    )
    .with_projection_contracts([GerbilSchemeProjectionContract::new(
        runtime_manifest_type_id(),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        POO_FLOW_LOOP_GOVERNOR_RUNTIME_MANIFEST_SCHEMA,
    ))])
}

fn loop_governor_bridge_type_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![
            type_spec_with_schema(
                "marlin.deck-runtime.loop-governor.abi-manifest",
                POO_FLOW_LOOP_GOVERNOR_MARLIN_ABI_SCHEMA,
                [
                    field("schema", "string", true),
                    field("producer", "string", true),
                    field("consumer", "string", true),
                    field("transport", "string", true),
                    field("operation", "string", true),
                    field("request_schema", "string", true),
                    field("control_owner", "string", true),
                    field("execution_owner", "string", true),
                ],
            ),
            type_spec_with_schema(
                "marlin.deck-runtime.loop-engine.discovery",
                POO_FLOW_LOOP_ENGINE_DISCOVERY_SCHEMA,
                [
                    field("schema", "string", true),
                    field("kind", "string", true),
                    field("operation", "string", true),
                    field("runtime_command_contract", "string", true),
                    field("object_families", "array", true).with_element_type_id("string"),
                    field("receipt_contracts", "array", true).with_element_type_id("string"),
                    field("control_owner", "string", true),
                    field("execution_owner", "string", true),
                    field("runtime_executed", "boolean", true),
                ],
            ),
            type_spec_with_schema(
                "marlin.deck-runtime.loop-governor.request-envelope",
                POO_FLOW_LOOP_GOVERNOR_MARLIN_REQUEST_SCHEMA,
                [
                    field("schema", "string", true),
                    field("operation", "string", true),
                    field("target", "string", true),
                    field("transport", "string", true),
                    field("governor", "object", true),
                    field("state_facts", "array", true),
                    field("control_owner", "string", true),
                    field("execution_owner", "string", true),
                ],
            ),
            type_spec_with_schema(
                "marlin.deck-runtime.loop-governor.runtime-manifest",
                POO_FLOW_LOOP_GOVERNOR_RUNTIME_MANIFEST_SCHEMA,
                [
                    field("schema", "string", true),
                    field("kind", "string", true),
                    field("bridge", "string", true),
                    field("producer", "string", true),
                    field("consumer", "string", true),
                    field("operation", "string", true),
                    field("request_schema", "string", true),
                    field(
                        "request_envelope",
                        "marlin.deck-runtime.loop-governor.request-envelope",
                        true,
                    ),
                    field(
                        "abi_manifest",
                        "marlin.deck-runtime.loop-governor.abi-manifest",
                        true,
                    ),
                    field(
                        "loop_engine_discovery",
                        "marlin.deck-runtime.loop-engine.discovery",
                        true,
                    ),
                    field("control_owner", "string", true),
                    field("execution_owner", "string", true),
                ],
            ),
        ],
    }
}

fn request_envelope_value() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        (
            "schema",
            POO_FLOW_LOOP_GOVERNOR_MARLIN_REQUEST_SCHEMA.into(),
        ),
        ("operation", "govern-loop".into()),
        ("target", "runtime-handoff".into()),
        ("transport", "scheme-abi".into()),
        ("governor", GerbilSchemeValue::empty_record()),
        ("state_facts", GerbilSchemeValue::vector([])),
        ("control_owner", "gerbil".into()),
        ("execution_owner", "marlin-agent-core".into()),
    ])
}

fn abi_manifest_value() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema", POO_FLOW_LOOP_GOVERNOR_MARLIN_ABI_SCHEMA.into()),
        ("producer", "poo-flow".into()),
        ("consumer", "marlin-agent-core".into()),
        ("transport", "scheme-abi".into()),
        ("operation", "govern-loop".into()),
        (
            "request_schema",
            POO_FLOW_LOOP_GOVERNOR_MARLIN_REQUEST_SCHEMA.into(),
        ),
        ("control_owner", "gerbil".into()),
        ("execution_owner", "marlin-agent-core".into()),
    ])
}

fn loop_engine_discovery_value() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("schema", POO_FLOW_LOOP_ENGINE_DISCOVERY_SCHEMA.into()),
        ("kind", "loop-engine-marlin-discovery".into()),
        ("operation", "loop-engine-runtime-handoff".into()),
        (
            "runtime_command_contract",
            "poo-flow.loop-engine.runtime-command.v1".into(),
        ),
        (
            "object_families",
            GerbilSchemeValue::vector([
                "loop-engine-profile".into(),
                "loop-engine-policy-extension".into(),
            ]),
        ),
        (
            "receipt_contracts",
            GerbilSchemeValue::vector([
                "poo-flow.loop-engine.profile-receipt.v1".into(),
                "poo-flow.loop-engine.policy-extension-receipt.v1".into(),
            ]),
        ),
        ("control_owner", "gerbil".into()),
        ("execution_owner", "marlin-agent-core".into()),
        ("runtime_executed", false.into()),
    ])
}

fn type_spec_with_schema<const N: usize>(
    type_id: &str,
    schema_id: &str,
    fields: [GerbilSchemeTypeFieldSpec; N],
) -> GerbilSchemeTypeSpec {
    GerbilSchemeTypeSpec {
        type_id: GerbilSchemeTypeId::new(type_id),
        schema_id: Some(GerbilSchemeSchemaId::new(schema_id)),
        fields: fields.into(),
    }
}

fn field(name: &str, type_id: &str, required: bool) -> GerbilSchemeTypeFieldSpec {
    GerbilSchemeTypeFieldSpec {
        name: GerbilSchemeFieldName::new(name),
        type_id: GerbilSchemeTypeId::new(type_id),
        element_type_id: None,
        required,
        description: None,
    }
}

trait FieldSpecExt {
    fn with_element_type_id(self, element_type_id: &str) -> Self;
}

impl FieldSpecExt for GerbilSchemeTypeFieldSpec {
    fn with_element_type_id(mut self, element_type_id: &str) -> Self {
        self.element_type_id = Some(GerbilSchemeTypeId::new(element_type_id));
        self
    }
}

fn runtime_manifest_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new("marlin.deck-runtime.loop-governor.runtime-manifest")
}

fn request_envelope_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new("marlin.deck-runtime.loop-governor.request-envelope")
}

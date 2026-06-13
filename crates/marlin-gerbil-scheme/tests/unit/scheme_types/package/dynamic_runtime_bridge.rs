use marlin_gerbil_scheme::{
    GerbilSchemeFieldName, GerbilSchemeSchemaId, GerbilSchemeTypeFieldSpec, GerbilSchemeTypeId,
    GerbilSchemeTypeManifest, GerbilSchemeTypeRegistry, GerbilSchemeTypeSpec,
    GerbilSchemeTypedValue, GerbilSchemeValue,
};

#[test]
fn downstream_scheme_runtime_bridge_decodes_graph_hook_and_session_without_rust_policy_types() {
    let registry = GerbilSchemeTypeRegistry::new(runtime_bridge_manifest())
        .expect("runtime bridge manifest should build dynamic registry");
    let envelope = GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new("downstream.runtime.bridge"),
        GerbilSchemeValue::record([
            (
                "graph_policy",
                GerbilSchemeValue::record([
                    ("strategy", "beam-search-v3".into()),
                    ("max_candidates", 12.into()),
                ]),
            ),
            (
                "hook_action",
                GerbilSchemeValue::record([
                    ("kind", "Rewrite".into()),
                    ("target", "command".into()),
                    ("command", "cargo test".into()),
                    ("replacement", "cargo test --locked".into()),
                    ("mode", "observe".into()),
                ]),
            ),
            (
                "session",
                GerbilSchemeValue::record([
                    ("session_id", "session:downstream:42".into()),
                    ("parent_session_id", "session:root".into()),
                    ("visibility", "isolated".into()),
                ]),
            ),
        ]),
    )
    .with_schema_id(GerbilSchemeSchemaId::new("downstream.runtime.bridge.v1"));

    let projection = registry
        .decode_dynamic_typed_value(&envelope)
        .expect("dynamic runtime bridge should decode through generic Scheme value bridge");

    assert_eq!(
        projection
            .get("graph_policy")
            .and_then(|graph| graph.get("strategy"))
            .and_then(GerbilSchemeValue::as_text),
        Some("beam-search-v3")
    );
    assert_eq!(
        projection
            .get("hook_action")
            .and_then(|hook| hook.get("mode"))
            .and_then(GerbilSchemeValue::as_text),
        Some("observe")
    );
    assert_eq!(
        projection
            .get("hook_action")
            .and_then(|hook| hook.get("replacement"))
            .and_then(GerbilSchemeValue::as_text),
        Some("cargo test --locked")
    );
    assert_eq!(
        projection
            .get("session")
            .and_then(|session| session.get("visibility"))
            .and_then(GerbilSchemeValue::as_text),
        Some("isolated")
    );
    assert_eq!(
        projection
            .get("session")
            .and_then(|session| session.get("parent_session_id"))
            .and_then(GerbilSchemeValue::as_text),
        Some("session:root")
    );
}

fn runtime_bridge_manifest() -> GerbilSchemeTypeManifest {
    GerbilSchemeTypeManifest {
        schema_id: GerbilSchemeSchemaId::new("marlin.scheme-types.manifest.v1"),
        types: vec![
            type_spec(
                "downstream.runtime.graph-policy",
                [
                    field("strategy", "string", true),
                    field("max_candidates", "integer", true),
                ],
            ),
            type_spec(
                "downstream.runtime.hook-action",
                [
                    field("kind", "string", true),
                    field("target", "string", true),
                    field("command", "string", true),
                    field("replacement", "string", true),
                    field("mode", "string", true),
                ],
            ),
            type_spec(
                "downstream.runtime.session",
                [
                    field("session_id", "string", true),
                    field("parent_session_id", "string", true),
                    field("visibility", "string", true),
                ],
            ),
            GerbilSchemeTypeSpec {
                type_id: GerbilSchemeTypeId::new("downstream.runtime.bridge"),
                schema_id: Some(GerbilSchemeSchemaId::new("downstream.runtime.bridge.v1")),
                fields: vec![
                    field("graph_policy", "downstream.runtime.graph-policy", true),
                    field("hook_action", "downstream.runtime.hook-action", true),
                    field("session", "downstream.runtime.session", true),
                ],
            },
        ],
    }
}

fn type_spec<const N: usize>(
    type_id: &str,
    fields: [GerbilSchemeTypeFieldSpec; N],
) -> GerbilSchemeTypeSpec {
    GerbilSchemeTypeSpec {
        type_id: GerbilSchemeTypeId::new(type_id),
        schema_id: None,
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

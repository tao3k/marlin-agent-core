use marlin_gerbil_scheme::{
    GerbilSchemeSchemaId, GerbilSchemeTypeId, GerbilSchemeTypeManifest,
    decode_gerbil_scheme_type_manifest,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub(super) struct StrategySelectionProjection {
    pub schema_id: String,
    pub matched: bool,
    pub action: String,
}

pub(super) fn strategy_selection_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new("marlin.deck-runtime.strategy-selection")
}

pub(super) fn strategy_selection_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new("marlin.deck-runtime.strategy-selection.v1")
}

pub(super) fn strategy_selection_manifest() -> GerbilSchemeTypeManifest {
    decode_gerbil_scheme_type_manifest(
        r#"{
            "schema_id": "marlin.scheme-types.manifest.v1",
            "types": [
                {
                    "type_id": "marlin.deck-runtime.strategy-selection",
                    "schema_id": "marlin.deck-runtime.strategy-selection.v1",
                    "fields": [
                        {"name": "schema_id", "type_id": "string", "required": true},
                        {"name": "matched", "type_id": "boolean", "required": true},
                        {"name": "action", "type_id": "string", "required": true}
                    ]
                }
            ]
        }"#,
    )
    .expect("decode scheme type manifest")
}

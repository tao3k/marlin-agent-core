use marlin_gerbil_scheme::{
    GerbilSchemeProjectionContract, GerbilSchemeSchemaId, GerbilSchemeTypeId,
    GerbilSchemeTypeManifest, GerbilSchemeTypedProjection,
    scheme_type_fixtures::decode_gerbil_scheme_type_manifest_fixture,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub(super) struct StrategySelectionProjection {
    pub schema_id: String,
    pub matched: bool,
    pub action: String,
}

impl GerbilSchemeTypedProjection for StrategySelectionProjection {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        GerbilSchemeProjectionContract::new(strategy_selection_type_id())
            .with_schema_id(strategy_selection_schema_id())
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub(super) struct StrategyDecisionProjection {
    pub schema_id: String,
    pub selection: StrategySelectionProjection,
    pub reason: String,
}

impl GerbilSchemeTypedProjection for StrategyDecisionProjection {
    fn scheme_projection_contract() -> GerbilSchemeProjectionContract {
        GerbilSchemeProjectionContract::new(strategy_decision_type_id())
            .with_schema_id(strategy_decision_schema_id())
    }
}

pub(super) fn strategy_selection_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new("marlin.deck-runtime.strategy-selection")
}

pub(super) fn strategy_selection_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new("marlin.deck-runtime.strategy-selection.v1")
}

pub(super) fn strategy_decision_type_id() -> GerbilSchemeTypeId {
    GerbilSchemeTypeId::new("marlin.deck-runtime.strategy-decision")
}

pub(super) fn strategy_decision_schema_id() -> GerbilSchemeSchemaId {
    GerbilSchemeSchemaId::new("marlin.deck-runtime.strategy-decision.v1")
}

pub(super) fn strategy_selection_manifest() -> GerbilSchemeTypeManifest {
    decode_gerbil_scheme_type_manifest_fixture(
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

pub(super) fn nested_strategy_manifest() -> GerbilSchemeTypeManifest {
    decode_gerbil_scheme_type_manifest_fixture(
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
                },
                {
                    "type_id": "marlin.deck-runtime.strategy-decision",
                    "schema_id": "marlin.deck-runtime.strategy-decision.v1",
                    "fields": [
                        {"name": "schema_id", "type_id": "string", "required": true},
                        {
                            "name": "selection",
                            "type_id": "marlin.deck-runtime.strategy-selection",
                            "required": true
                        },
                        {"name": "reason", "type_id": "string", "required": true}
                    ]
                }
            ]
        }"#,
    )
    .expect("decode nested scheme type manifest")
}

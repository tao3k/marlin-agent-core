use super::{
    deck_runtime_native_readiness_plan, downstream_strategy_package,
    duplicate_native_symbol_contract,
};
use crate::scheme_types::support::{strategy_selection_schema_id, strategy_selection_type_id};
use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_ABI_ID, GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
    GerbilSchemeNativeAbiContract, GerbilSchemeNativeAbiId, GerbilSchemeNativeAbiReadinessPlan,
    GerbilSchemeNativeSymbol, GerbilSchemeProjectionContract, GerbilSchemeTypeDecodeError,
    GerbilSchemeTypeId, validate_gerbil_scheme_package_manifest,
    validate_gerbil_scheme_package_native_readiness,
};

#[test]
fn scheme_package_manifest_rejects_unknown_projection_type() {
    let manifest = downstream_strategy_package().with_projection_contracts([
        GerbilSchemeProjectionContract::new(GerbilSchemeTypeId::new(
            "marlin.missing-projection-type",
        )),
    ]);

    let error = validate_gerbil_scheme_package_manifest(&manifest)
        .expect_err("unknown projection contract type should fail package validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::UnknownType {
            type_id: GerbilSchemeTypeId::new("marlin.missing-projection-type")
        }
    );
}

#[test]
fn scheme_package_manifest_rejects_duplicate_projection_contracts() {
    let contract = GerbilSchemeProjectionContract::new(strategy_selection_type_id())
        .with_schema_id(strategy_selection_schema_id());
    let manifest =
        downstream_strategy_package().with_projection_contracts([contract.clone(), contract]);

    let error = validate_gerbil_scheme_package_manifest(&manifest)
        .expect_err("duplicate projection contracts should fail package validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::DuplicateProjectionContract {
            type_id: strategy_selection_type_id(),
            schema_id: Some(strategy_selection_schema_id()),
        }
    );
}

#[test]
fn scheme_package_manifest_rejects_empty_native_abi_exports() {
    let manifest =
        downstream_strategy_package().with_native_abi(GerbilSchemeNativeAbiContract::new(
            GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID),
            GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
        ));

    let error = validate_gerbil_scheme_package_manifest(&manifest)
        .expect_err("native ABI without exported symbols should fail package validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::MissingNativeSymbols {
            abi_id: GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID),
        }
    );
}

#[test]
fn scheme_package_manifest_rejects_duplicate_native_abi_exports() {
    let manifest =
        downstream_strategy_package().with_native_abi(duplicate_native_symbol_contract());

    let error = validate_gerbil_scheme_package_manifest(&manifest)
        .expect_err("duplicate native ABI symbols should fail package validation");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::DuplicateNativeSymbol {
            symbol: GerbilSchemeNativeSymbol::new("marlin_deck_runtime_select_model_route"),
        }
    );
}

#[test]
fn scheme_package_native_readiness_rejects_wrong_exported_symbol() {
    let missing_symbol = GerbilSchemeNativeSymbol::new("marlin_downstream_missing_export");
    let manifest = downstream_strategy_package().with_native_abi(
        GerbilSchemeNativeAbiContract::new(
            GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID),
            GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
        )
        .with_exported_symbols([missing_symbol.clone()]),
    );

    let error = validate_gerbil_scheme_package_native_readiness(
        &manifest,
        &deck_runtime_native_readiness_plan(),
    )
    .expect_err("readiness should fail when manifest requires a missing native symbol");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::MissingNativeSymbol {
            symbol: missing_symbol
        }
    );
}

#[test]
fn scheme_package_native_readiness_rejects_abi_version_mismatch() {
    let manifest = downstream_strategy_package().with_native_abi(
        GerbilSchemeNativeAbiContract::new(
            GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID),
            GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
        )
        .with_exported_symbols([GerbilSchemeNativeSymbol::new(
            "marlin_deck_runtime_select_model_route",
        )]),
    );
    let readiness_plan = GerbilSchemeNativeAbiReadinessPlan::new(
        GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID),
        GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION + 1,
    )
    .with_exported_symbols([GerbilSchemeNativeSymbol::new(
        "marlin_deck_runtime_select_model_route",
    )]);

    let error = validate_gerbil_scheme_package_native_readiness(&manifest, &readiness_plan)
        .expect_err("readiness should fail when native ABI versions differ");

    assert_eq!(
        error,
        GerbilSchemeTypeDecodeError::NativeAbiVersionMismatch {
            abi_id: GerbilSchemeNativeAbiId::new(GERBIL_DECK_RUNTIME_NATIVE_ABI_ID),
            expected: GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION,
            actual: GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION + 1,
        }
    );
}

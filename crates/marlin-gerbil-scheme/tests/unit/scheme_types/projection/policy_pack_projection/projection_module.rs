use super::{
    GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL,
    GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID, GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID,
    GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID, GerbilPooLoopProgramCompilerBoundary,
    GerbilPooLoopProgramCompilerOwner, GerbilPooLoopProgramCompilerSerializationBoundary,
    GerbilSchemeNativeSymbol, GerbilSchemeSchemaId, GerbilSchemeTypeId, GerbilSchemeValue,
    decode_gerbil_loop_policy_projection_module,
    gerbil_deck_runtime_loop_policy_projection_module_request,
    gerbil_deck_runtime_native_projection_readiness_plan, loop_policy_projection_module_envelope,
    loop_policy_projection_module_registry, poo_loop_program_compiler_fixture,
};

#[test]
fn loop_policy_projection_module_declares_deck_native_projection_gate() {
    let request = gerbil_deck_runtime_loop_policy_projection_module_request();
    assert_eq!(
        request.symbol,
        GerbilSchemeNativeSymbol::new(
            GERBIL_DECK_RUNTIME_PROJECT_LOOP_POLICY_PROJECTION_MODULE_SYMBOL
        )
    );
    assert_eq!(
        request.contract.type_id(),
        &GerbilSchemeTypeId::new(GERBIL_LOOP_POLICY_PROJECTION_MODULE_TYPE_ID)
    );
    assert_eq!(
        request.contract.schema_id(),
        Some(&GerbilSchemeSchemaId::new(
            GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID
        ))
    );

    let readiness_plan = gerbil_deck_runtime_native_projection_readiness_plan();
    assert!(readiness_plan.exported_symbols.contains(&request.symbol));
}

#[test]
fn loop_policy_projection_module_decodes_vertical_mainline_descriptor() {
    let registry = loop_policy_projection_module_registry();
    let projection = decode_gerbil_loop_policy_projection_module(
        &registry,
        &loop_policy_projection_module_envelope(profile_projection_module_fixture(
            "runtime-reactive-tool-loop",
            GerbilSchemeValue::from("runtime-reactive-tool-loop"),
            GerbilSchemeValue::vector(["+scripted-e2e".into(), "+verification".into()]),
            true,
        )),
    )
    .expect("profile projection module");

    assert_eq!(
        projection.kind.as_str(),
        GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID
    );
    assert_eq!(
        projection.module_id,
        "poo-flow.loop-engine.runtime-reactive-tool-loop"
    );
    assert_eq!(projection.profile_id.as_str(), "runtime-reactive-tool-loop");
    assert_eq!(
        projection.owner,
        GerbilPooLoopProgramCompilerOwner::GerbilPooFlow
    );
    assert_eq!(
        projection.vertical_case_id.as_deref(),
        Some("runtime-reactive-tool-loop")
    );
    assert_eq!(
        projection.vertical_capability_tags,
        vec!["+scripted-e2e".to_owned(), "+verification".to_owned()]
    );
    assert!(projection.vertical_mainline);
    assert_eq!(
        projection.rust_type.as_str(),
        GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID
    );
    assert_eq!(
        projection.scheme_boundary,
        GerbilPooLoopProgramCompilerBoundary::SchemeTypesToRustTypes
    );
    assert_eq!(
        projection.serialization_boundary,
        GerbilPooLoopProgramCompilerSerializationBoundary::RustOwnedCliTraceCrossProcess
    );
    assert_eq!(
        projection.profile_id.as_str(),
        projection.compiler_receipt.profile_id.as_str()
    );
}

#[test]
fn loop_policy_projection_module_decodes_scheme_false_for_non_mainline_descriptor() {
    let registry = loop_policy_projection_module_registry();
    let projection = decode_gerbil_loop_policy_projection_module(
        &registry,
        &loop_policy_projection_module_envelope(profile_projection_module_fixture(
            "supporting-policy-pack",
            GerbilSchemeValue::from(false),
            GerbilSchemeValue::vector([]),
            false,
        )),
    )
    .expect("non-mainline projection module");

    assert_eq!(projection.profile_id.as_str(), "supporting-policy-pack");
    assert_eq!(projection.vertical_case_id, None);
    assert!(projection.vertical_capability_tags.is_empty());
    assert!(!projection.vertical_mainline);
}

fn profile_projection_module_fixture(
    profile_id: &str,
    vertical_case_id: GerbilSchemeValue,
    vertical_capability_tags: GerbilSchemeValue,
    vertical_mainline: bool,
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        (
            "kind",
            GERBIL_LOOP_POLICY_PROJECTION_MODULE_SCHEMA_ID.into(),
        ),
        (
            "module-id",
            format!("poo-flow.loop-engine.{profile_id}").into(),
        ),
        ("profile-id", profile_id.into()),
        ("owner", "gerbil-poo-flow".into()),
        (
            "source-module",
            ":config-interface/modules/policy-pack".into(),
        ),
        ("poo-flow-module", "loop-engine".into()),
        (
            "poo-flow-capability-lanes",
            GerbilSchemeValue::vector(["fun-flow".into(), "loop-engine".into()]),
        ),
        ("vertical-case-id", vertical_case_id),
        ("vertical-capability-tags", vertical_capability_tags),
        ("vertical-mainline?", vertical_mainline.into()),
        (
            "rust-type",
            GERBIL_POO_LOOP_PROGRAM_COMPILER_SCHEMA_ID.into(),
        ),
        ("scheme-boundary", "scheme-types-to-rust-types".into()),
        (
            "serialization-boundary",
            "rust-owned-cli-trace-cross-process".into(),
        ),
        (
            "compiler-receipt",
            poo_loop_program_compiler_fixture_with_profile(profile_id),
        ),
    ])
}

fn poo_loop_program_compiler_fixture_with_profile(profile_id: &str) -> GerbilSchemeValue {
    let receipt = poo_loop_program_compiler_fixture([7; 32]);
    let mut fields = receipt
        .as_record()
        .expect("compiler receipt record")
        .clone();
    fields.insert("profile-id".to_owned(), profile_id.into());
    GerbilSchemeValue::Record(fields)
}

use super::support::write_empty_file;
use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION,
    GerbilDeckRuntimeNativeAotConfig, GerbilDeckRuntimeNativeAotProfile,
    GerbilDeckRuntimeNativeAotStatus, GerbilSchemeNativeAbiId,
};
use tempfile::Builder;

#[test]
fn deck_runtime_native_aot_plan_records_link_unit_compile() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-plan-")
        .tempdir()
        .expect("create root");
    let compiled_runtime_scm = root.path().join("compiled/deck-runtime-native~0.scm");
    let gsc = root.path().join("toolchain/gsc");
    let header = root.path().join("include/marlin_deck_runtime_native.h");
    write_empty_file(&compiled_runtime_scm);
    write_empty_file(&gsc);
    write_empty_file(&header);

    let plan = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_compiled_runtime_scm(&compiled_runtime_scm)
        .with_gsc(&gsc)
        .with_header(&header)
        .with_c_compiler("clang")
        .plan();

    assert_eq!(plan.profile, GerbilDeckRuntimeNativeAotProfile::DeckRuntime);
    assert_eq!(
        plan.status,
        GerbilDeckRuntimeNativeAotStatus::ReadyToBuildLinkUnit
    );
    assert_eq!(plan.compiled_runtime_scm, compiled_runtime_scm);
    assert_eq!(plan.header, header);
    assert_eq!(
        plan.object,
        root.path().join("compiled/deck-runtime-native~0.o")
    );
    assert_eq!(
        plan.link_c_source,
        root.path().join("compiled/deck-runtime-native~0_.c")
    );
    assert_eq!(
        plan.link_object,
        root.path().join("compiled/deck-runtime-native~0_.o")
    );
    assert_eq!(
        plan.exported_symbols
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        [
            "marlin_deck_runtime_initialize",
            "marlin_deck_runtime_select_model_route"
        ]
    );
    assert_eq!(plan.gsc_compile_object.program, gsc);
    assert_eq!(
        plan.gsc_compile_object.args[..4],
        ["-target", "C", "-cc", "clang"]
    );
    assert!(plan.gsc_compile_object.args.contains(&"-obj".to_string()));
    assert_eq!(plan.gsc_generate_link_source.program, gsc);
    assert!(
        plan.gsc_generate_link_source
            .args
            .contains(&"-link".to_string())
    );
    assert!(
        !plan
            .gsc_generate_link_source
            .args
            .contains(&"-flat".to_string())
    );
    assert_eq!(plan.gsc_compile_link_object.program, gsc);
    assert!(
        plan.gsc_compile_link_object
            .args
            .contains(&"-obj".to_string())
    );
    assert!(
        plan.gsc_compile_link_object
            .args
            .contains(&"-cc-options".to_string())
    );
    assert!(
        plan.gsc_compile_link_object
            .args
            .contains(&"-D___LIBRARY".to_string())
    );
    assert!(
        plan.gsc_compile_link_object
            .args
            .iter()
            .any(|arg| arg.ends_with("deck-runtime-native~0_.c"))
    );
    assert_eq!(plan.audit_symbols.program, std::path::PathBuf::from("nm"));
    assert_eq!(
        plan.audit_symbols.args,
        [
            plan.object.to_string_lossy().into_owned(),
            plan.link_object.to_string_lossy().into_owned()
        ]
    );
    assert_eq!(plan.detail, None);
}

#[test]
fn agent_policy_routing_native_aot_plan_records_profile_symbols() {
    let root = Builder::new()
        .prefix("marlin-gerbil-agent-policy-routing-native-aot-plan-")
        .tempdir()
        .expect("create root");
    let compiled_runtime_scm = root
        .path()
        .join("compiled/agent-policy-routing-native~0.scm");
    let gsc = root.path().join("toolchain/gsc");
    let header = root
        .path()
        .join("include/marlin_agent_policy_routing_native.h");
    write_empty_file(&compiled_runtime_scm);
    write_empty_file(&gsc);
    write_empty_file(&header);

    let plan = GerbilDeckRuntimeNativeAotConfig::agent_policy_routing(root.path())
        .with_compiled_runtime_scm(&compiled_runtime_scm)
        .with_gsc(&gsc)
        .with_header(&header)
        .with_c_compiler("clang")
        .plan();

    assert_eq!(
        plan.profile,
        GerbilDeckRuntimeNativeAotProfile::AgentPolicyRouting
    );
    assert_eq!(
        plan.status,
        GerbilDeckRuntimeNativeAotStatus::ReadyToBuildLinkUnit
    );
    assert_eq!(
        plan.object,
        root.path().join("compiled/agent-policy-routing-native~0.o")
    );
    assert_eq!(
        plan.link_c_source,
        root.path()
            .join("compiled/agent-policy-routing-native~0_.c")
    );
    assert_eq!(
        plan.link_object,
        root.path()
            .join("compiled/agent-policy-routing-native~0_.o")
    );
    assert_eq!(
        plan.exported_symbols
            .iter()
            .map(|symbol| symbol.as_str())
            .collect::<Vec<_>>(),
        [
            "marlin_agent_policy_routing_initialize",
            "marlin_agent_policy_routing_select_edges"
        ]
    );

    let readiness_plan = plan.scheme_native_abi_readiness_plan();
    assert_eq!(
        readiness_plan.abi_id,
        GerbilSchemeNativeAbiId::new(GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_ID)
    );
    assert_eq!(
        readiness_plan.version,
        GERBIL_AGENT_POLICY_ROUTING_NATIVE_ABI_VERSION
    );
}

#[test]
fn deck_runtime_native_aot_plan_reports_missing_compiled_runtime() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-missing-compiled-runtime-")
        .tempdir()
        .expect("create root");
    let gsc = root.path().join("toolchain/gsc");
    let header = root.path().join("include/marlin_deck_runtime_native.h");
    write_empty_file(&gsc);
    write_empty_file(&header);

    let plan = GerbilDeckRuntimeNativeAotConfig::new(root.path())
        .with_gsc(gsc)
        .with_header(header)
        .plan();

    assert_eq!(
        plan.status,
        GerbilDeckRuntimeNativeAotStatus::MissingCompiledRuntime
    );
    assert!(plan.detail.as_deref().is_some_and(|detail| {
        detail.contains("missing compiled native Deck runtime Scheme artifact")
    }));
}

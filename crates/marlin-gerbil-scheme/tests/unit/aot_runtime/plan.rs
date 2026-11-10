use super::support::{
    write_agent_policy_routing_abi_aot_dependency_scms, write_deck_route_abi_aot_dependency_scms,
    write_empty_file,
};
use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_ABI_ID, GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION,
    GerbilDeckRuntimeAotConfig, GerbilDeckRuntimeAotProfile, GerbilDeckRuntimeAotStatus,
    GerbilSchemeCAbiId,
};
use tempfile::Builder;

fn assert_single_binding_dependency(
    plan: &marlin_gerbil_scheme::GerbilDeckRuntimeAotPlan,
    expected_dependency_scm: std::path::PathBuf,
    expected_runtime_object: std::path::PathBuf,
) {
    let expected_dependency_object = expected_dependency_scm.with_extension("o");

    assert_eq!(
        plan.compiled_runtime_dependency_scms.as_slice(),
        std::slice::from_ref(&expected_dependency_scm)
    );
    assert_eq!(
        plan.dependency_objects.as_slice(),
        std::slice::from_ref(&expected_dependency_object)
    );
    assert_eq!(
        plan.module_objects,
        vec![expected_dependency_object, expected_runtime_object]
    );
    assert_eq!(plan.gsc_compile_dependency_objects.len(), 1);
    assert_eq!(
        plan.gsc_compile_dependency_objects[0].args.last(),
        Some(&expected_dependency_scm.to_string_lossy().into_owned())
    );

    let link_source_args = &plan.gsc_generate_link_source.args;
    let dependency_arg = expected_dependency_scm.to_string_lossy().into_owned();
    let runtime_arg = plan.compiled_runtime_scm.to_string_lossy().into_owned();
    let dependency_position = link_source_args
        .iter()
        .position(|arg| arg == &dependency_arg)
        .expect("dependency scm passed to gsc -link");
    let runtime_position = link_source_args
        .iter()
        .position(|arg| arg == &runtime_arg)
        .expect("runtime scm passed to gsc -link");
    assert!(
        dependency_position < runtime_position,
        "binding dependency scm must precede wrapper scm in gsc -link"
    );
}

#[test]
fn deck_route_abi_aot_plan_records_link_unit_compile() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-plan-")
        .tempdir()
        .expect("create root");
    let compiled_runtime_scm = root.path().join("compiled/deck-route-abi~0.scm");
    let gsc = root.path().join("toolchain/gsc");
    let header = root.path().join("include/marlin_deck_route_abi.h");
    write_empty_file(&compiled_runtime_scm);
    write_deck_route_abi_aot_dependency_scms(root.path());
    write_empty_file(&gsc);
    write_empty_file(&header);

    let plan = GerbilDeckRuntimeAotConfig::new(root.path())
        .with_compiled_runtime_scm(&compiled_runtime_scm)
        .with_gsc(&gsc)
        .with_header(&header)
        .with_c_compiler("clang")
        .plan();

    assert_eq!(plan.profile, GerbilDeckRuntimeAotProfile::DeckRuntime);
    assert_eq!(
        plan.status,
        GerbilDeckRuntimeAotStatus::ReadyToBuildLinkUnit
    );
    assert_eq!(plan.compiled_runtime_scm, compiled_runtime_scm);
    assert_eq!(plan.header, header);
    assert_eq!(plan.object, root.path().join("compiled/deck-route-abi~0.o"));
    assert_single_binding_dependency(
        &plan,
        root.path().join(".gerbil/native/_deck-route-abi~0.scm"),
        root.path().join("compiled/deck-route-abi~0.o"),
    );
    assert_eq!(
        plan.link_c_source,
        root.path().join("compiled/deck-route-abi~0_.c")
    );
    assert_eq!(
        plan.link_object,
        root.path().join("compiled/deck-route-abi~0_.o")
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
            .any(|arg| arg.ends_with("deck-route-abi~0_.c"))
    );
    assert_eq!(plan.audit_symbols.program, std::path::PathBuf::from("nm"));
    assert_eq!(
        plan.audit_symbols.args,
        [
            plan.dependency_objects[0].to_string_lossy().into_owned(),
            plan.object.to_string_lossy().into_owned(),
            plan.link_object.to_string_lossy().into_owned()
        ]
    );
    assert_eq!(plan.detail, None);
}

#[test]
fn agent_policy_routing_abi_aot_plan_records_profile_symbols() {
    let root = Builder::new()
        .prefix("marlin-gerbil-agent-policy-routing-abi-aot-plan-")
        .tempdir()
        .expect("create root");
    let compiled_runtime_scm = root.path().join("compiled/agent-policy-routing-abi~0.scm");
    let gsc = root.path().join("toolchain/gsc");
    let header = root
        .path()
        .join("include/marlin_agent_policy_routing_abi.h");
    write_empty_file(&compiled_runtime_scm);
    write_agent_policy_routing_abi_aot_dependency_scms(root.path());
    write_empty_file(&gsc);
    write_empty_file(&header);

    let plan = GerbilDeckRuntimeAotConfig::agent_policy_routing(root.path())
        .with_compiled_runtime_scm(&compiled_runtime_scm)
        .with_gsc(&gsc)
        .with_header(&header)
        .with_c_compiler("clang")
        .plan();

    assert_eq!(
        plan.profile,
        GerbilDeckRuntimeAotProfile::AgentPolicyRouting
    );
    assert_eq!(
        plan.status,
        GerbilDeckRuntimeAotStatus::ReadyToBuildLinkUnit
    );
    assert_eq!(
        plan.object,
        root.path().join("compiled/agent-policy-routing-abi~0.o")
    );
    assert_single_binding_dependency(
        &plan,
        root.path()
            .join(".gerbil/native/_agent-policy-routing-abi~0.scm"),
        root.path().join("compiled/agent-policy-routing-abi~0.o"),
    );
    assert_eq!(
        plan.link_c_source,
        root.path().join("compiled/agent-policy-routing-abi~0_.c")
    );
    assert_eq!(
        plan.link_object,
        root.path().join("compiled/agent-policy-routing-abi~0_.o")
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

    let readiness_plan = plan.scheme_c_abi_readiness_plan();
    assert_eq!(
        readiness_plan.abi_id,
        GerbilSchemeCAbiId::new(GERBIL_AGENT_POLICY_ROUTING_ABI_ID)
    );
    assert_eq!(
        readiness_plan.version,
        GERBIL_AGENT_POLICY_ROUTING_ABI_VERSION
    );
}

#[test]
fn deck_route_abi_aot_plan_reports_missing_compiled_runtime() {
    let root = Builder::new()
        .prefix("marlin-gerbil-native-aot-missing-compiled-runtime-")
        .tempdir()
        .expect("create root");
    let gsc = root.path().join("toolchain/gsc");
    let header = root.path().join("include/marlin_deck_route_abi.h");
    write_empty_file(&gsc);
    write_empty_file(&header);

    let plan = GerbilDeckRuntimeAotConfig::new(root.path())
        .with_gsc(gsc)
        .with_header(header)
        .plan();

    assert_eq!(
        plan.status,
        GerbilDeckRuntimeAotStatus::MissingCompiledRuntime
    );
    assert!(plan.detail.as_deref().is_some_and(|detail| {
        detail.contains("missing compiled native Deck runtime Scheme artifact")
    }));
}

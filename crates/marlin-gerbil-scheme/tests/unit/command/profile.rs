use marlin_gerbil_scheme::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, GerbilCommandProfile, GerbilCommandSpec,
};

#[test]
fn command_profile_round_trips_json_configuration() {
    let profile = GerbilCommandProfile::new("/opt/gerbil/bin/gxi")
        .arg(GERBIL_ADAPTER_MODULE)
        .current_dir(".data/gerbil")
        .env("GERBIL_LOADPATH", "gerbil/src");

    let encoded = serde_json::to_string(&profile).expect("profile should encode as json");
    let decoded = GerbilCommandProfile::from_json(&encoded).expect("profile should decode");

    assert_eq!(decoded, profile);
}

#[test]
fn command_profile_builds_exec_spec_without_shell_parsing() {
    let profile = GerbilCommandProfile::new("/opt/gerbil/bin/gxi")
        .arg(GERBIL_ADAPTER_MODULE)
        .arg("(import :marlin/compiler)")
        .env("GERBIL_LOADPATH", "gerbil/src");
    let spec: GerbilCommandSpec = profile.into();

    assert_eq!(spec.program.to_string_lossy(), "/opt/gerbil/bin/gxi");
    assert_eq!(spec.args.len(), 2);
    assert_eq!(spec.args[0].to_string_lossy(), GERBIL_ADAPTER_MODULE);
    assert_eq!(spec.args[1].to_string_lossy(), "(import :marlin/compiler)");
    assert_eq!(
        spec.env
            .iter()
            .find(|(key, _value)| key.to_string_lossy() == "GERBIL_LOADPATH")
            .map(|(_key, value)| value.to_string_lossy()),
        Some("gerbil/src".into())
    );
}

#[test]
fn command_profile_builds_marlin_runtime_module_entry() {
    let profile = GerbilCommandProfile::marlin_runtime_module("/opt/gerbil/bin/gxi", "gerbil");

    assert_eq!(profile.program, "/opt/gerbil/bin/gxi");
    assert_eq!(profile.args, [GERBIL_ADAPTER_MODULE.to_owned()]);
    assert_eq!(
        profile.env.get(GERBIL_LOADPATH_ENV).map(String::as_str),
        Some("gerbil/src")
    );
}

#[test]
fn command_spec_builds_marlin_runtime_module_entry() {
    let spec = GerbilCommandSpec::marlin_runtime_module("/opt/gerbil/bin/gxi", "gerbil");

    assert_eq!(spec.program.to_string_lossy(), "/opt/gerbil/bin/gxi");
    assert_eq!(spec.args.len(), 1);
    assert_eq!(spec.args[0].to_string_lossy(), GERBIL_ADAPTER_MODULE);
    assert_eq!(
        spec.env
            .iter()
            .find(|(key, _value)| key.to_string_lossy() == GERBIL_LOADPATH_ENV)
            .map(|(_key, value)| value.to_string_lossy()),
        Some("gerbil/src".into())
    );
}

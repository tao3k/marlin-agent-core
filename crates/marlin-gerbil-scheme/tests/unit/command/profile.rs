use super::support::loop_graph_artifact;
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCommandProfile, GerbilCommandSpec,
    GerbilCompiler, GerbilSource,
};

#[test]
fn command_profile_round_trips_json_configuration() {
    let profile = GerbilCommandProfile::new("/opt/gerbil/bin/gxi")
        .arg("--stdio-json")
        .current_dir(".data/gerbil")
        .env("GERBIL_LOADPATH", "fixtures/gerbil");

    let encoded = serde_json::to_string(&profile).expect("profile should encode as json");
    let decoded = GerbilCommandProfile::from_json(&encoded).expect("profile should decode");

    assert_eq!(decoded, profile);
}

#[test]
fn command_profile_builds_exec_spec_without_shell_parsing() {
    let profile = GerbilCommandProfile::new("/opt/gerbil/bin/gxi")
        .arg("--stdio-json")
        .arg("(import :marlin/compiler)")
        .env("GERBIL_LOADPATH", "fixtures/gerbil");
    let spec: GerbilCommandSpec = profile.into();

    assert_eq!(spec.program.to_string_lossy(), "/opt/gerbil/bin/gxi");
    assert_eq!(spec.args.len(), 2);
    assert_eq!(spec.args[0].to_string_lossy(), "--stdio-json");
    assert_eq!(spec.args[1].to_string_lossy(), "(import :marlin/compiler)");
    assert_eq!(
        spec.env
            .iter()
            .find(|(key, _value)| key.to_string_lossy() == "GERBIL_LOADPATH")
            .map(|(_key, value)| value.to_string_lossy()),
        Some("fixtures/gerbil".into())
    );
}

#[test]
fn command_compiler_can_be_built_from_profile() {
    let profile = GerbilCommandProfile::new("/bin/sh").arg("-c").arg(
        "printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"from-profile\",\"nodes\":[],\"edges\":[]}}}'",
    );
    let compiler = GerbilCommandCompiler::from_profile(profile);

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .expect("profile-backed command should decode response");

    assert_eq!(artifact, loop_graph_artifact("from-profile"));
}

#[test]
fn command_compiler_can_be_built_from_profile_json() {
    let profile = GerbilCommandProfile::new("/bin/sh").arg("-c").arg(
        "printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"from-profile-json\",\"nodes\":[],\"edges\":[]}}}'",
    );
    let profile_json = serde_json::to_string(&profile).expect("profile should encode as json");
    let compiler =
        GerbilCommandCompiler::from_profile_json(&profile_json).expect("profile json should parse");

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .expect("json profile-backed command should decode response");

    assert_eq!(artifact, loop_graph_artifact("from-profile-json"));
}

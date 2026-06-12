use super::support::loop_graph_artifact;
use marlin_gerbil_scheme::{
    GERBIL_ADAPTER_MODULE, GERBIL_COMMAND_ADAPTER_PATH, GERBIL_LOADPATH_ENV,
    GERBIL_MARLIN_ADAPTER_PATH, GERBIL_MARLIN_PROTOCOL_PATH, GerbilArtifactKind,
    GerbilCommandCompiler, GerbilCommandProfile, GerbilCommandSpec, GerbilCompiler,
    GerbilRuntimeBinding, GerbilSource, default_gerbil_gxi_program, gerbil_runtime_loadpath,
};
use tempfile::{Builder, TempDir};

#[test]
fn command_profile_round_trips_json_configuration() {
    let profile = GerbilCommandProfile::new("/opt/gerbil/bin/gxi")
        .arg("--stdio-json")
        .current_dir(".data/gerbil")
        .env("GERBIL_LOADPATH", "gerbil/src");

    let encoded = serde_json::to_string(&profile).expect("profile should encode as json");
    let decoded = GerbilCommandProfile::from_json(&encoded).expect("profile should decode");

    assert_eq!(decoded, profile);
}

#[test]
fn command_profile_builds_exec_spec_without_shell_parsing() {
    let profile = GerbilCommandProfile::new("/opt/gerbil/bin/gxi")
        .arg("--stdio-json")
        .arg("(import :marlin/compiler)")
        .env("GERBIL_LOADPATH", "gerbil/src");
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

#[test]
fn command_compiler_builds_default_marlin_runtime_module_with_assets() {
    let root = test_root("default-runtime-module");
    let compiler = GerbilCommandCompiler::from_default_marlin_runtime_module(root.path())
        .expect("default runtime compiler should write loadpath assets");
    let spec = compiler.spec();

    assert_eq!(spec.program, default_gerbil_gxi_program());
    assert_eq!(spec.args.len(), 1);
    assert_eq!(spec.args[0].to_string_lossy(), GERBIL_ADAPTER_MODULE);
    assert_eq!(
        spec.env
            .iter()
            .find(|(key, _value)| key.to_string_lossy() == GERBIL_LOADPATH_ENV)
            .map(|(_key, value)| value.to_string_lossy()),
        Some(gerbil_runtime_loadpath(root.path()).to_string_lossy())
    );
    assert!(root.path().join(GERBIL_COMMAND_ADAPTER_PATH).is_file());
    assert!(root.path().join(GERBIL_MARLIN_ADAPTER_PATH).is_file());
}

#[test]
fn runtime_binding_writes_assets_and_exposes_compiler_spec() {
    let root = test_root("runtime-binding");
    let binding = GerbilRuntimeBinding::from_default_gxi(root.path())
        .expect("runtime binding should write assets");

    assert_eq!(binding.loadpath_root(), root.path());
    assert!(
        binding
            .written_assets()
            .iter()
            .any(|asset| asset.ends_with(GERBIL_COMMAND_ADAPTER_PATH))
    );
    assert!(
        binding
            .written_assets()
            .iter()
            .any(|asset| asset.ends_with(GERBIL_MARLIN_PROTOCOL_PATH))
    );
    assert_eq!(binding.spec().program, default_gerbil_gxi_program());
    assert_eq!(binding.spec().args.len(), 1);
    assert_eq!(
        binding.spec().args[0].to_string_lossy(),
        GERBIL_ADAPTER_MODULE
    );
    assert_eq!(binding.compiler().spec(), binding.spec());
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
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"from-profile-json\",\"nodes\":[],\"edges\":[]}}}'",
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

fn test_root(name: &str) -> TempDir {
    Builder::new()
        .prefix(&format!("marlin-gerbil-scheme-{name}-"))
        .tempdir()
        .unwrap_or_else(|error| panic!("creates {name} test root: {error}"))
}

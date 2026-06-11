use marlin_gerbil_ir::CompiledLoopGraph;
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCommandProfile, GerbilCommandSpec,
    GerbilCompileRequest, GerbilCompileResponse, GerbilCompiledArtifact, GerbilCompiler,
    GerbilSource, GerbilWorkspaceContractFacts,
};
use marlin_org_model::{
    OrgContractRegistry, OrgContractResolutionReport, OrgContractValidationReport,
};
use std::{env, path::PathBuf};

fn loop_graph_artifact(graph_id: &str) -> GerbilCompiledArtifact {
    GerbilCompiledArtifact::LoopGraph(CompiledLoopGraph {
        graph_id: graph_id.to_string(),
        nodes: Vec::new(),
        edges: Vec::new(),
    })
}

const RICH_LOOP_GRAPH_SOURCE: &str = r#"(loop-graph gerbil-source-loop
  (node provider ask-model (config role planner retries one))
  (node tool run-tool (config mode execute))
  (edge provider tool success)
  (edge tool provider none))"#;

fn local_gxi() -> Option<PathBuf> {
    let gxi = env::var("MARLIN_GERBIL_GXI")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/opt/homebrew/opt/gerbil-scheme/bin/gxi"));

    if !gxi.exists() {
        eprintln!(
            "skipping real gxi test because {} is missing",
            gxi.display()
        );
        return None;
    }

    Some(gxi)
}

fn gerbil_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("gerbil")
}

fn assert_rich_loop_graph_artifact(artifact: GerbilCompiledArtifact) {
    match artifact {
        GerbilCompiledArtifact::LoopGraph(graph) => {
            assert_eq!(graph.graph_id, "gerbil-source-loop");
            assert_eq!(graph.nodes.len(), 2);
            assert_eq!(graph.nodes[0].id, "provider");
            assert_eq!(graph.nodes[0].executor, "ask-model");
            assert_eq!(
                graph.nodes[0].config.get("role").map(String::as_str),
                Some("planner")
            );
            assert_eq!(
                graph.nodes[0].config.get("retries").map(String::as_str),
                Some("one")
            );
            assert_eq!(graph.nodes[1].id, "tool");
            assert_eq!(graph.nodes[1].executor, "run-tool");
            assert_eq!(
                graph.nodes[1].config.get("mode").map(String::as_str),
                Some("execute")
            );
            assert_eq!(graph.edges.len(), 2);
            assert_eq!(graph.edges[0].from, "provider");
            assert_eq!(graph.edges[0].to, "tool");
            assert_eq!(graph.edges[0].condition.as_deref(), Some("success"));
            assert_eq!(graph.edges[1].from, "tool");
            assert_eq!(graph.edges[1].to, "provider");
            assert_eq!(graph.edges[1].condition, None);
        }
        other => panic!("expected loop graph artifact, got {other:?}"),
    }
}

#[test]
fn command_protocol_round_trips_json_contract() {
    let request = GerbilCompileRequest {
        source: GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
        expected: GerbilArtifactKind::LoopGraph,
        contract_facts: Some(GerbilWorkspaceContractFacts {
            registry: OrgContractRegistry::default(),
            resolutions: OrgContractResolutionReport::default(),
            validations: OrgContractValidationReport::default(),
        }),
    };

    let encoded = serde_json::to_string(&request).expect("request should encode as json");
    let decoded: GerbilCompileRequest =
        serde_json::from_str(&encoded).expect("request should decode from json");

    assert_eq!(decoded, request);
    assert!(decoded.contract_facts.is_some());
}

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
fn command_compiler_reads_typed_artifact_from_stdout() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"from-command\",\"nodes\":[],\"edges\":[]}}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .expect("command output should decode to requested artifact kind");

    assert_eq!(artifact, loop_graph_artifact("from-command"));
}

#[test]
fn command_compiler_passes_configured_environment() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"'\"$MARLIN_GERBIL_TEST_GRAPH\"'\",\"nodes\":[],\"edges\":[]}}}'",
    )
    .env("MARLIN_GERBIL_TEST_GRAPH", "from-env");
    let compiler = GerbilCommandCompiler::new(command);

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .expect("command should receive configured environment values");

    assert_eq!(artifact, loop_graph_artifact("from-env"));
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

#[test]
fn command_compiler_rejects_mismatched_artifact_kind() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "printf '%s\n' '{\"artifact\":{\"WorkspaceSchema\":{\"schema_id\":\"workspace-record\",\"required_properties\":[\"ID\"],\"todo_states\":[\"TODO\",\"DONE\"]}}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let error = compiler
        .compile(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .unwrap_err();

    assert!(error.contains("expected LoopGraph"));
    assert!(error.contains("got WorkspaceSchema"));
}

#[test]
fn command_response_carries_typed_artifact() {
    let response = GerbilCompileResponse {
        artifact: loop_graph_artifact("response-loop"),
    };

    assert_eq!(response.artifact.kind(), GerbilArtifactKind::LoopGraph);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_fixture() {
    let Some(gxi) = local_gxi() else {
        return;
    };

    let fixture_root = gerbil_fixture_root();
    let fixture = fixture_root.join("fixed-loop-graph-adapter.ss");
    let compiler = GerbilCommandCompiler::new(
        GerbilCommandSpec::new(gxi)
            .env("GERBIL_LOADPATH", fixture_root.as_os_str().to_os_string())
            .arg(fixture),
    );

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            GerbilArtifactKind::LoopGraph,
        )
        .expect("real gxi fixture should compile source text into a typed loop graph artifact");

    assert_rich_loop_graph_artifact(artifact);
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_can_call_real_gxi_module_entry() {
    let Some(gxi) = local_gxi() else {
        return;
    };

    let fixture_root = gerbil_fixture_root();
    let compiler = GerbilCommandCompiler::new(
        GerbilCommandSpec::new(gxi)
            .env("GERBIL_LOADPATH", fixture_root.as_os_str().to_os_string())
            .arg(":marlin/adapter"),
    );

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            GerbilArtifactKind::LoopGraph,
        )
        .expect(
            "real gxi module entry should compile source text into a typed loop graph artifact",
        );

    assert_rich_loop_graph_artifact(artifact);
}

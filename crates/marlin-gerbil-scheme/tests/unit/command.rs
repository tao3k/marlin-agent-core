use marlin_gerbil_ir::CompiledLoopGraph;
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCommandProfile, GerbilCommandSpec,
    GerbilCompileRequest, GerbilCompileResponse, GerbilCompiledArtifact, GerbilCompiler,
    GerbilSource,
};
use std::{env, path::PathBuf};

fn loop_graph_artifact(graph_id: &str) -> GerbilCompiledArtifact {
    GerbilCompiledArtifact::LoopGraph(CompiledLoopGraph {
        graph_id: graph_id.to_string(),
        nodes: Vec::new(),
        edges: Vec::new(),
    })
}

#[test]
fn command_protocol_round_trips_json_contract() {
    let request = GerbilCompileRequest {
        source: GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
        expected: GerbilArtifactKind::LoopGraph,
    };

    let encoded = serde_json::to_string(&request).expect("request should encode as json");
    let decoded: GerbilCompileRequest =
        serde_json::from_str(&encoded).expect("request should decode from json");

    assert_eq!(decoded, request);
}

#[test]
fn command_profile_round_trips_json_configuration() {
    let profile = GerbilCommandProfile::new("/opt/gerbil/bin/gxi")
        .arg("--stdio-json")
        .current_dir(".data/gerbil");

    let encoded = serde_json::to_string(&profile).expect("profile should encode as json");
    let decoded = GerbilCommandProfile::from_json(&encoded).expect("profile should decode");

    assert_eq!(decoded, profile);
}

#[test]
fn command_profile_builds_exec_spec_without_shell_parsing() {
    let profile = GerbilCommandProfile::new("/opt/gerbil/bin/gxi")
        .arg("--stdio-json")
        .arg("(import :marlin/compiler)");
    let spec: GerbilCommandSpec = profile.into();

    assert_eq!(spec.program.to_string_lossy(), "/opt/gerbil/bin/gxi");
    assert_eq!(spec.args.len(), 2);
    assert_eq!(spec.args[0].to_string_lossy(), "--stdio-json");
    assert_eq!(spec.args[1].to_string_lossy(), "(import :marlin/compiler)");
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
    let gxi = env::var("MARLIN_GERBIL_GXI")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/opt/homebrew/opt/gerbil-scheme/bin/gxi"));

    if !gxi.exists() {
        eprintln!(
            "skipping real gxi fixture test because {} is missing",
            gxi.display()
        );
        return;
    }

    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("gerbil");
    let fixture = fixture_root.join("fixed-loop-graph-adapter.ss");
    let compiler = GerbilCommandCompiler::new(
        GerbilCommandSpec::new("/usr/bin/env")
            .arg(format!("GERBIL_LOADPATH={}", fixture_root.display()))
            .arg(gxi)
            .arg(fixture),
    );

    let artifact = compiler
        .compile(
            GerbilSource::new(
                "audit/control-plane",
                r#"(loop-graph gerbil-source-loop
  (node provider ask-model (config role planner retries one))
  (node tool run-tool (config mode execute))
  (edge provider tool success)
  (edge tool provider none))"#,
            ),
            GerbilArtifactKind::LoopGraph,
        )
        .expect("real gxi fixture should compile source text into a typed loop graph artifact");

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

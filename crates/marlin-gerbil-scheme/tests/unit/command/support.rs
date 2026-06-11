use marlin_gerbil_ir::CompiledLoopGraph;
use marlin_gerbil_scheme::{GerbilCommandCompiler, GerbilCommandSpec, GerbilCompiledArtifact};
use std::{env, path::PathBuf};

pub fn loop_graph_artifact(graph_id: &str) -> GerbilCompiledArtifact {
    GerbilCompiledArtifact::LoopGraph(CompiledLoopGraph {
        graph_id: graph_id.to_string(),
        nodes: Vec::new(),
        edges: Vec::new(),
    })
}

pub const RICH_LOOP_GRAPH_SOURCE: &str = r#"(loop-graph gerbil-source-loop
  (node provider ask-model (config role planner retries one))
  (node tool run-tool (config mode execute))
  (edge provider tool success)
  (edge tool provider none))"#;

pub fn local_gxi() -> Option<PathBuf> {
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

pub fn gerbil_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("gerbil")
}

pub fn real_gxi_module_compiler() -> Option<GerbilCommandCompiler> {
    let gxi = local_gxi()?;
    let fixture_root = gerbil_fixture_root();
    Some(GerbilCommandCompiler::new(
        GerbilCommandSpec::new(gxi)
            .env("GERBIL_LOADPATH", fixture_root.as_os_str().to_os_string())
            .arg(":marlin/adapter"),
    ))
}

pub fn assert_rich_loop_graph_artifact(artifact: GerbilCompiledArtifact) {
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

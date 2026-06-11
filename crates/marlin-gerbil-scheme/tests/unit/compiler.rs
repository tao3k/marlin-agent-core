use marlin_gerbil_ir::{CompiledLoopGraph, WorkspaceSchemaSpec};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCompiledArtifact, GerbilCompiler, GerbilSource, compile_checked,
};

struct MatchingCompiler;

impl GerbilCompiler for MatchingCompiler {
    fn compile(
        &self,
        source: GerbilSource,
        expected: GerbilArtifactKind,
    ) -> Result<GerbilCompiledArtifact, String> {
        assert_eq!(source.module, "audit/control-plane");
        assert_eq!(expected, GerbilArtifactKind::LoopGraph);

        Ok(GerbilCompiledArtifact::LoopGraph(CompiledLoopGraph {
            graph_id: "audit-loop".to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }))
    }
}

struct MismatchedCompiler;

impl GerbilCompiler for MismatchedCompiler {
    fn compile(
        &self,
        _source: GerbilSource,
        _expected: GerbilArtifactKind,
    ) -> Result<GerbilCompiledArtifact, String> {
        Ok(GerbilCompiledArtifact::WorkspaceSchema(
            WorkspaceSchemaSpec {
                schema_id: "workspace-record".to_string(),
                required_properties: vec!["ID".to_string()],
                todo_states: vec!["TODO".to_string(), "DONE".to_string()],
            },
        ))
    }
}

#[test]
fn compile_checked_accepts_matching_artifact_kind() {
    let artifact = compile_checked(
        &MatchingCompiler,
        GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
        GerbilArtifactKind::LoopGraph,
    )
    .expect("compiler output should match requested artifact kind");

    assert_eq!(artifact.kind(), GerbilArtifactKind::LoopGraph);
}

#[test]
fn compile_checked_rejects_mismatched_artifact_kind() {
    let error = compile_checked(
        &MismatchedCompiler,
        GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
        GerbilArtifactKind::LoopGraph,
    )
    .unwrap_err();

    assert!(error.contains("expected LoopGraph"));
    assert!(error.contains("got WorkspaceSchema"));
}

use super::support::{
    RELEASE_TOPOLOGY_SOURCE, assert_release_topology_artifact,
    assert_workspace_patch_intent_artifact, assert_workspace_schema_artifact, loop_graph_artifact,
};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCommandSpec, GerbilCompileRequest,
    GerbilCompiledArtifact, GerbilCompiler, GerbilSource, GerbilWorkspaceContractFacts,
};
use marlin_org_model::{
    OrgContractRegistry, OrgContractResolutionReport, OrgContractValidationReport,
};

#[test]
fn command_compiler_reads_typed_artifact_from_stdout() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"from-command\",\"nodes\":[],\"edges\":[]}}}'",
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
fn command_compiler_reads_batched_artifacts_from_stdout_lines() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"batch-graph\",\"nodes\":[],\"edges\":[]}}}' '{\"artifact\":{\"WorkspaceSchema\":{\"schema_id\":\"workspace-record\",\"required_properties\":[\"ID\",\"TITLE\"],\"todo_states\":[\"TODO\",\"DONE\"]}}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let artifacts = compiler
        .compile_requests(vec![
            GerbilCompileRequest::new(
                GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
                GerbilArtifactKind::LoopGraph,
            ),
            GerbilCompileRequest::new(
                GerbilSource::new(
                    "audit/workspace-schema",
                    "(workspace-schema workspace-record)",
                ),
                GerbilArtifactKind::WorkspaceSchema,
            ),
        ])
        .expect("command output should decode newline-delimited artifacts");

    assert_eq!(artifacts.len(), 2);
    assert_eq!(artifacts[0], loop_graph_artifact("batch-graph"));
    assert_workspace_schema_artifact(artifacts[1].clone());
}

#[test]
fn command_compiler_preserves_batched_request_errors_from_stdout_lines() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"batch-graph\",\"nodes\":[],\"edges\":[]}}}' '{\"error\":{\"message\":\"expected workspace-schema form: loop-graph\"}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let results = compiler
        .compile_request_results(vec![
            GerbilCompileRequest {
                source: GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
                expected: GerbilArtifactKind::LoopGraph,
                contract_facts: None,
            },
            GerbilCompileRequest {
                source: GerbilSource::new("audit/control-plane", "(loop-graph invalid)"),
                expected: GerbilArtifactKind::WorkspaceSchema,
                contract_facts: None,
            },
        ])
        .expect("command output should decode newline-delimited request results");

    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0].as_ref().expect("first result should succeed"),
        &loop_graph_artifact("batch-graph")
    );
    let error = results[1].as_ref().expect_err("second result should fail");
    assert!(error.contains("gerbil compiler command failed for request 1"));
    assert!(error.contains("expected workspace-schema form"));
    assert!(error.contains("loop-graph"));
}

#[test]
fn command_compiler_reads_workspace_schema_from_stdout() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"WorkspaceSchema\":{\"schema_id\":\"workspace-record\",\"required_properties\":[\"ID\",\"TITLE\"],\"todo_states\":[\"TODO\",\"DONE\"]}}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let artifact = compiler
        .compile(
            GerbilSource::new(
                "audit/workspace-schema",
                "(workspace-schema workspace-record)",
            ),
            GerbilArtifactKind::WorkspaceSchema,
        )
        .expect("command output should decode to requested workspace schema artifact kind");

    assert_workspace_schema_artifact(artifact);
}

#[test]
fn command_compiler_reads_workspace_patch_intent_from_stdout() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"WorkspacePatchIntent\":{\"intent_id\":\"intent:memory\",\"patch\":{\"reason\":\"gerbil intent\",\"source_agent\":\"gerbil\",\"ops\":[{\"SetTodo\":{\"node\":\"memory.org:1:goal\",\"state\":\"Done\"}},{\"SetProperty\":{\"node\":\"memory.org:1:goal\",\"key\":\"OWNER\",\"value\":\"gerbil\"}},{\"MarkMemoryCandidate\":{\"node\":\"memory.org:1:goal\",\"dispatch\":\"long-term\"}}]},\"dry_run_first\":true}}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let artifact = compiler
        .compile(
            GerbilSource::new(
                "audit/workspace-patch-intent",
                "(workspace-patch-intent \"intent:memory\")",
            ),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .expect("command output should decode to requested workspace patch intent artifact kind");

    assert_workspace_patch_intent_artifact(artifact);
}

#[test]
fn command_compiler_sends_default_contract_facts_for_workspace_patch_intent() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "if grep -q 'contract_facts'; then intent=with-contracts; else intent=missing-contracts; fi; printf '%s\n' \"{\\\"artifact\\\":{\\\"WorkspacePatchIntent\\\":{\\\"intent_id\\\":\\\"$intent\\\",\\\"patch\\\":{\\\"reason\\\":\\\"gerbil intent\\\",\\\"source_agent\\\":\\\"gerbil\\\",\\\"ops\\\":[]},\\\"dry_run_first\\\":true}}}\"",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let artifact = compiler
        .compile(
            GerbilSource::new(
                "audit/workspace-patch-intent",
                "(workspace-patch-intent \"intent:memory\")",
            ),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .expect("workspace patch-intent compile should send contract facts by default");

    let GerbilCompiledArtifact::WorkspacePatchIntent(intent) = artifact else {
        panic!("expected workspace patch-intent artifact");
    };
    assert_eq!(intent.intent_id, "with-contracts");
}

#[test]
fn command_compiler_reads_release_topology_from_stdout() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"ReleaseTopology\":{\"topology_id\":\"release:gerbil\",\"crate_name\":\"marlin-gerbil-scheme\",\"publish_enabled\":false,\"asset_audit_command\":\"cargo package -p marlin-gerbil-scheme --allow-dirty --no-verify --list\",\"package_assets\":[\"README.md\",\"fixtures/gerbil\"],\"runtime_dependency_chain\":[\"marlin-gerbil-ir\",\"marlin-workspace-patch\"],\"workflow_dependency_chain\":[\"marlin-org-workflow\",\"marlin-org-store\"],\"gates\":[{\"gate_id\":\"real-gxi\",\"command\":\"cargo test -p marlin-gerbil-scheme --test unit_test command::real_gxi -- --ignored\",\"requires_local_gerbil\":true,\"required_artifacts\":[\"workspace_schema\",\"workspace_patch_intent\"],\"visibility\":[{\"report_key\":\"real_gxi_release_gate\",\"evidence_keys\":[\"workspace_schema\",\"workspace_patch_intent\"],\"artifact_paths\":[\"fixtures/gerbil/command-adapter.ss\"]}]}]}}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let artifact = compiler
        .compile(
            GerbilSource::new("audit/release-topology", RELEASE_TOPOLOGY_SOURCE),
            GerbilArtifactKind::ReleaseTopology,
        )
        .expect("command output should decode to requested release topology artifact kind");

    assert_release_topology_artifact(artifact);
}

#[test]
fn command_compiler_passes_configured_environment() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"'\"$MARLIN_GERBIL_TEST_GRAPH\"'\",\"nodes\":[],\"edges\":[]}}}'",
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
fn compile_request_defaults_workspace_patch_intent_contract_facts() {
    let request = GerbilCompileRequest::new(
        GerbilSource::new(
            "audit/workspace-patch-intent",
            "(workspace-patch-intent \"intent:memory\")",
        ),
        GerbilArtifactKind::WorkspacePatchIntent,
    );
    let schema_request = GerbilCompileRequest::new(
        GerbilSource::new(
            "audit/workspace-schema",
            "(workspace-schema workspace-record)",
        ),
        GerbilArtifactKind::WorkspaceSchema,
    );

    assert!(
        request.contract_facts.is_some(),
        "workspace patch-intent requests carry contract facts by default"
    );
    assert!(schema_request.contract_facts.is_none());
}

#[test]
fn command_compiler_sends_contract_facts_when_requested() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "if grep -q 'contract_facts'; then graph=with-contracts; else graph=missing-contracts; fi; printf '%s\n' \"{\\\"artifact\\\":{\\\"LoopGraph\\\":{\\\"graph_id\\\":\\\"$graph\\\",\\\"nodes\\\":[],\\\"edges\\\":[]}}}\"",
    );
    let compiler = GerbilCommandCompiler::new(command);

    let artifact = compiler
        .compile_with_contract_facts(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
            GerbilWorkspaceContractFacts {
                registry: OrgContractRegistry::default(),
                resolutions: OrgContractResolutionReport::default(),
                validations: OrgContractValidationReport::default(),
            },
        )
        .expect("command should receive contract facts in request json");

    assert_eq!(artifact, loop_graph_artifact("with-contracts"));
}

#[test]
fn command_compiler_reports_stdout_diagnostics_when_command_fails() {
    let command = GerbilCommandSpec::new("/bin/sh")
        .arg("-c")
        .arg("cat >/dev/null; printf '%s\n' 'adapter expected LoopGraph'; exit 70");
    let compiler = GerbilCommandCompiler::new(command);

    let error = compiler
        .compile(
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .unwrap_err();

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("exit status: 70"));
    assert!(error.contains("adapter expected LoopGraph"));
}

#[test]
fn command_compiler_rejects_mismatched_artifact_kind() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"WorkspaceSchema\":{\"schema_id\":\"workspace-record\",\"required_properties\":[\"ID\"],\"todo_states\":[\"TODO\",\"DONE\"]}}}'",
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

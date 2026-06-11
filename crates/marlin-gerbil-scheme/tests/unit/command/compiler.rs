use super::support::{
    RELEASE_TOPOLOGY_SOURCE, assert_release_topology_artifact,
    assert_workspace_patch_intent_artifact, assert_workspace_schema_artifact, loop_graph_artifact,
};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCommandSpec, GerbilCompiler, GerbilSource,
    GerbilWorkspaceContractFacts,
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

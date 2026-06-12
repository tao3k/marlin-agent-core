use marlin_gerbil_ir::CompiledLoopGraph;
use marlin_gerbil_scheme::{
    GerbilCommandCompiler, GerbilCommandSpec, GerbilCompiledArtifact, default_gerbil_gxi_program,
};
use marlin_org_model::TodoState;
use marlin_workspace_patch::WorkspacePatchOp;
use std::path::PathBuf;

pub const MARLIN_REQUIRE_REAL_GXI_ENV: &str = "MARLIN_REQUIRE_REAL_GXI";

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

pub const WORKSPACE_SCHEMA_SOURCE: &str = r#"(workspace-schema workspace-record
  (required ID TITLE)
  (todo TODO DONE))"#;

pub const WORKSPACE_PATCH_INTENT_SOURCE: &str = r#"(workspace-patch-intent "intent:memory"
  (dry-run-first #t)
  (patch
    (reason "gerbil intent")
    (source-agent "gerbil")
    (set-todo "memory.org:1:goal" DONE)
    (set-property "memory.org:1:goal" OWNER "gerbil")
    (mark-memory-candidate "memory.org:1:goal" "long-term")))"#;

pub const WORKSPACE_SOURCE_COMMIT_INTENT_SOURCE: &str = r#"(workspace-patch-intent "intent:source-commit"
  (dry-run-first #t)
  (patch
    (reason "gerbil source commit")
    (source-agent "gerbil")
    (set-todo "memory.org:1:goal" DONE)
    (set-property "memory.org:1:goal" OWNER "gerbil")))"#;

pub const AGENT_SCENARIO_CONTRACT_SOURCE: &str = r#"(agent-scenario-contract gerbil-scenario
  (description "from gerbil")
  (step run
    (input path LOOP.org)
    (event-topic kernel.execution)
    (span-name harness.execution))
  (evidence Runtime))"#;

pub const RELEASE_TOPOLOGY_SOURCE: &str = r#"(release-topology "release:gerbil"
  (crate "marlin-gerbil-scheme")
  (publish-enabled #f)
  (asset-audit-command "cargo package -p marlin-gerbil-scheme --allow-dirty --no-verify --list")
  (package-assets README.md "gerbil")
  (runtime-dependency-chain "marlin-gerbil-ir" "marlin-workspace-patch")
  (workflow-dependency-chain "marlin-org-workflow" "marlin-org-store")
  (gate real-gxi
    (command "cargo test -p marlin-gerbil-scheme --test unit_test command::real_gxi -- --ignored")
    (requires-local-gerbil #t)
    (required-artifacts workspace_schema workspace_patch_intent)
    (visibility
      (report-key real_gxi_release_gate)
      (evidence-keys workspace_schema workspace_patch_intent)
      (artifact-paths "gerbil/bin/command-adapter.ss"))))"#;

pub fn local_gxi() -> Option<PathBuf> {
    let gxi = default_gerbil_gxi_program();

    if !gxi.exists() {
        let message = format!(
            "skipping real gxi test because {} is missing",
            gxi.display()
        );
        if std::env::var_os(MARLIN_REQUIRE_REAL_GXI_ENV).is_some() {
            panic!(
                "{message}; unset {MARLIN_REQUIRE_REAL_GXI_ENV} or set MARLIN_GERBIL_GXI to an existing executable"
            );
        }
        eprintln!("{message}");
        return None;
    }

    Some(gxi)
}

pub fn gerbil_runtime_package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gerbil")
}

pub fn real_gxi_module_compiler() -> Option<GerbilCommandCompiler> {
    let gxi = local_gxi()?;
    let runtime_package_root = gerbil_runtime_package_root();
    Some(GerbilCommandCompiler::from_marlin_runtime_module(
        gxi,
        runtime_package_root,
    ))
}

pub fn real_gxi_command_adapter_batch_compiler() -> Option<GerbilCommandCompiler> {
    let gxi = local_gxi()?;
    let runtime_package_root = gerbil_runtime_package_root();
    Some(GerbilCommandCompiler::new(
        GerbilCommandSpec::marlin_runtime_batch_launcher(gxi, runtime_package_root),
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

pub fn assert_workspace_schema_artifact(artifact: GerbilCompiledArtifact) {
    match artifact {
        GerbilCompiledArtifact::WorkspaceSchema(schema) => {
            assert_eq!(schema.schema_id, "workspace-record");
            assert_eq!(schema.required_properties, ["ID", "TITLE"]);
            assert_eq!(schema.todo_states, ["TODO", "DONE"]);
        }
        other => panic!("expected workspace schema artifact, got {other:?}"),
    }
}

pub fn assert_workspace_patch_intent_artifact(artifact: GerbilCompiledArtifact) {
    match artifact {
        GerbilCompiledArtifact::WorkspacePatchIntent(intent) => {
            assert_eq!(intent.intent_id, "intent:memory");
            assert!(intent.dry_run_first);
            assert_eq!(intent.patch.reason, "gerbil intent");
            assert_eq!(intent.patch.source_agent.as_deref(), Some("gerbil"));
            assert_eq!(intent.patch.ops.len(), 3);
            match &intent.patch.ops[0] {
                WorkspacePatchOp::SetTodo { node, state } => {
                    assert_eq!(node.as_str(), "memory.org:1:goal");
                    assert_eq!(state, &TodoState::Done);
                }
                other => panic!("expected set-todo op, got {other:?}"),
            }
            match &intent.patch.ops[1] {
                WorkspacePatchOp::SetProperty { node, key, value } => {
                    assert_eq!(node.as_str(), "memory.org:1:goal");
                    assert_eq!(key, "OWNER");
                    assert_eq!(value, "gerbil");
                }
                other => panic!("expected set-property op, got {other:?}"),
            }
            match &intent.patch.ops[2] {
                WorkspacePatchOp::MarkMemoryCandidate { node, dispatch } => {
                    assert_eq!(node.as_str(), "memory.org:1:goal");
                    assert_eq!(dispatch, "long-term");
                }
                other => panic!("expected mark-memory-candidate op, got {other:?}"),
            }
        }
        other => panic!("expected workspace patch intent artifact, got {other:?}"),
    }
}

pub fn assert_release_topology_artifact(artifact: GerbilCompiledArtifact) {
    match artifact {
        GerbilCompiledArtifact::ReleaseTopology(topology) => {
            assert_eq!(topology.topology_id, "release:gerbil");
            assert_eq!(topology.crate_name, "marlin-gerbil-scheme");
            assert!(!topology.publish_enabled);
            assert_eq!(
                topology.asset_audit_command,
                "cargo package -p marlin-gerbil-scheme --allow-dirty --no-verify --list"
            );
            assert_eq!(topology.package_assets, ["README.md", "gerbil"]);
            assert_eq!(
                topology.runtime_dependency_chain,
                ["marlin-gerbil-ir", "marlin-workspace-patch"]
            );
            assert_eq!(
                topology.workflow_dependency_chain,
                ["marlin-org-workflow", "marlin-org-store"]
            );
            assert_eq!(topology.gates.len(), 1);
            assert_eq!(topology.gates[0].gate_id, "real-gxi");
            assert_eq!(
                topology.gates[0].command,
                "cargo test -p marlin-gerbil-scheme --test unit_test command::real_gxi -- --ignored"
            );
            assert!(topology.gates[0].requires_local_gerbil);
            assert_eq!(
                topology.gates[0].required_artifacts,
                ["workspace_schema", "workspace_patch_intent"]
            );
            assert_eq!(
                topology.gates[0].visibility[0].report_key,
                "real_gxi_release_gate"
            );
            assert_eq!(
                topology.gates[0].visibility[0].evidence_keys,
                ["workspace_schema", "workspace_patch_intent"]
            );
            assert_eq!(
                topology.gates[0].visibility[0].artifact_paths,
                ["gerbil/bin/command-adapter.ss"]
            );
        }
        other => panic!("expected release topology artifact, got {other:?}"),
    }
}

pub fn assert_agent_scenario_contract_artifact(artifact: GerbilCompiledArtifact) {
    match artifact {
        GerbilCompiledArtifact::AgentScenarioContract(contract) => {
            assert!(contract.is_supported_schema());
            assert_eq!(contract.scenario.id, "gerbil-scenario");
            assert_eq!(
                contract.scenario.description.as_deref(),
                Some("from gerbil")
            );
            assert_eq!(contract.scenario.steps.len(), 1);
            let step = &contract.scenario.steps[0];
            assert_eq!(step.name, "run");
            assert_eq!(step.input.get("path").map(String::as_str), Some("LOOP.org"));
            assert_eq!(
                step.expected_event_topics
                    .iter()
                    .map(|topic| topic.as_str())
                    .collect::<Vec<_>>(),
                vec!["kernel.execution"]
            );
            assert_eq!(
                step.expected_span_names
                    .iter()
                    .map(|span| span.as_str())
                    .collect::<Vec<_>>(),
                vec!["harness.execution"]
            );
            assert_eq!(
                contract
                    .scenario
                    .expected_evidence
                    .iter()
                    .map(|kind| format!("{kind:?}"))
                    .collect::<Vec<_>>(),
                vec!["Runtime"]
            );
        }
        other => panic!("expected agent scenario contract artifact, got {other:?}"),
    }
}

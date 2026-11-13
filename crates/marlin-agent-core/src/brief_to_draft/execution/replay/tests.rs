// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::{
    GraphLoopStrategy, GraphNodeExecutionReceipt, RuntimePlanSnapshot, RuntimeWorkspaceProject,
    RuntimeWorkspaceProjectId,
};
use marlin_agent_runtime::plugin::{
    ContributionId, EnableWorkflow, PluginDenial, ProjectGrant, ReleaseAttestation, ReleaseTrust,
    SignedRelease, WorkflowRegistration, WorkspaceEffectError, WorkspaceFileBroker,
};
use ring::signature::{Ed25519KeyPair, KeyPair};

use super::super::{CREATE, RENDER, graph};
use super::*;

const MANIFEST: &str =
    include_str!("../../../../../../scheme/t/fixtures/brief-to-draft/plugin.toml");

#[test]
fn replay_checks_the_signed_workflow_contract_after_authenticating_the_archive() {
    let key = Ed25519KeyPair::from_seed_unchecked(&[3; 32]).unwrap();
    let attestation = ReleaseAttestation {
        publisher: "marlin.fixture".to_owned().try_into().unwrap(),
        source_revision: [2; 20],
        build_evidence_digest: [4; 32],
    };
    let keys = [(
        attestation.publisher.clone(),
        key.public_key().as_ref().try_into().unwrap(),
    )];
    let trust = ReleaseTrust::new(keys.clone().into());
    let original_signature = key.sign(&SignedRelease::signing_bytes(
        MANIFEST.as_bytes(),
        &attestation,
    ));
    let signed = SignedRelease {
        manifest: MANIFEST.as_bytes(),
        attestation: attestation.clone(),
        signature: original_signature.as_ref().try_into().unwrap(),
    };
    let release = trust
        .verify(SignedRelease {
            attestation: signed.attestation.clone(),
            ..signed
        })
        .unwrap();
    let render_input = BriefRenderInput {
        title: "Launch brief".into(),
        audience: "Operators".into(),
        audience_required: true,
        release_note: true,
    };
    let settings_digest = render_input.settings_digest();
    let mut host = PluginHost::new(
        ReleaseTrust::new(keys.into()),
        [WorkflowRegistration {
            policy_module: policy_module(),
            input_schema: INPUT_SCHEMA.to_owned().try_into().unwrap(),
            settings_schema: SETTINGS_SCHEMA.to_owned().try_into().unwrap(),
            required_capabilities: file_capabilities(),
            policy_revision: [1; 32],
            validate_settings: Box::new(move |_, _| Ok(settings_digest)),
        }],
    )
    .unwrap();
    let bundle = host.install(signed).unwrap();
    let project = RuntimeWorkspaceProjectId::new("fixture");
    host.replace_project_grant(ProjectGrant {
        project: project.clone(),
        bundle,
        revision: 1,
        capabilities: file_capabilities(),
    })
    .unwrap();
    let workflow = ContributionId::try_from("brief-to-draft".to_owned()).unwrap();
    let activation = host
        .enable(EnableWorkflow {
            plugin: release.id(),
            version: release.version(),
            workflow: &workflow,
            project: &project,
            project_settings: &toml::Table::new(),
        })
        .unwrap();

    let worktree = tempfile::tempdir().unwrap();
    let root = worktree.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("briefs")).unwrap();
    std::fs::create_dir(root.join("drafts")).unwrap();
    let brief = b"* Launch brief\n:PROPERTIES:\n:AUDIENCE: Operators\n:END:\n";
    std::fs::write(root.join("briefs/launch.org"), brief).unwrap();
    let broker =
        WorkspaceFileBroker::open(&RuntimeWorkspaceProject::new(project.as_str(), &root)).unwrap();
    let input = broker
        .read(&host, &activation.id, &capability("read-brief"))
        .unwrap();
    assert_eq!(input.value, brief);
    let draft = render_input.render();
    let create = capability("create-draft");
    let output = broker
        .create(&host, &activation.id, &create, &draft)
        .unwrap();
    let prepared = broker
        .prepare_create(&host, &activation.id, &create)
        .unwrap();
    assert_eq!(host.disable(&activation.id).unwrap().pending, 0);
    assert!(matches!(
        prepared.execute(b"blocked"),
        Err(WorkspaceEffectError::Admission(
            PluginDenial::StaleActivation
        ))
    ));
    assert_eq!(
        std::fs::read(root.join("drafts/launch.org")).unwrap(),
        draft
    );
    let graph = graph();
    let requirement = workflow_abi_requirement();
    let readiness = GraphCAbiReadinessReceipt::evaluate(
        &requirement,
        requirement.required_symbols.iter().cloned(),
    );
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::gerbil_c_abi("brief-to-draft", "v0"),
        graph.clone(),
        digest_ref(input.digest),
        digest_ref(hash(&serde_json::to_vec(&graph).unwrap())),
    )
    .with_c_abi_requirement(requirement);
    let snapshot = RuntimePlanSnapshot {
        run_id: "offline-fixture".into(),
        graph_id: graph.graph_id,
        active_node: None,
    };
    let admission = marlin_agent_kernel::compile_graph_policy_proposal_with_c_abi_readiness(
        &snapshot.run_id,
        &proposal,
        &readiness,
    );
    assert!(admission.is_accepted());
    let execution =
        GraphLoopExecutionResult::completed(snapshot, vec!["render".into(), "create".into()])
            .with_node_receipts(vec![
                GraphNodeExecutionReceipt::completed("render", RENDER),
                GraphNodeExecutionReceipt::completed("create", CREATE),
            ]);
    let archive = BriefRun {
        activation: activation.id,
        render_input,
        readiness,
        proposal,
        admission: admission.receipt,
        execution,
        input: (input.id, input.value),
        output: Ok(Some((output.id, draft))),
    }
    .export(&host)
    .unwrap();
    drop(broker);
    worktree.close().unwrap();
    assert!(!root.exists());
    assert!(BriefRunEvidence::verify(&archive, &hash(&archive), &release).is_ok());

    let policy = policy_module();
    for (declared, substitution) in [
        (policy.as_str(), "marlin.fixture.unrelated"),
        (INPUT_SCHEMA, "marlin.fixture.start.v0"),
        (SETTINGS_SCHEMA, "marlin.fixture.settings.v0"),
        ("drafts/launch.org", "drafts/unrelated.org"),
    ] {
        let alternate = MANIFEST.replace(declared, substitution);
        let signature = key.sign(&SignedRelease::signing_bytes(
            alternate.as_bytes(),
            &attestation,
        ));
        let release = trust
            .verify(SignedRelease {
                manifest: alternate.as_bytes(),
                attestation: attestation.clone(),
                signature: signature.as_ref().try_into().unwrap(),
            })
            .unwrap();
        let mut updated: serde_json::Value = serde_json::from_slice(&archive).unwrap();
        let bundle = serde_json::to_value(release.digest()).unwrap();
        updated["effects"]["activation"]["bundle"] = bundle.clone();
        updated["effects"]["grant"]["bundle"] = bundle;
        if declared == "drafts/launch.org" {
            updated["effects"]["grant"]["capabilities"]["create-draft"]["path"] =
                substitution.into();
        }
        let updated = serde_json::to_vec(&updated).unwrap();
        let evidence: BriefRunEvidence = serde_json::from_slice(&updated).unwrap();
        evidence.effects.validate_release(&release).unwrap();
        assert!(
            matches!(
                BriefRunEvidence::verify(&updated, &hash(&updated), &release),
                Err(WorkflowEvidenceError::Inconsistent)
            ),
            "{declared} accepted as {substitution}"
        );
    }
}

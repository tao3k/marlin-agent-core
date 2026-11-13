// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(unix)]

use gerbil_scheme::{GerbilRuntime, LinkedGerbilProgram};
use marlin_agent_core::brief_to_draft::{BriefRunEvidence, registration, run};
use marlin_agent_core::{GraphLoopExecutionStatus, RunId, TokioAgentRuntime};
use marlin_agent_protocol::{RuntimeWorkspaceProject, RuntimeWorkspaceProjectId};
use marlin_agent_runtime::plugin::{
    CapabilityId, ClosedWorkflowEvidence, EffectOutcome, EnableWorkflow, PluginDenial, PluginHost,
    ProjectGrant, ReleaseAttestation, ReleaseTrust, SignedRelease, VerifiedRelease,
    WorkflowEvidenceError, WorkspaceEffectError, WorkspaceFileBroker,
};
use marlin_gerbil_scheme::{BriefSettings, BriefTemplate, GerbilBriefPlanner, GerbilBriefSettings};
use ring::signature::{Ed25519KeyPair, KeyPair};

const MANIFEST: &str = include_str!("../../../../scheme/t/fixtures/brief-to-draft/plugin.toml");

unsafe extern "C" {
    fn ___LNK_marlin__brief__to__draft(
        state: *mut gerbil_scheme_sys::GerbilGlobalState,
    ) -> *mut gerbil_scheme_sys::GerbilModuleOrLink;
    fn marlin_brief_to_draft_validate_settings(root: i64) -> i64;
    fn marlin_brief_to_draft_propose(root: i64) -> i64;
    fn gerbil_scheme_rust_take_call_error_raw() -> i64;
}

#[test]
fn signed_workflow_enables_with_the_real_scheme_settings_policy() {
    // SAFETY: Cargo links the selected program and its matching SDK bridge.
    let program = unsafe { LinkedGerbilProgram::from_linker(___LNK_marlin__brief__to__draft) };
    let vm = GerbilRuntime::initialize_program_single_processor(program).unwrap();
    // SAFETY: the linked Scheme export uses gerbil-rs-rooted-call on this VM.
    let export = unsafe { vm.bind_object_export(marlin_brief_to_draft_validate_settings) }.unwrap();
    let policy = GerbilBriefSettings::from_linked_export(&vm, export);
    let release_note_settings = BriefSettings {
        require_audience: Some(true),
        template: Some(BriefTemplate::ReleaseNote),
    };
    let result = policy.validate(
        BriefSettings {
            require_audience: None,
            ..release_note_settings
        },
        BriefSettings::default(),
    );
    assert_eq!(
        result.unwrap_or_else(|error| panic!(
            "Scheme settings call failed: {error:?} {}",
            scheme_diagnostic(&vm)
        )),
        Ok(release_note_settings),
    );
    let registration = registration(policy, [1; 32]).unwrap();
    let key = Ed25519KeyPair::from_seed_unchecked(&[3; 32]).unwrap();
    let attestation = ReleaseAttestation {
        publisher: "marlin.fixture".to_owned().try_into().unwrap(),
        source_revision: [2; 20],
        build_evidence_digest: [4; 32],
    };
    let signature = key.sign(&SignedRelease::signing_bytes(
        MANIFEST.as_bytes(),
        &attestation,
    ));
    let signed = SignedRelease {
        manifest: MANIFEST.as_bytes(),
        attestation: attestation.clone(),
        signature: signature.as_ref().try_into().unwrap(),
    };
    let trust = ReleaseTrust::new(
        [(
            attestation.publisher,
            key.public_key().as_ref().try_into().unwrap(),
        )]
        .into(),
    );
    let release = trust
        .verify(SignedRelease {
            attestation: signed.attestation.clone(),
            ..signed
        })
        .unwrap();
    let unrelated_manifest = MANIFEST.replace(
        "marlin.brief-to-draft.draft-output",
        "marlin.fixture.unrelated",
    );
    let unrelated_signature = key.sign(&SignedRelease::signing_bytes(
        unrelated_manifest.as_bytes(),
        &signed.attestation,
    ));
    let unrelated_release = trust
        .verify(SignedRelease {
            manifest: unrelated_manifest.as_bytes(),
            attestation: signed.attestation.clone(),
            signature: unrelated_signature.as_ref().try_into().unwrap(),
        })
        .unwrap();
    let project = RuntimeWorkspaceProjectId::new("fixture-project");
    let read_id = CapabilityId::try_from("read-brief".to_owned()).unwrap();
    let write_id = CapabilityId::try_from("create-draft".to_owned()).unwrap();
    let write = registration.required_capabilities[&write_id].clone();
    let mut grant = ProjectGrant {
        project: project.clone(),
        bundle: release.digest(),
        revision: 1,
        capabilities: registration.required_capabilities.clone(),
    };
    let mut host = PluginHost::new(trust, [registration]).unwrap();
    assert_eq!(host.install(signed).unwrap(), release.digest());
    let workflow = "brief-to-draft".to_owned().try_into().unwrap();
    let request = |project_settings| EnableWorkflow {
        plugin: release.id(),
        version: release.version(),
        workflow: &workflow,
        project: &project,
        project_settings,
    };
    let defaults = toml::Table::new();
    assert_eq!(
        host.enable(request(&defaults)),
        Err(PluginDenial::ScopeMismatch)
    );
    let mut read_only = grant.clone();
    read_only.capabilities.retain(|id, _| *id == read_id);
    host.replace_project_grant(read_only).unwrap();
    assert_eq!(
        host.enable(request(&defaults)),
        Err(PluginDenial::MissingCapability(write_id.clone()))
    );
    grant.revision = 2;
    host.replace_project_grant(grant).unwrap();
    let release_note = host.enable(request(&defaults)).unwrap();
    let wrong_project = RuntimeWorkspaceProjectId::new("other-fixture-project");
    assert!(matches!(
        host.admit_effect(&wrong_project, &release_note.id, &write_id, &write),
        Err(PluginDenial::ScopeMismatch)
    ));
    let malformed = toml::from_str("[draft-output]\ntemplate = 'unknown'").unwrap();
    assert_eq!(
        host.enable(request(&malformed)),
        Err(PluginDenial::InvalidSettings)
    );
    let incompatible = toml::from_str("[brief-input]\nrequire-audience = false").unwrap();
    assert_eq!(
        host.enable(request(&incompatible)),
        Err(PluginDenial::ConflictingSettings)
    );
    assert_eq!(host.enable(request(&defaults)).unwrap(), release_note);
    let effect = host
        .admit_effect(&project, &release_note.id, &write_id, &write)
        .unwrap();
    let effect_id = effect.id();
    assert_eq!(host.disable(&release_note.id).unwrap().pending, 1);
    assert!(matches!(
        host.admit_effect(&project, &release_note.id, &write_id, &write),
        Err(PluginDenial::StaleActivation)
    ));
    effect.finish(EffectOutcome::Denied);
    let disabled = host.disable(&release_note.id).unwrap();
    assert_eq!(disabled.pending, 0);
    assert_eq!(
        disabled.outcomes.get(&effect_id),
        Some(&EffectOutcome::Denied)
    );
    let override_settings = toml::from_str(
        "[brief-input]\nrequire-audience = false\n[draft-output]\ntemplate = 'brief-note'",
    )
    .unwrap();
    let brief_note = host.enable(request(&override_settings)).unwrap();
    assert_ne!(brief_note.settings_digest, release_note.settings_digest);
    assert_ne!(brief_note.id, release_note.id);
    verify_workspace_effects(
        &host,
        &brief_note.id,
        project.clone(),
        &read_id,
        &write_id,
        &release,
    );
    let active = host.enable(request(&defaults)).unwrap();
    let plan_export = unsafe { vm.bind_object_export(marlin_brief_to_draft_propose) }.unwrap();
    let planner = GerbilBriefPlanner::from_linked_export(&vm, plan_export);
    let worktree = tempfile::tempdir().unwrap();
    let root = worktree.path().canonicalize().unwrap();
    let workspace = RuntimeWorkspaceProject::new(project.as_str(), &root);
    std::fs::create_dir(root.join("briefs")).unwrap();
    std::fs::create_dir(root.join("drafts")).unwrap();
    let source = "* Launch brief\n:PROPERTIES:\n:AUDIENCE: Operators\n:END:\n";
    std::fs::write(root.join("briefs/launch.org"), source).unwrap();
    let broker = WorkspaceFileBroker::open(&workspace).unwrap();
    let tokio = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    tokio.block_on(async {
        let (runtime, _events) = TokioAgentRuntime::new(32);
        let mismatched = run(
            &host,
            &active.id,
            &broker,
            &planner,
            (false, BriefTemplate::BriefNote),
            RunId::new("mismatch"),
            &runtime,
        )
        .await;
        assert!(matches!(
            mismatched,
            Err(marlin_agent_core::brief_to_draft::BriefRunError::SettingsMismatch)
        ));
        let evidence = run(
            &host,
            &active.id,
            &broker,
            &planner,
            (true, BriefTemplate::ReleaseNote),
            RunId::new("draft"),
            &runtime,
        )
        .await
        .unwrap_or_else(|error| {
            panic!(
                "graph execution failed: {error:?} {}",
                scheme_diagnostic(&vm)
            )
        });
        assert_eq!(
            evidence.execution.status,
            GraphLoopExecutionStatus::Completed
        );
        assert_eq!(evidence.execution.visited_nodes, ["render", "create"]);
        assert_eq!(evidence.execution.node_receipts.len(), 2);
        let graph = evidence.proposal.proposed_graph.clone();
        let output = evidence.output.as_ref().unwrap().as_ref().unwrap();
        assert_eq!(
            output.1,
            b"* Release note: Launch brief\nAudience: Operators\n"
        );
        assert_eq!(
            std::fs::read(root.join("drafts/launch.org")).unwrap(),
            output.1
        );
        let pending = broker.prepare_create(&host, &active.id, &write_id).unwrap();
        assert_eq!(host.disable(&active.id).unwrap().pending, 0);
        assert!(matches!(
            pending.execute(b"blocked"),
            Err(WorkspaceEffectError::Admission(
                PluginDenial::StaleActivation
            ))
        ));
        let archive = evidence.export(&host).unwrap();
        let expected = evidence_digest(&archive);
        let storage = tempfile::tempdir().unwrap();
        let path = storage.path().join("draft-run.json");
        std::fs::write(&path, archive).unwrap();
        std::fs::write(storage.path().join("digest"), expected).unwrap();
        std::fs::write(storage.path().join("plugin.toml"), MANIFEST).unwrap();
        std::fs::write(storage.path().join("signature"), signature.as_ref()).unwrap();
        drop(broker);
        worktree.close().unwrap();
        let verifier = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "--nocapture",
                "verify_offline_archive",
            ])
            .current_dir(storage.path())
            .env_remove("GERBIL_PATH")
            .env_remove("GERBIL_LOADPATH")
            .env_remove("GERBIL_HOME")
            .env_remove("GERBIL_GXI")
            .env_remove("GERBIL_GSC")
            .env_remove("GAMBOPT")
            .output()
            .unwrap();
        assert!(verifier.status.success(), "{verifier:?}");
        assert!(
            String::from_utf8_lossy(&verifier.stdout).contains("BRIEF-OFFLINE-VERIFIED"),
            "{verifier:?}"
        );
        let archive = std::fs::read(path).unwrap();
        let verified = BriefRunEvidence::verify(&archive, &expected, &release).unwrap();
        assert_eq!(verified.execution().visited_nodes, ["render", "create"]);
        let mut unrelated: serde_json::Value = serde_json::from_slice(&archive).unwrap();
        let bundle = serde_json::to_value(unrelated_release.digest()).unwrap();
        unrelated["effects"]["activation"]["bundle"] = bundle.clone();
        unrelated["effects"]["grant"]["bundle"] = bundle;
        let unrelated = serde_json::to_vec(&unrelated).unwrap();
        assert!(matches!(
            BriefRunEvidence::verify(&unrelated, &evidence_digest(&unrelated), &unrelated_release,),
            Err(WorkflowEvidenceError::Inconsistent)
        ));
        let mut changed: serde_json::Value = serde_json::from_slice(&archive).unwrap();
        changed["execution"]["node_receipts"]
            .as_array_mut()
            .unwrap()
            .pop();
        let changed = serde_json::to_vec(&changed).unwrap();
        assert!(matches!(
            BriefRunEvidence::verify(&changed, &expected, &release),
            Err(WorkflowEvidenceError::Unauthenticated)
        ));
        assert!(matches!(
            BriefRunEvidence::verify(&changed, &evidence_digest(&changed), &release),
            Err(WorkflowEvidenceError::Inconsistent)
        ));
        let mut wrong_settings: serde_json::Value = serde_json::from_slice(&archive).unwrap();
        wrong_settings["render_input"]["release_note"] = false.into();
        let wrong_settings = serde_json::to_vec(&wrong_settings).unwrap();
        assert!(matches!(
            BriefRunEvidence::verify(&wrong_settings, &evidence_digest(&wrong_settings), &release),
            Err(WorkflowEvidenceError::Inconsistent)
        ));
        let mut wrong_draft: serde_json::Value = serde_json::from_slice(&archive).unwrap();
        let replacement = b"* Release note: Different title\nAudience: Operators\n";
        let effect = &mut wrong_draft["effects"]["effects"][1];
        effect["bytes"] = serde_json::to_value(replacement.as_slice()).unwrap();
        effect["outcome"]["Committed"]["output_digest"] =
            serde_json::to_value(evidence_digest(replacement)).unwrap();
        let wrong_draft = serde_json::to_vec(&wrong_draft).unwrap();
        assert!(matches!(
            BriefRunEvidence::verify(&wrong_draft, &evidence_digest(&wrong_draft), &release),
            Err(WorkflowEvidenceError::Inconsistent)
        ));
        assert!(!root.exists());

        let worktree = tempfile::tempdir().unwrap();
        let root = worktree.path().canonicalize().unwrap();
        let workspace = RuntimeWorkspaceProject::new(project.as_str(), &root);
        std::fs::create_dir(root.join("briefs")).unwrap();
        std::fs::create_dir(root.join("drafts")).unwrap();
        std::fs::write(root.join("briefs/launch.org"), source).unwrap();
        let broker = WorkspaceFileBroker::open(&workspace).unwrap();
        let overridden = host.enable(request(&override_settings)).unwrap();
        assert_eq!(overridden.settings_digest, brief_note.settings_digest);
        assert_ne!(overridden.settings_digest, active.settings_digest);
        let alternate = run(
            &host,
            &overridden.id,
            &broker,
            &planner,
            (false, BriefTemplate::BriefNote),
            RunId::new("brief-note"),
            &runtime,
        )
        .await
        .unwrap();
        assert_eq!(
            alternate.execution.status,
            GraphLoopExecutionStatus::Completed
        );
        assert_eq!(alternate.proposal.proposed_graph, graph);
        assert_eq!(
            std::fs::read(root.join("drafts/launch.org")).unwrap(),
            b"* Brief note: Launch brief\nAudience: Operators\n"
        );
        host.disable(&overridden.id).unwrap();
        let alternate = alternate.export(&host).unwrap();
        drop(broker);
        worktree.close().unwrap();
        BriefRunEvidence::verify(&alternate, &evidence_digest(&alternate), &release).unwrap();

        let worktree = tempfile::tempdir().unwrap();
        let root = worktree.path().canonicalize().unwrap();
        let workspace = RuntimeWorkspaceProject::new(project.as_str(), &root);
        std::fs::create_dir(root.join("briefs")).unwrap();
        std::fs::create_dir(root.join("drafts")).unwrap();
        std::fs::write(root.join("briefs/launch.org"), source).unwrap();
        let destination = root.join("drafts/launch.org");
        std::fs::write(&destination, b"existing").unwrap();
        let broker = WorkspaceFileBroker::open(&workspace).unwrap();
        let denied = host.enable(request(&defaults)).unwrap();
        let rejected = run(
            &host,
            &denied.id,
            &broker,
            &planner,
            (true, BriefTemplate::ReleaseNote),
            RunId::new("existing-draft"),
            &runtime,
        )
        .await
        .unwrap();
        assert_eq!(rejected.execution.status, GraphLoopExecutionStatus::Failed);
        assert!(matches!(
            rejected.output,
            Err(WorkspaceEffectError::Effect {
                outcome: EffectOutcome::Denied,
                ..
            })
        ));
        assert_eq!(std::fs::read(&destination).unwrap(), b"existing");
        assert_eq!(host.disable(&denied.id).unwrap().pending, 0);
        let archive = rejected.export(&host).unwrap();
        drop(broker);
        worktree.close().unwrap();
        let verified =
            BriefRunEvidence::verify(&archive, &evidence_digest(&archive), &release).unwrap();
        assert_eq!(
            verified.execution().status,
            GraphLoopExecutionStatus::Failed
        );
        assert_eq!(verified.execution().visited_nodes, ["render"]);
    });
}

#[test]
#[ignore = "subprocess verifier for the linked Brief-to-Draft test"]
fn verify_offline_archive() {
    let publisher = "marlin.fixture".to_owned().try_into().unwrap();
    let attestation = ReleaseAttestation {
        publisher,
        source_revision: [2; 20],
        build_evidence_digest: [4; 32],
    };
    let key = Ed25519KeyPair::from_seed_unchecked(&[3; 32]).unwrap();
    let trust = ReleaseTrust::new(
        [(
            attestation.publisher.clone(),
            key.public_key().as_ref().try_into().unwrap(),
        )]
        .into(),
    );
    let release = trust
        .verify(SignedRelease {
            manifest: &std::fs::read("plugin.toml").unwrap(),
            attestation,
            signature: &std::fs::read("signature").unwrap().try_into().unwrap(),
        })
        .unwrap();
    let archive = std::fs::read("draft-run.json").unwrap();
    let digest = std::fs::read("digest").unwrap().try_into().unwrap();
    let verified = BriefRunEvidence::verify(&archive, &digest, &release).unwrap();
    assert_eq!(
        verified.execution().status,
        GraphLoopExecutionStatus::Completed
    );
    assert_eq!(verified.execution().visited_nodes, ["render", "create"]);
    assert!(!std::path::Path::new("drafts").exists());
    println!("BRIEF-OFFLINE-VERIFIED");
}

fn verify_workspace_effects(
    host: &PluginHost<'_>,
    activation: &marlin_agent_runtime::plugin::ActivationId,
    project: RuntimeWorkspaceProjectId,
    read: &CapabilityId,
    create: &CapabilityId,
    release: &VerifiedRelease,
) {
    use std::{fs, os::unix::fs::symlink};

    let worktree = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let worktree_root = worktree.path().canonicalize().unwrap();
    let project = RuntimeWorkspaceProject::new(project.as_str(), &worktree_root);
    let broker = WorkspaceFileBroker::open(&project).unwrap();
    fs::create_dir(worktree_root.join("briefs")).unwrap();
    fs::write(outside.path().join("outside.org"), "outside").unwrap();
    let brief_path = worktree_root.join("briefs/launch.org");
    symlink(outside.path().join("outside.org"), &brief_path).unwrap();
    let escaped_read = denied(broker.read(host, activation, read));
    fs::remove_file(&brief_path).unwrap();
    let brief = b"* Launch brief\nAudience: Operators\n";
    fs::write(&brief_path, brief).unwrap();
    let input = broker.read(host, activation, read).unwrap();
    assert_eq!(input.value, brief);
    let draft = b"* Launch draft\nAudience: Operators\n";
    let drafts = worktree_root.join("drafts");
    symlink(outside.path(), &drafts).unwrap();
    let escaped_write = denied(broker.create(host, activation, create, draft));
    assert!(!outside.path().join("launch.org").exists());
    fs::remove_file(&drafts).unwrap();
    fs::create_dir(&drafts).unwrap();
    let output = broker.create(host, activation, create, draft).unwrap();
    let existing = denied(broker.create(host, activation, create, b"replacement"));
    assert_eq!(fs::read(drafts.join("launch.org")).unwrap(), draft);
    assert_eq!(fs::read_dir(&drafts).unwrap().count(), 1);
    assert!(matches!(
        host.export_closed_effects(activation, Default::default()),
        Err(WorkflowEvidenceError::Incomplete)
    ));
    let receipt = host.disable(activation).unwrap();
    assert_eq!(receipt.pending, 0);
    assert_eq!(
        receipt.outcomes,
        [
            (escaped_read, EffectOutcome::Denied),
            (escaped_write, EffectOutcome::Denied),
            (
                input.id,
                EffectOutcome::Committed {
                    output_digest: input.digest
                }
            ),
            (
                output.id,
                EffectOutcome::Committed {
                    output_digest: output.digest
                }
            ),
            (existing, EffectOutcome::Denied),
        ]
        .into()
    );
    assert!(matches!(
        broker.create(host, activation, create, draft),
        Err(WorkspaceEffectError::Admission(
            PluginDenial::StaleActivation
        ))
    ));
    assert!(matches!(
        host.export_closed_effects(activation, Default::default()),
        Err(WorkflowEvidenceError::Incomplete)
    ));
    let retained = [(input.id, input.value), (output.id, draft.to_vec())].into();
    let evidence = host.export_closed_effects(activation, retained).unwrap();
    let bytes = evidence.encode().unwrap();
    let digest = evidence_digest(&bytes);
    let archive = tempfile::tempdir().unwrap();
    let path = archive.path().join("workflow-effects.json");
    fs::write(&path, bytes).unwrap();
    drop(broker);
    worktree.close().unwrap();
    assert!(!worktree_root.exists());
    let bytes = fs::read(path).unwrap();
    let verified = ClosedWorkflowEvidence::verify(&bytes, &digest, release).unwrap();
    assert_eq!(verified.effects().len(), 5);
    let mut changed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mut missing_payload = changed.clone();
    missing_payload["effects"][3]["bytes"] = serde_json::Value::Null;
    let missing_payload = serde_json::to_vec(&missing_payload).unwrap();
    assert!(matches!(
        ClosedWorkflowEvidence::verify(
            &missing_payload,
            &evidence_digest(&missing_payload),
            release
        ),
        Err(WorkflowEvidenceError::Incomplete)
    ));
    changed["effects"][3]["bytes"][0] = 0.into();
    let modified = serde_json::to_vec(&changed).unwrap();
    assert!(matches!(
        ClosedWorkflowEvidence::verify(&modified, &digest, release),
        Err(WorkflowEvidenceError::Unauthenticated)
    ));
    assert!(matches!(
        ClosedWorkflowEvidence::verify(&modified, &evidence_digest(&modified), release),
        Err(WorkflowEvidenceError::Inconsistent)
    ));
    changed["effects"].as_array_mut().unwrap().remove(3);
    let removed = serde_json::to_vec(&changed).unwrap();
    assert!(matches!(
        ClosedWorkflowEvidence::verify(&removed, &digest, release),
        Err(WorkflowEvidenceError::Unauthenticated)
    ));
    assert!(matches!(
        ClosedWorkflowEvidence::verify(&removed, &evidence_digest(&removed), release),
        Err(WorkflowEvidenceError::Inconsistent)
    ));
    assert!(!worktree_root.exists());
}

fn evidence_digest(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).into()
}

fn denied<T>(
    result: Result<marlin_agent_runtime::plugin::WorkspaceEffect<T>, WorkspaceEffectError>,
) -> marlin_agent_runtime::plugin::EffectId {
    let Err(WorkspaceEffectError::Effect {
        id,
        outcome: EffectOutcome::Denied,
        ..
    }) = result
    else {
        panic!("workspace effect must be denied");
    };
    id
}

fn scheme_diagnostic(runtime: &GerbilRuntime) -> String {
    unsafe extern "C" fn take_error(_: i64) -> i64 {
        // SAFETY: the adapter runs on the owner and transfers the SDK diagnostic root.
        unsafe { gerbil_scheme_rust_take_call_error_raw() }
    }
    // SAFETY: this consumes the last call diagnostic on the same live VM.
    let bytes = unsafe { runtime.bind_object_export(take_error) }
        .ok()
        .and_then(|export| export.call(&runtime.nil_object().ok()?).into_result().ok())
        .and_then(|value| value.into_bytevector().ok())
        .and_then(|value| value.to_vec().into_result().ok());
    bytes
        .map(|value| String::from_utf8_lossy(&value).into_owned())
        .unwrap_or_default()
}

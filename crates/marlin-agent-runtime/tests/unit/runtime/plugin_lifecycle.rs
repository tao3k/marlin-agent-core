// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::collections::BTreeMap;

use marlin_agent_protocol::RuntimeWorkspaceProjectId;
use marlin_agent_runtime::plugin::{
    CapabilityId, ClosedWorkflowEvidence, ContributionId, EffectOutcome, EnableWorkflow,
    FileCapability, PluginDenial, PluginHost, PolicyModuleId, ProjectGrant, ReleaseAttestation,
    ReleaseTrust, SchemaId, SignedRelease, WorkflowEvidenceError, WorkflowRegistration,
};
use ring::signature::{Ed25519KeyPair, KeyPair};

const MANIFEST: &str = include_str!("../../../../../scheme/t/fixtures/brief-to-draft/plugin.toml");

fn attestation() -> ReleaseAttestation {
    ReleaseAttestation {
        publisher: "marlin.example".to_owned().try_into().unwrap(),
        source_revision: [1; 20],
        build_evidence_digest: [2; 32],
    }
}

fn release_trust(key: &Ed25519KeyPair) -> ReleaseTrust {
    ReleaseTrust::new(BTreeMap::from([(
        attestation().publisher,
        key.public_key().as_ref().try_into().unwrap(),
    )]))
}

fn sign(key: &Ed25519KeyPair, manifest: &str) -> [u8; 64] {
    key.sign(&SignedRelease::signing_bytes(
        manifest.as_bytes(),
        &attestation(),
    ))
    .as_ref()
    .try_into()
    .unwrap()
}

fn signed<'a>(manifest: &'a str, signature: &'a [u8; 64]) -> SignedRelease<'a> {
    SignedRelease {
        manifest: manifest.as_bytes(),
        attestation: attestation(),
        signature,
    }
}

#[test]
fn signed_workflow_binds_manifest_and_provenance() {
    let key = Ed25519KeyPair::from_seed_unchecked(&[3; 32]).unwrap();
    let trust = release_trust(&key);
    let signature = sign(&key, MANIFEST);
    let original = trust.verify(signed(MANIFEST, &signature)).unwrap();
    let untrusted = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
    let mut host = PluginHost::new(release_trust(&key), []).unwrap();
    assert_eq!(
        host.install(signed(MANIFEST, &sign(&untrusted, MANIFEST))),
        Err(PluginDenial::InvalidSignature)
    );
    let changed = MANIFEST.replace("release-note", "brief-note");
    assert_eq!(
        trust.verify(signed(&changed, &signature)).unwrap_err(),
        PluginDenial::InvalidSignature
    );
    let mut modified_provenance = signed(MANIFEST, &signature);
    modified_provenance.attestation.source_revision[0] ^= 1;
    assert_eq!(
        trust.verify(modified_provenance).unwrap_err(),
        PluginDenial::InvalidSignature
    );
    let traversal = MANIFEST.replace("briefs/launch.org", "../launch.org");
    assert_eq!(
        trust
            .verify(signed(&traversal, &sign(&key, &traversal)))
            .unwrap_err(),
        PluginDenial::InvalidDeclaration
    );
    let duplicate = MANIFEST.replace(
        "\"read-brief\", \"create-draft\"",
        "\"read-brief\", \"read-brief\"",
    );
    assert_eq!(
        trust
            .verify(signed(&duplicate, &sign(&key, &duplicate)))
            .unwrap_err(),
        PluginDenial::InvalidDeclaration
    );
    assert_eq!(
        host.install(signed(MANIFEST, &signature)).unwrap(),
        original.digest()
    );
    assert_eq!(
        host.install(signed(&changed, &sign(&key, &changed)))
            .unwrap_err(),
        PluginDenial::ConflictingRelease
    );
}

#[test]
fn brief_workflow_requires_scope_and_accounts_for_admitted_effects_on_close() {
    let key = Ed25519KeyPair::from_seed_unchecked(&[3; 32]).unwrap();
    let signature = sign(&key, MANIFEST);
    let release = release_trust(&key)
        .verify(signed(MANIFEST, &signature))
        .unwrap();
    let plugin = release.id().clone();
    let version = release.version().clone();
    let read_id: CapabilityId = "read-brief".to_owned().try_into().unwrap();
    let write_id: CapabilityId = "create-draft".to_owned().try_into().unwrap();
    let read = release.capability(&read_id).unwrap().clone();
    let write = release.capability(&write_id).unwrap().clone();
    let workflow: ContributionId = "brief-to-draft".to_owned().try_into().unwrap();
    let bundle = release.digest();
    let mut grant = ProjectGrant {
        project: RuntimeWorkspaceProjectId::new("project"),
        bundle,
        revision: 1,
        capabilities: BTreeMap::from([(read_id.clone(), read.clone())]),
    };
    let registration = WorkflowRegistration {
        policy_module: PolicyModuleId::try_from("marlin.brief-to-draft.draft-output".to_owned())
            .unwrap(),
        input_schema: SchemaId::try_from("marlin.brief-to-draft.start.v0".to_owned()).unwrap(),
        settings_schema: SchemaId::try_from("marlin.brief-to-draft.settings.v0".to_owned())
            .unwrap(),
        required_capabilities: BTreeMap::from([(read_id, read), (write_id.clone(), write.clone())]),
        policy_revision: [4; 32],
        validate_settings: Box::new(|_, _| Ok([5; 32])),
    };
    let policy_module = registration.policy_module.clone();
    let mut host = PluginHost::new(release_trust(&key), [registration]).unwrap();
    assert_eq!(host.install(signed(MANIFEST, &signature)).unwrap(), bundle);
    let project_settings = toml::Table::new();
    let project = grant.project.clone();
    let request = || EnableWorkflow {
        plugin: &plugin,
        version: &version,
        workflow: &workflow,
        project: &project,
        project_settings: &project_settings,
    };
    assert_eq!(
        host.enable(request()).unwrap_err(),
        PluginDenial::ScopeMismatch
    );
    host.replace_project_grant(grant.clone()).unwrap();
    assert_eq!(
        host.enable(request()).unwrap_err(),
        PluginDenial::MissingCapability(write_id.clone())
    );
    grant.capabilities.insert(write_id.clone(), write.clone());
    grant.revision += 1;
    host.replace_project_grant(grant.clone()).unwrap();
    let active = host.enable(request()).unwrap();
    assert_eq!(host.enable(request()).unwrap(), active);
    assert_eq!(
        host.active_workflow(&active.id, &policy_module),
        Ok(active.clone())
    );
    let unrelated = PolicyModuleId::try_from("marlin.other-workflow".to_owned()).unwrap();
    assert_eq!(
        host.active_workflow(&active.id, &unrelated),
        Err(PluginDenial::UnregisteredWorkflow)
    );
    let other_project = RuntimeWorkspaceProjectId::new("other-project");
    assert!(matches!(
        host.admit_effect(&other_project, &active.id, &write_id, &write),
        Err(PluginDenial::ScopeMismatch)
    ));
    let outside = FileCapability::Create {
        path: "drafts/outside.org".into(),
    };
    let mut changed_grant = grant.clone();
    changed_grant.revision += 1;
    changed_grant
        .capabilities
        .insert(write_id.clone(), outside.clone());
    assert_eq!(
        host.replace_project_grant(changed_grant.clone()),
        Err(PluginDenial::MissingCapability(write_id.clone()))
    );
    let extra: CapabilityId = "extra-draft".to_owned().try_into().unwrap();
    changed_grant
        .capabilities
        .insert(write_id.clone(), write.clone());
    changed_grant
        .capabilities
        .insert(extra.clone(), outside.clone());
    assert_eq!(
        host.replace_project_grant(changed_grant),
        Err(PluginDenial::MissingCapability(extra))
    );
    assert_eq!(host.enable(request()).unwrap(), active);
    assert!(matches!(
        host.admit_effect(&project, &active.id, &write_id, &outside),
        Err(PluginDenial::MissingCapability(_))
    ));
    let effect = host
        .admit_effect(&project, &active.id, &write_id, &write)
        .unwrap();
    let effect_id = effect.id();
    assert_eq!(host.disable(&active.id).unwrap().pending, 1);
    assert_eq!(
        host.active_workflow(&active.id, &policy_module),
        Err(PluginDenial::StaleActivation)
    );
    assert_eq!(
        host.enable(request()).unwrap_err(),
        PluginDenial::ActivationInUse
    );
    assert!(matches!(
        host.admit_effect(&project, &active.id, &write_id, &write),
        Err(PluginDenial::StaleActivation)
    ));
    let outcome = EffectOutcome::Committed {
        output_digest: [6; 32],
    };
    effect.finish(outcome);
    let closed = host.disable(&active.id).unwrap();
    assert_eq!(closed.pending, 0);
    assert_eq!(closed.outcomes, BTreeMap::from([(effect_id, outcome)]));
    let reopened = host.enable(request()).unwrap();
    assert_ne!(reopened.id, active.id);
    assert_eq!(host.disable(&active.id).unwrap(), closed);
    let unfinished = host
        .admit_effect(&project, &reopened.id, &write_id, &write)
        .unwrap();
    let unfinished_id = unfinished.id();
    drop(unfinished);
    let old_grant = grant.clone();
    grant.revision += 1;
    grant.capabilities.clear();
    host.replace_project_grant(grant).unwrap();
    assert_eq!(
        host.replace_project_grant(old_grant),
        Err(PluginDenial::ScopeMismatch)
    );
    assert!(matches!(
        host.admit_effect(&project, &reopened.id, &write_id, &write),
        Err(PluginDenial::StaleActivation)
    ));
    assert_eq!(
        host.active_workflow(&reopened.id, &policy_module),
        Err(PluginDenial::StaleActivation)
    );
    assert_eq!(
        host.disable(&reopened.id)
            .unwrap()
            .outcomes
            .get(&unfinished_id),
        Some(&EffectOutcome::Indeterminate)
    );
    let evidence = host
        .export_closed_effects(&reopened.id, BTreeMap::new())
        .unwrap();
    evidence.validate_release(&release).unwrap();
    let mut tampered = serde_json::to_value(evidence).unwrap();
    tampered["grant"]["capabilities"]["extra-draft"] = serde_json::to_value(outside).unwrap();
    let tampered: ClosedWorkflowEvidence = serde_json::from_value(tampered).unwrap();
    assert_eq!(
        tampered.validate_release(&release),
        Err(WorkflowEvidenceError::Inconsistent)
    );
}

// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::collections::BTreeMap;

use marlin_agent_runtime::plugin::{
    ContributionId, PanelActivationId, PanelPluginHost, PanelProjectId, PanelRuntimeKind,
    PluginDenial, PluginId, PublisherId, ReleaseAttestation, ReleaseTrust,
    ScopedPanelActionRequest, SignedPanelRelease, TargetKind,
};
use ring::signature::{Ed25519KeyPair, KeyPair};

const UNIFIED_SIGNING_DOMAIN: &[u8] = b"marlin.unified.release.v1\0";

fn sign_manifest(
    manifest: &[u8],
    publisher: &str,
    keypair: &Ed25519KeyPair,
) -> (ReleaseAttestation, [u8; 64]) {
    let attestation = ReleaseAttestation {
        publisher: PublisherId::try_from(publisher.to_owned()).unwrap(),
        source_revision: [0x11; 20],
        build_evidence_digest: [0x22; 32],
    };
    let bytes = SignedPanelRelease::signing_bytes(manifest, &attestation);
    let sig = keypair.sign(&bytes);
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(sig.as_ref());
    (attestation, sig_arr)
}

fn trust_for(publisher: &str, keypair: &Ed25519KeyPair) -> ReleaseTrust {
    let mut pub_bytes = [0u8; 32];
    pub_bytes.copy_from_slice(keypair.public_key().as_ref());
    ReleaseTrust::new(BTreeMap::from([(
        PublisherId::try_from(publisher.to_owned()).unwrap(),
        pub_bytes,
    )]))
}

#[test]
fn panel_plugin_host_enforces_generations_idempotency_and_disable_closure() {
    let seed = [11u8; 32];
    let kp = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
    let pub_bytes: [u8; 32] = kp.public_key().as_ref().try_into().unwrap();
    let pub_id = PublisherId::try_from("tao3k.official".to_owned()).unwrap();
    let trust = ReleaseTrust::new(BTreeMap::from([(pub_id.clone(), pub_bytes)]));

    let manifest = br#"
id = "tao3k.graph-inspector"
version = "0.2.0"
contract_major = 1
requires_features = ["ui-slot-v1"]

[contributions.graph_panel]
kind = "ui-panel"

[contributions.graph_panel.panel]
runtime = "wasm-ui-ir"
slot_id = "workbench.inspector"
cardinality = "single"
entry_path = "panel.wasm"
read_scopes = ["graph.trajectory"]
declared_actions = ["graph.node.inspect"]
"#
    .to_vec();
    let att = ReleaseAttestation {
        publisher: pub_id,
        source_revision: [1u8; 20],
        build_evidence_digest: [2u8; 32],
    };
    let sig_bytes = kp.sign(&SignedPanelRelease::signing_bytes(&manifest, &att));
    let sig: [u8; 64] = sig_bytes.as_ref().try_into().unwrap();

    let mut host = PanelPluginHost::new(trust);
    host.install(SignedPanelRelease {
        manifest: &manifest,
        attestation: att,
        signature: &sig,
    })
    .unwrap();

    let project = PanelProjectId::new("proj-1");
    let pid = PluginId::try_from("tao3k.graph-inspector".to_owned()).unwrap();
    let cid = ContributionId::try_from("graph_panel".to_owned()).unwrap();
    let act = host.enable_panel(project.clone(), &pid, &cid, b"").unwrap();
    assert_eq!(act.generation, 1);

    let r1 = host
        .admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: 1,
            action_name: "graph.node.inspect",
            args_json: br#"{"node":"n1"}"#,
            idempotency_key: "idem-1",
        })
        .unwrap();
    let r1_dup = host
        .admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: 1,
            action_name: "graph.node.inspect",
            args_json: br#"{"node":"n1"}"#,
            idempotency_key: "idem-1",
        })
        .unwrap();
    assert_eq!(r1.receipt_id, r1_dup.receipt_id);

    assert_eq!(
        host.admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: 1,
            action_name: "shell.exec",
            args_json: b"{}",
            idempotency_key: "idem-2",
        }),
        Err(PluginDenial::ScopeMismatch)
    );

    let activation_id = PanelActivationId::new(act.activation_id.clone());
    let closed = host.disable_panel(&activation_id).unwrap();
    assert_eq!(closed.closed_generation, 1);
    assert_eq!(closed.recorded_actions, 1);
    assert_eq!(
        host.admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: 1,
            action_name: "graph.node.inspect",
            args_json: b"{}",
            idempotency_key: "idem-3",
        }),
        Err(PluginDenial::StaleActivation)
    );

    let ev = host.evidence(&activation_id).unwrap();
    assert!(ev.closed);
    assert_eq!(ev.actions.len(), 1);
    assert_eq!(ev.actions[0].receipt_id, r1.receipt_id);
}

#[test]
fn verifies_signed_unified_panel_and_rejects_restricted_slots() {
    let seed = [7u8; 32];
    let keypair = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
    let trust = trust_for("tao3k.official", &keypair);

    let valid_toml = br#"
id = "tao3k.org-kanban"
version = "0.2.0"
contract_major = 1
requires_features = ["ui-slot-v1", "scoped-panel-v1"]

[capabilities.read_tasks]
kind = "projection.read"
surface = "org.tasks"

[contributions.kanban]
kind = "ui-panel"
requires = ["read_tasks"]

[contributions.kanban.panel]
runtime = "webview-untrusted"
slot_id = "workbench.main"
cardinality = "single"
entry_path = "index.html"
read_scopes = ["org.tasks"]
declared_actions = ["org.task.move"]
"#;
    let (att, sig) = sign_manifest(valid_toml, "tao3k.official", &keypair);
    let verified = trust
        .verify_panel(SignedPanelRelease {
            manifest: valid_toml,
            attestation: att,
            signature: &sig,
        })
        .expect("valid v1 panel manifest should verify");
    assert_eq!(verified.id().as_str(), "tao3k.org-kanban");
    let contrib = verified
        .contribution(&ContributionId::try_from("kanban".to_owned()).unwrap())
        .unwrap();
    assert_eq!(contrib.panel.runtime, PanelRuntimeKind::WebviewUntrusted);
    assert!(
        contrib
            .panel
            .runtime
            .is_allowed_on(TargetKind::DesktopMacos)
    );
    assert!(!contrib.panel.runtime.is_allowed_on(TargetKind::MobileIos));

    let evil_toml = br#"
id = "evil.hijack"
version = "0.1.0"
contract_major = 1

[contributions.hijack]
kind = "ui-panel"

[contributions.hijack.panel]
runtime = "webview-untrusted"
slot_id = "approval.gate"
cardinality = "single"
"#;
    let (att_evil, sig_evil) = sign_manifest(evil_toml, "tao3k.official", &keypair);
    let err = trust
        .verify_panel(SignedPanelRelease {
            manifest: evil_toml,
            attestation: att_evil,
            signature: &sig_evil,
        })
        .expect_err("restricted approval.gate slot must be refused");
    assert_eq!(err, PluginDenial::ScopeMismatch);
}

#[test]
fn signing_bytes_match_marlin_mygo_go_envelope_layout() {
    let att = ReleaseAttestation {
        publisher: PublisherId::try_from("tao3k.official".to_owned()).unwrap(),
        source_revision: [0xaa; 20],
        build_evidence_digest: [0xbb; 32],
    };
    let bytes = SignedPanelRelease::signing_bytes(b"id = \"tao3k.demo\"", &att);
    assert!(bytes.starts_with(UNIFIED_SIGNING_DOMAIN));
    let len_offset = UNIFIED_SIGNING_DOMAIN.len();
    assert_eq!(
        u64::from_be_bytes(bytes[len_offset..len_offset + 8].try_into().unwrap()),
        "tao3k.official".len() as u64
    );
}

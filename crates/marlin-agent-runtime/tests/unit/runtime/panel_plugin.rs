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

#[test]
fn sm28_adv_l4_panel_enforces_required_grants_and_revocation_closure() {
    use marlin_agent_runtime::plugin::CapabilityId;

    let seed = [28u8; 32];
    let kp = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
    let trust = trust_for("tao3k.official", &kp);
    let manifest = br#"
id = "tao3k.grant-panel"
version = "1.0.0"
contract_major = 1

[capabilities.read_tasks]
kind = "projection.read"
surface = "org.tasks"

[contributions.board]
kind = "ui-panel"
requires = ["read_tasks"]

[contributions.board.panel]
runtime = "wasm-ui-ir"
slot_id = "workbench.main"
cardinality = "single"
entry_path = "panel.wasm"
read_scopes = ["org.tasks"]
declared_actions = ["org.task.move"]
"#;
    let (att, sig) = sign_manifest(manifest, "tao3k.official", &kp);
    let mut host = PanelPluginHost::new(trust);
    host.install(SignedPanelRelease {
        manifest,
        attestation: att,
        signature: &sig,
    })
    .unwrap();

    let project = PanelProjectId::new("proj-sm28");
    let pid = PluginId::try_from("tao3k.grant-panel".to_owned()).unwrap();
    let cid = ContributionId::try_from("board".to_owned()).unwrap();
    let cap_read = CapabilityId::try_from("read_tasks".to_owned()).unwrap();
    let cap_undeclared = CapabilityId::try_from("ghost_cap".to_owned()).unwrap();

    // 1. Enabling a panel requiring `read_tasks` MUST FAIL CLOSED without an operator grant.
    assert_eq!(
        host.enable_panel(project.clone(), &pid, &cid, b""),
        Err(PluginDenial::MissingCapability(cap_read.clone()))
    );

    // 2. Granting an undeclared capability MUST be refused.
    assert_eq!(
        host.set_project_grants(project.clone(), pid.clone(), [cap_undeclared]),
        Err(PluginDenial::InvalidDeclaration)
    );

    // 3. Granting the required capability admits activation.
    host.set_project_grants(project.clone(), pid.clone(), [cap_read])
        .unwrap();
    let act = host.enable_panel(project.clone(), &pid, &cid, b"").unwrap();
    assert_eq!(act.generation, 1);

    // 4. Revoking the grant MUST atomically close the live generation so subsequent actions fail.
    host.set_project_grants(project.clone(), pid.clone(), std::iter::empty())
        .unwrap();
    assert!(
        host.evidence(&PanelActivationId::new(act.activation_id.clone()))
            .unwrap()
            .closed
    );
    assert_eq!(
        host.admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: act.generation,
            action_name: "org.task.move",
            args_json: b"{}",
            idempotency_key: "idem-after-revoke",
        }),
        Err(PluginDenial::StaleActivation)
    );
}

#[test]
fn sm28_adv_l4_panel_blocks_rollback_publisher_hijack_traversal_and_idempotency_conflict() {
    let seed_a = [29u8; 32];
    let seed_b = [30u8; 32];
    let kp_a = Ed25519KeyPair::from_seed_unchecked(&seed_a).unwrap();
    let kp_b = Ed25519KeyPair::from_seed_unchecked(&seed_b).unwrap();
    let pub_a = PublisherId::try_from("tao3k.official".to_owned()).unwrap();
    let pub_b = PublisherId::try_from("evil.publisher".to_owned()).unwrap();
    let trust = ReleaseTrust::new(BTreeMap::from([
        (pub_a, kp_a.public_key().as_ref().try_into().unwrap()),
        (pub_b, kp_b.public_key().as_ref().try_into().unwrap()),
    ]));

    // 1. Signed manifest with path-traversal / absolute / backslash / NUL entry_path MUST fail verify_panel.
    for bad_path in [
        "../../../etc/passwd",
        "/etc/shadow",
        "assets\\\\..\\\\evil.wasm",
        "./panel.wasm",
        "assets//panel.wasm",
        "C:evil.wasm",
        "",
    ] {
        let bad_manifest = format!(
            r#"
id = "tao3k.traversal"
version = "1.0.0"
contract_major = 1

[contributions.main]
kind = "ui-panel"

[contributions.main.panel]
runtime = "wasm-ui-ir"
slot_id = "workbench.main"
cardinality = "single"
entry_path = "{bad_path}"
declared_actions = ["org.act"]
"#
        );
        let (att, sig) = sign_manifest(bad_manifest.as_bytes(), "tao3k.official", &kp_a);
        let err = trust
            .verify_panel(SignedPanelRelease {
                manifest: bad_manifest.as_bytes(),
                attestation: att,
                signature: &sig,
            })
            .expect_err(&format!("hostile entry_path {bad_path:?} must be rejected"));
        assert_eq!(err, PluginDenial::InvalidDeclaration);
    }

    // 2. Install v2.0.0 by publisher A.
    let v2 = br#"
id = "tao3k.versioned"
version = "2.0.0"
contract_major = 1

[contributions.main]
kind = "ui-panel"

[contributions.main.panel]
runtime = "wasm-ui-ir"
slot_id = "workbench.main"
cardinality = "single"
entry_path = "assets/panel.wasm"
declared_actions = ["org.read", "org.write"]
"#;
    let mut host = PanelPluginHost::new(trust);
    let (att_v2, sig_v2) = sign_manifest(v2, "tao3k.official", &kp_a);
    host.install(SignedPanelRelease {
        manifest: v2,
        attestation: att_v2,
        signature: &sig_v2,
    })
    .unwrap();

    // 3. Signed version downgrade (v1.0.0) by same publisher MUST be refused (anti-rollback).
    let v1 = br#"
id = "tao3k.versioned"
version = "1.0.0"
contract_major = 1

[contributions.main]
kind = "ui-panel"

[contributions.main.panel]
runtime = "wasm-ui-ir"
slot_id = "workbench.main"
cardinality = "single"
entry_path = "assets/panel.wasm"
declared_actions = ["org.read"]
"#;
    let (att_v1, sig_v1) = sign_manifest(v1, "tao3k.official", &kp_a);
    assert_eq!(
        host.install(SignedPanelRelease {
            manifest: v1,
            attestation: att_v1,
            signature: &sig_v1,
        })
        .unwrap_err(),
        PluginDenial::ConflictingRelease
    );

    // 4. Cross-publisher hijack (v3.0.0 signed by publisher B for same plugin_id) MUST be refused.
    let v3_hijack = br#"
id = "tao3k.versioned"
version = "3.0.0"
contract_major = 1

[contributions.main]
kind = "ui-panel"

[contributions.main.panel]
runtime = "wasm-ui-ir"
slot_id = "workbench.main"
cardinality = "single"
entry_path = "assets/panel.wasm"
declared_actions = ["org.read"]
"#;
    let (att_hijack, sig_hijack) = sign_manifest(v3_hijack, "evil.publisher", &kp_b);
    assert_eq!(
        host.install(SignedPanelRelease {
            manifest: v3_hijack,
            attestation: att_hijack,
            signature: &sig_hijack,
        })
        .unwrap_err(),
        PluginDenial::ConflictingRelease
    );

    // 5. Conflicting idempotency replay (same idempotency_key, different action_name or args_json) MUST be refused.
    let project = PanelProjectId::new("proj-idem");
    let pid = PluginId::try_from("tao3k.versioned".to_owned()).unwrap();
    let cid = ContributionId::try_from("main".to_owned()).unwrap();
    let act = host.enable_panel(project.clone(), &pid, &cid, b"").unwrap();
    let first = host
        .admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: act.generation,
            action_name: "org.read",
            args_json: br#"{"id":1}"#,
            idempotency_key: "idem-key-conflict",
        })
        .unwrap();
    // Exact duplicate replays cached receipt.
    let exact = host
        .admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: act.generation,
            action_name: "org.read",
            args_json: br#"{"id":1}"#,
            idempotency_key: "idem-key-conflict",
        })
        .unwrap();
    assert_eq!(first.receipt_id, exact.receipt_id);

    // Same idempotency_key with different args_json MUST fail!
    assert_eq!(
        host.admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: act.generation,
            action_name: "org.read",
            args_json: br#"{"id":2}"#,
            idempotency_key: "idem-key-conflict",
        }),
        Err(PluginDenial::ScopeMismatch)
    );
    // Same idempotency_key with different action_name MUST fail!
    assert_eq!(
        host.admit_scoped_action(ScopedPanelActionRequest {
            project_id: &project,
            plugin_id: &pid,
            contribution_id: &cid,
            generation: act.generation,
            action_name: "org.write",
            args_json: br#"{"id":1}"#,
            idempotency_key: "idem-key-conflict",
        }),
        Err(PluginDenial::ScopeMismatch)
    );

    // 6. Same-publisher upgrade v3.0.0 MUST atomically close prior active generation.
    let (att_v3_ok, sig_v3_ok) = sign_manifest(v3_hijack, "tao3k.official", &kp_a);
    host.install(SignedPanelRelease {
        manifest: v3_hijack,
        attestation: att_v3_ok,
        signature: &sig_v3_ok,
    })
    .unwrap();
    assert!(
        host.evidence(&PanelActivationId::new(act.activation_id))
            .unwrap()
            .closed
    );
}

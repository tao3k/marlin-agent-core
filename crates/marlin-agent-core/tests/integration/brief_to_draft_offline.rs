// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(unix)]

use std::{fs, path::Path};

use marlin_agent_core::{GraphLoopExecutionStatus, brief_to_draft::BriefRunEvidence};
use marlin_agent_runtime::plugin::{ReleaseAttestation, ReleaseTrust, SignedRelease};
use ring::signature::{Ed25519KeyPair, KeyPair};

#[test]
#[ignore = "consumes the archive produced by the linked Brief-to-Draft test"]
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
            manifest: &fs::read("plugin.toml").unwrap(),
            attestation,
            signature: &fs::read("signature").unwrap().try_into().unwrap(),
        })
        .unwrap();
    let archive = fs::read("draft-run.json").unwrap();
    let digest = fs::read("digest").unwrap().try_into().unwrap();
    let verified = BriefRunEvidence::verify(&archive, &digest, &release).unwrap();
    assert_eq!(
        verified.execution().status,
        GraphLoopExecutionStatus::Completed
    );
    assert_eq!(verified.execution().visited_nodes, ["render", "create"]);
    assert!(!Path::new("drafts").exists());
    println!("BRIEF-OFFLINE-VERIFIED");
}

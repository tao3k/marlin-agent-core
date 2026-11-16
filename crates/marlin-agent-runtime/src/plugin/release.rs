// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Verify the publisher and validate workflow-only TOML declarations.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use ring::{digest, signature};
use serde::{Deserialize, Serialize};

use super::PluginDenial;

const MANIFEST_LIMIT: usize = 16 * 1024;
const CAPABILITY_LIMIT: usize = 32;
const WORKFLOW_LIMIT: usize = 16;

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(try_from = "String")]
        pub struct $name(String);

        impl TryFrom<String> for $name {
            type Error = &'static str;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.len() > 128
                    || !value.split('.').all(|part| {
                        part.as_bytes()
                            .first()
                            .is_some_and(u8::is_ascii_alphanumeric)
                            && part.bytes().all(|byte| {
                                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
                            })
                    })
                {
                    return Err("invalid plugin identifier");
                }
                Ok(Self(value))
            }
        }

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

identifier!(PluginId);
identifier!(PublisherId);
identifier!(CapabilityId);
identifier!(ContributionId);
identifier!(PolicyModuleId);
identifier!(SchemaId);

/// The semantic version of a release.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(try_from = "String")]
pub struct PluginVersion(semver::Version);

impl TryFrom<String> for PluginVersion {
    type Error = semver::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        semver::Version::parse(&value).map(Self)
    }
}

/// SHA-256 of the signed workflow-only release envelope.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BundleDigest([u8; 32]);

impl BundleDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// A declared operation on one workspace-relative file.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum FileCapability {
    #[serde(rename = "workspace.read-file")]
    Read { path: String },
    #[serde(rename = "workspace.create-file")]
    Create { path: String },
}

impl FileCapability {
    pub fn path(&self) -> &Path {
        Path::new(match self {
            Self::Read { path } | Self::Create { path } => path,
        })
    }

    fn is_workspace_path(&self) -> bool {
        let path = match self {
            Self::Read { path } | Self::Create { path } => path,
        };
        path.len() <= 1024
            && path.split('/').all(|part| {
                !part.is_empty()
                    && !matches!(part, "." | "..")
                    && !part.contains(['\\', ':'])
                    && !part.chars().any(char::is_control)
            })
    }
}

/// A contribution binding typed schemas, policy, and requested capabilities.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPreset {
    kind: WorkflowKind,
    pub input_schema: SchemaId,
    pub policy_module: PolicyModuleId,
    pub settings_schema: SchemaId,
    pub requires: Vec<CapabilityId>,
    #[serde(default)]
    pub settings: toml::Table,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum WorkflowKind {
    WorkflowPreset,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    id: PluginId,
    version: PluginVersion,
    contract_major: u16,
    requires_features: Vec<String>,
    capabilities: BTreeMap<CapabilityId, FileCapability>,
    contributions: BTreeMap<ContributionId, WorkflowPreset>,
}

impl Manifest {
    fn validate(&self) -> Result<(), PluginDenial> {
        if self.contract_major != 0 || self.requires_features != ["workflow-preset"] {
            return Err(PluginDenial::UnsupportedContract);
        }
        if !self.id.0.contains('.')
            || self.capabilities.len() > CAPABILITY_LIMIT
            || self.contributions.is_empty()
            || self.contributions.len() > WORKFLOW_LIMIT
            || !self
                .capabilities
                .values()
                .all(FileCapability::is_workspace_path)
        {
            return Err(PluginDenial::InvalidDeclaration);
        }
        let declared: BTreeSet<_> = self.capabilities.keys().collect();
        for workflow in self.contributions.values() {
            match workflow.kind {
                WorkflowKind::WorkflowPreset => {}
            }
            let required: BTreeSet<_> = workflow.requires.iter().collect();
            if workflow.requires.len() > CAPABILITY_LIMIT
                || required.len() != workflow.requires.len()
                || !required.is_subset(&declared)
            {
                return Err(PluginDenial::InvalidDeclaration);
            }
        }
        Ok(())
    }
}

/// Release provenance supplied by the publisher and covered by its signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseAttestation {
    pub publisher: PublisherId,
    pub source_revision: [u8; 20],
    pub build_evidence_digest: [u8; 32],
}

/// The signature covers the exact standalone TOML bytes and provenance.
pub struct SignedRelease<'a> {
    pub manifest: &'a [u8],
    pub attestation: ReleaseAttestation,
    pub signature: &'a [u8; 64],
}

impl SignedRelease<'_> {
    pub fn signing_bytes(manifest: &[u8], attestation: &ReleaseAttestation) -> Vec<u8> {
        let mut bytes = b"marlin.workflow-only.release.v0\0".to_vec();
        bytes.extend_from_slice(&(attestation.publisher.0.len() as u64).to_be_bytes());
        bytes.extend_from_slice(attestation.publisher.0.as_bytes());
        bytes.extend_from_slice(&attestation.source_revision);
        bytes.extend_from_slice(&attestation.build_evidence_digest);
        bytes.extend_from_slice(manifest);
        bytes
    }
}

/// Deployment-owned publisher keys. The package cannot add trust roots.
#[derive(Default)]
pub struct ReleaseTrust(BTreeMap<PublisherId, [u8; 32]>);

impl ReleaseTrust {
    pub fn new(keys: BTreeMap<PublisherId, [u8; 32]>) -> Self {
        Self(keys)
    }

    pub fn verify(&self, release: SignedRelease<'_>) -> Result<VerifiedRelease, PluginDenial> {
        if release.manifest.len() > MANIFEST_LIMIT {
            return Err(PluginDenial::InvalidDeclaration);
        }
        let key = self
            .0
            .get(&release.attestation.publisher)
            .ok_or(PluginDenial::UnknownPublisher)?;
        let bytes = SignedRelease::signing_bytes(release.manifest, &release.attestation);
        signature::UnparsedPublicKey::new(&signature::ED25519, key)
            .verify(&bytes, release.signature)
            .map_err(|_| PluginDenial::InvalidSignature)?;
        let text =
            std::str::from_utf8(release.manifest).map_err(|_| PluginDenial::InvalidDeclaration)?;
        let manifest: Manifest =
            toml::from_str(text).map_err(|_| PluginDenial::InvalidDeclaration)?;
        manifest.validate()?;
        let digest = digest::digest(&digest::SHA256, &bytes);
        let mut hash = [0; 32];
        hash.copy_from_slice(digest.as_ref());
        Ok(VerifiedRelease {
            digest: BundleDigest(hash),
            manifest,
            attestation: release.attestation,
        })
    }
}

/// An authenticated release with a validated declaration and bound provenance.
#[derive(Debug)]
pub struct VerifiedRelease {
    digest: BundleDigest,
    manifest: Manifest,
    attestation: ReleaseAttestation,
}

impl VerifiedRelease {
    pub fn digest(&self) -> BundleDigest {
        self.digest
    }
    pub fn id(&self) -> &PluginId {
        &self.manifest.id
    }
    pub fn version(&self) -> &PluginVersion {
        &self.manifest.version
    }
    pub fn attestation(&self) -> &ReleaseAttestation {
        &self.attestation
    }
    pub fn workflow(&self, id: &ContributionId) -> Option<&WorkflowPreset> {
        self.manifest.contributions.get(id)
    }
    pub fn capability(&self, id: &CapabilityId) -> Option<&FileCapability> {
        self.manifest.capabilities.get(id)
    }
}

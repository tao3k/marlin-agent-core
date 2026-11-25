// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rust-owned admission for signed `marlin.unified.release.v1` UI panel bundles
//! consumed by `marlin-mygo` hosts (Desktop GPU, iOS, Android, Remote).
//!
//! Cryptographic signing bytes are byte-for-byte identical to
//! `marlin-mygo/pkg/schema/manifest.go::SigningBytes` under domain
//! `"marlin.unified.release.v1\0"`.

use std::collections::{BTreeMap, BTreeSet};

use ring::{digest, signature};
use serde::{Deserialize, Serialize};

use super::{
    BundleDigest, CapabilityId, ContributionId, PluginDenial, PluginId, PluginVersion,
    ReleaseAttestation, ReleaseTrust, SchemaId,
};

const UNIFIED_SIGNING_DOMAIN: &[u8] = b"marlin.unified.release.v1\0";
const MANIFEST_LIMIT: usize = 16 * 1024;
const CAPABILITY_LIMIT: usize = 32;
const CONTRIBUTION_LIMIT: usize = 16;
const SCOPE_LIST_LIMIT: usize = 32;

/// Host-restricted slots that can NEVER mount untrusted Marketplace plugins.
const RESTRICTED_SLOTS: &[&str] = &["approval.gate", "host.auth", "host.secrets"];

/// Validated dot-separated UI slot identifier (e.g. `"workbench.main"`).
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String")]
pub struct SlotId(String);

impl TryFrom<String> for SlotId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.len() > 128
            || !value.split('.').all(|part| {
                part.as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
            })
        {
            return Err("invalid slot identifier");
        }
        Ok(Self(value))
    }
}

impl SlotId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_restricted(&self) -> bool {
        RESTRICTED_SLOTS.contains(&self.0.as_str())
    }
}

/// Target surface kind mirrored across Scheme `ui-mygo`, Rust, Protobuf, and Go.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetKind {
    DesktopMacos,
    DesktopLinux,
    DesktopWindows,
    MobileIos,
    MobileAndroid,
    RemoteHeadless,
}

/// Sandboxed frontend panel runtime admitted by `marlin-mygo`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PanelRuntimeKind {
    Declarative,
    WebviewUntrusted,
    WasmUiIr,
    MygoJsWasm,
}

impl PanelRuntimeKind {
    /// Enforces platform compliance: iOS/Android App Store policies permit
    /// declarative over-the-air packs only; Desktop permits full sandboxed TS/WASM.
    pub fn is_allowed_on(self, target: TargetKind) -> bool {
        match target {
            TargetKind::DesktopMacos | TargetKind::DesktopLinux | TargetKind::DesktopWindows => {
                true
            }
            TargetKind::MobileIos | TargetKind::MobileAndroid | TargetKind::RemoteHeadless => {
                matches!(self, Self::Declarative)
            }
        }
    }
}

/// Slot mounting cardinality matching `marlin.ui.v1.SlotCardinality`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SlotCardinality {
    Single,
    List,
    Keyed,
}

/// Capability requested by a unified UI panel release.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum PanelCapability {
    #[serde(rename = "projection.read")]
    ProjectionRead { surface: String },
    #[serde(rename = "device.protocol")]
    DeviceProtocol {
        target: TargetKind,
        protocol: String,
        #[serde(default)]
        optional: bool,
    },
}

impl PanelCapability {
    fn is_valid(&self) -> bool {
        match self {
            Self::ProjectionRead { surface } => valid_dot_id(surface),
            Self::DeviceProtocol { protocol, .. } => valid_dot_id(protocol),
        }
    }
}

/// Client presentation panel descriptor embedded in a `ui-panel` contribution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PanelDescriptor {
    pub runtime: PanelRuntimeKind,
    pub slot_id: SlotId,
    pub cardinality: SlotCardinality,
    #[serde(default)]
    pub slot_key: Option<String>,
    #[serde(default)]
    pub entry_path: Option<String>,
    #[serde(default)]
    pub read_scopes: Vec<String>,
    #[serde(default)]
    pub declared_actions: Vec<String>,
}

/// One UI panel or theme pack contribution in `marlin-plugin.toml` (`contract_major = 1`).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PanelContribution {
    pub kind: PanelContributionKind,
    #[serde(default)]
    pub settings_schema: Option<SchemaId>,
    #[serde(default)]
    pub requires: Vec<CapabilityId>,
    pub panel: PanelDescriptor,
}

/// Contribution kind declared by a unified UI panel release.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PanelContributionKind {
    UiPanel,
    ThemePack,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnifiedPanelManifest {
    id: PluginId,
    version: PluginVersion,
    contract_major: u16,
    #[serde(default)]
    requires_features: Vec<String>,
    #[serde(default)]
    capabilities: BTreeMap<CapabilityId, PanelCapability>,
    contributions: BTreeMap<ContributionId, PanelContribution>,
}

impl UnifiedPanelManifest {
    fn validate(&self) -> Result<(), PluginDenial> {
        if self.contract_major != 1 {
            return Err(PluginDenial::UnsupportedContract);
        }
        if !self.id.as_str().contains('.')
            || self.capabilities.len() > CAPABILITY_LIMIT
            || self.contributions.is_empty()
            || self.contributions.len() > CONTRIBUTION_LIMIT
            || !self.capabilities.values().all(PanelCapability::is_valid)
        {
            return Err(PluginDenial::InvalidDeclaration);
        }
        let declared: BTreeSet<_> = self.capabilities.keys().collect();
        for contrib in self.contributions.values() {
            // Host-restricted slots (e.g. approval.gate) are ALWAYS rejected at Rust admission.
            if contrib.panel.slot_id.is_restricted() {
                return Err(PluginDenial::ScopeMismatch);
            }
            if matches!(contrib.panel.cardinality, SlotCardinality::Keyed)
                && contrib.panel.slot_key.as_deref().is_none_or(str::is_empty)
            {
                return Err(PluginDenial::InvalidDeclaration);
            }
            if contrib.panel.read_scopes.len() > SCOPE_LIST_LIMIT
                || contrib.panel.declared_actions.len() > SCOPE_LIST_LIMIT
                || !contrib.panel.read_scopes.iter().all(|s| valid_dot_id(s))
                || !contrib
                    .panel
                    .declared_actions
                    .iter()
                    .all(|a| valid_dot_id(a))
            {
                return Err(PluginDenial::InvalidDeclaration);
            }
            let required: BTreeSet<_> = contrib.requires.iter().collect();
            if contrib.requires.len() > CAPABILITY_LIMIT
                || required.len() != contrib.requires.len()
                || !required.is_subset(&declared)
            {
                return Err(PluginDenial::InvalidDeclaration);
            }
        }
        Ok(())
    }
}

/// Signed v1 unified UI panel release matching `marlin-mygo/pkg/schema.SignedRelease`.
pub struct SignedPanelRelease<'a> {
    pub manifest: &'a [u8],
    pub attestation: ReleaseAttestation,
    pub signature: &'a [u8; 64],
}

impl SignedPanelRelease<'_> {
    /// Canonical domain-separated signing payload matching Go `schema.SigningBytes` byte-for-byte.
    pub fn signing_bytes(manifest: &[u8], attestation: &ReleaseAttestation) -> Vec<u8> {
        let pub_bytes = attestation.publisher.as_str().as_bytes();
        let mut bytes = Vec::with_capacity(
            UNIFIED_SIGNING_DOMAIN.len() + 8 + pub_bytes.len() + 20 + 32 + manifest.len(),
        );
        bytes.extend_from_slice(UNIFIED_SIGNING_DOMAIN);
        bytes.extend_from_slice(&(pub_bytes.len() as u64).to_be_bytes());
        bytes.extend_from_slice(pub_bytes);
        bytes.extend_from_slice(&attestation.source_revision);
        bytes.extend_from_slice(&attestation.build_evidence_digest);
        bytes.extend_from_slice(manifest);
        bytes
    }
}

/// Authenticated and structurally admitted v1 UI panel release.
#[derive(Debug)]
pub struct VerifiedPanelRelease {
    digest: BundleDigest,
    manifest: UnifiedPanelManifest,
    attestation: ReleaseAttestation,
}

impl VerifiedPanelRelease {
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

    pub fn contribution(&self, id: &ContributionId) -> Option<&PanelContribution> {
        self.manifest.contributions.get(id)
    }

    pub fn contributions(&self) -> &BTreeMap<ContributionId, PanelContribution> {
        &self.manifest.contributions
    }

    pub fn requires_features(&self) -> &[String] {
        &self.manifest.requires_features
    }
}

impl ReleaseTrust {
    /// Verifies a `marlin.unified.release.v1` UI panel bundle against deployment-owned Ed25519 keys.
    pub fn verify_panel(
        &self,
        release: SignedPanelRelease<'_>,
    ) -> Result<VerifiedPanelRelease, PluginDenial> {
        if release.manifest.is_empty() || release.manifest.len() > MANIFEST_LIMIT {
            return Err(PluginDenial::InvalidDeclaration);
        }
        let key = self
            .key_for(&release.attestation.publisher)
            .ok_or(PluginDenial::UnknownPublisher)?;
        let bytes = SignedPanelRelease::signing_bytes(release.manifest, &release.attestation);
        signature::UnparsedPublicKey::new(&signature::ED25519, key)
            .verify(&bytes, release.signature)
            .map_err(|_| PluginDenial::InvalidSignature)?;
        let text =
            std::str::from_utf8(release.manifest).map_err(|_| PluginDenial::InvalidDeclaration)?;
        let manifest: UnifiedPanelManifest =
            toml::from_str(text).map_err(|_| PluginDenial::InvalidDeclaration)?;
        manifest.validate()?;
        let digest = digest::digest(&digest::SHA256, &bytes);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(digest.as_ref());
        Ok(VerifiedPanelRelease {
            digest: BundleDigest::from_bytes(hash),
            manifest,
            attestation: release.attestation,
        })
    }
}

fn valid_dot_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.split('.').all(|part| {
            part.as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        })
}

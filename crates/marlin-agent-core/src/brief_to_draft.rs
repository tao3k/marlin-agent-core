// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Register the trusted Brief-to-Draft workflow with the Rust plugin host.

use std::collections::BTreeMap;

use marlin_agent_runtime::plugin::{
    CapabilityId, FileCapability, PluginDenial, PolicyModuleId, WorkflowPreset,
    WorkflowRegistration,
};
use marlin_gerbil_scheme::{
    BriefSettings, BriefSettingsDenial, BriefTemplate, GerbilBriefSettings,
};
#[cfg(unix)]
mod execution;
#[cfg(unix)]
pub use execution::{BriefRun, BriefRunError, BriefRunEvidence, run};

use serde::Deserialize;
use sha2::{Digest, Sha256};

const INPUT_SCHEMA: &str = "marlin.brief-to-draft.start.v0";
const SETTINGS_SCHEMA: &str = "marlin.brief-to-draft.settings.v0";

fn policy_module() -> PolicyModuleId {
    "marlin.brief-to-draft.draft-output"
        .to_owned()
        .try_into()
        .expect("valid Brief-to-Draft policy module ID")
}

fn capability(name: &str) -> CapabilityId {
    name.to_owned()
        .try_into()
        .expect("registered Brief-to-Draft capability")
}

fn file_capabilities() -> BTreeMap<CapabilityId, FileCapability> {
    [
        (
            capability("read-brief"),
            FileCapability::Read {
                path: "briefs/launch.org".into(),
            },
        ),
        (
            capability("create-draft"),
            FileCapability::Create {
                path: "drafts/launch.org".into(),
            },
        ),
    ]
    .into()
}

/// Bind the typed settings export and its host-verified policy revision.
pub fn registration(
    policy: GerbilBriefSettings<'_>,
    policy_revision: [u8; 32],
) -> Result<WorkflowRegistration<'_>, PluginDenial> {
    let malformed = |_| PluginDenial::UnregisteredWorkflow;
    Ok(WorkflowRegistration {
        policy_module: policy_module(),
        input_schema: INPUT_SCHEMA.to_owned().try_into().map_err(malformed)?,
        settings_schema: SETTINGS_SCHEMA.to_owned().try_into().map_err(malformed)?,
        required_capabilities: file_capabilities(),
        policy_revision,
        validate_settings: Box::new(move |preset, project| validate(&policy, preset, project)),
    })
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct Settings {
    brief_input: BriefInput,
    draft_output: DraftOutput,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct BriefInput {
    require_audience: Option<bool>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct DraftOutput {
    template: Option<Template>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Template {
    BriefNote,
    ReleaseNote,
}

fn decode(table: &toml::Table) -> Result<BriefSettings, PluginDenial> {
    let settings: Settings = table
        .clone()
        .try_into()
        .map_err(|_| PluginDenial::InvalidSettings)?;
    Ok(BriefSettings {
        require_audience: settings.brief_input.require_audience,
        template: settings
            .draft_output
            .template
            .map(|template| match template {
                Template::BriefNote => BriefTemplate::BriefNote,
                Template::ReleaseNote => BriefTemplate::ReleaseNote,
            }),
    })
}

fn validate(
    policy: &GerbilBriefSettings<'_>,
    preset: &WorkflowPreset,
    project: &toml::Table,
) -> Result<[u8; 32], PluginDenial> {
    let result = policy
        .validate(decode(&preset.settings)?, decode(project)?)
        .map_err(|_| PluginDenial::PolicyUnavailable)?
        .map_err(|denial| match denial {
            BriefSettingsDenial::InvalidOption => PluginDenial::InvalidSettings,
            BriefSettingsDenial::AudienceRequiredForReleaseNote => {
                PluginDenial::ConflictingSettings
            }
        })?;
    let (Some(audience), Some(template)) = (result.require_audience, result.template) else {
        return Err(PluginDenial::PolicyUnavailable);
    };
    Ok(settings_digest(audience, template))
}

fn settings_digest(audience: bool, template: BriefTemplate) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"marlin.brief-to-draft.settings.v0\0");
    digest.update([
        u8::from(audience),
        u8::from(template == BriefTemplate::ReleaseNote),
    ]);
    digest.finalize().into()
}

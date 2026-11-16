// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Owner-local typed projection of the selected Brief-to-Draft policy.

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierKind, LinkedObjectExport,
    NativeError as SchemeVmError, RootedSchemeObject, SchemeScalar,
};

/// A declared draft template in the Brief-to-Draft settings schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BriefTemplate {
    BriefNote,
    ReleaseNote,
}

impl BriefTemplate {
    fn symbol(self) -> &'static str {
        match self {
            Self::BriefNote => "brief-note",
            Self::ReleaseNote => "release-note",
        }
    }
}

/// Typed settings from one contributor, with omission preserved.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BriefSettings {
    pub require_audience: Option<bool>,
    pub template: Option<BriefTemplate>,
}

/// A policy refusal projected without transferring Scheme objects to Rust callers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BriefSettingsDenial {
    InvalidOption,
    AudienceRequiredForReleaseNote,
}

/// Failure to invoke or decode the registered Scheme procedure.
#[derive(Debug)]
pub enum GerbilBriefSettingsError {
    Scheme(SchemeVmError),
    Projection,
}

impl From<SchemeVmError> for GerbilBriefSettingsError {
    fn from(error: SchemeVmError) -> Self {
        Self::Scheme(error)
    }
}

/// A rooted Brief-to-Draft settings procedure borrowed from the existing VM owner.
pub struct GerbilBriefSettings<'vm> {
    runtime: &'vm GerbilRuntime,
    procedure: LinkedObjectExport<'vm>,
}

impl<'vm> GerbilBriefSettings<'vm> {
    /// Bind the selected package's statically linked settings export.
    pub fn from_linked_export(
        runtime: &'vm GerbilRuntime,
        procedure: LinkedObjectExport<'vm>,
    ) -> Self {
        Self { runtime, procedure }
    }

    /// Apply the trusted Module's defaults, overrides and cross-module contract.
    pub fn validate(
        &self,
        bundle: BriefSettings,
        project: BriefSettings,
    ) -> Result<Result<BriefSettings, BriefSettingsDenial>, GerbilBriefSettingsError> {
        let bundle_audience = self.audience(bundle.require_audience)?;
        let bundle_template = self.template(bundle.template)?;
        let project_audience = self.audience(project.require_audience)?;
        let project_template = self.template(project.template)?;
        let input = self.runtime.collection_from_objects(
            CollectionKind::Vector,
            &[
                &bundle_audience,
                &bundle_template,
                &project_audience,
                &project_template,
            ],
        )?;
        let result = self.procedure.call(&input.into_object()).into_result()?;
        decode_settings(result)
    }

    fn audience(&self, value: Option<bool>) -> Result<RootedSchemeObject<'_>, SchemeVmError> {
        match value {
            Some(value) => self.runtime.scalar_object(SchemeScalar::Boolean(value)),
            None => self.runtime.nil_object(),
        }
    }

    fn template(
        &self,
        value: Option<BriefTemplate>,
    ) -> Result<RootedSchemeObject<'_>, SchemeVmError> {
        match value {
            Some(value) => self
                .runtime
                .intern_identifier(IdentifierKind::Symbol, value.symbol()),
            None => self.runtime.scalar_object(SchemeScalar::Boolean(false)),
        }
    }
}

fn decode_settings(
    result: RootedSchemeObject<'_>,
) -> Result<Result<BriefSettings, BriefSettingsDenial>, GerbilBriefSettingsError> {
    let row = result.into_collection(CollectionKind::Vector)?;
    if row.len() != 3 {
        return Err(GerbilBriefSettingsError::Projection);
    }
    let accepted = row.element(0)?.as_ref().scalar()?;
    let audience = row.element(1)?.as_ref().scalar()?;
    let code = row.element(2)?.as_ref().scalar()?;
    match (accepted, audience, code) {
        (
            SchemeScalar::Boolean(true),
            SchemeScalar::Boolean(audience),
            SchemeScalar::Fixnum(code @ (0 | 1)),
        ) => Ok(Ok(BriefSettings {
            require_audience: Some(audience),
            template: Some(if code == 0 {
                BriefTemplate::BriefNote
            } else {
                BriefTemplate::ReleaseNote
            }),
        })),
        (SchemeScalar::Boolean(false), SchemeScalar::Boolean(false), SchemeScalar::Fixnum(1)) => {
            Ok(Err(BriefSettingsDenial::InvalidOption))
        }
        (SchemeScalar::Boolean(false), SchemeScalar::Boolean(false), SchemeScalar::Fixnum(2)) => {
            Ok(Err(BriefSettingsDenial::AudienceRequiredForReleaseNote))
        }
        _ => Err(GerbilBriefSettingsError::Projection),
    }
}

/// Executor kinds accepted from the Brief-to-Draft Scheme policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BriefNode {
    BriefNote,
    ReleaseNote,
    CreateDraft,
}

/// Source-derived facts and ordered executor kinds selected by Scheme.
#[derive(Debug)]
pub struct BriefProposal {
    pub title: String,
    pub audience: String,
    pub nodes: [BriefNode; 2],
}

/// The owner-local C ABI export for source-backed workflow proposals.
pub struct GerbilBriefPlanner<'vm> {
    runtime: &'vm GerbilRuntime,
    procedure: LinkedObjectExport<'vm>,
}

impl<'vm> GerbilBriefPlanner<'vm> {
    /// Bind the host-selected linked proposal export on its VM owner.
    pub fn from_linked_export(
        runtime: &'vm GerbilRuntime,
        procedure: LinkedObjectExport<'vm>,
    ) -> Self {
        Self { runtime, procedure }
    }

    /// Read the current SDK ABI version for this bound export.
    pub fn abi_version(&self) -> Result<u32, GerbilBriefSettingsError> {
        Ok(self.runtime.abi_version()?)
    }

    /// Project typed Org facts and a bounded graph plan, or a policy refusal.
    pub fn propose(
        &self,
        source: &str,
        audience_required: bool,
        template: BriefTemplate,
    ) -> Result<Option<BriefProposal>, GerbilBriefSettingsError> {
        let source = self.runtime.string_from_utf8(source)?.into_object();
        let audience = self
            .runtime
            .scalar_object(SchemeScalar::Boolean(audience_required))?;
        let template = self
            .runtime
            .intern_identifier(IdentifierKind::Symbol, template.symbol())?;
        let input = self
            .runtime
            .collection_from_objects(CollectionKind::Vector, &[&source, &audience, &template])?;
        let result = self.procedure.call(&input.into_object()).into_result()?;
        if result.as_ref().kind()? == gerbil_scheme::ObjectKind::Boolean {
            return match result.as_ref().scalar()? {
                SchemeScalar::Boolean(false) => Ok(None),
                _ => Err(GerbilBriefSettingsError::Projection),
            };
        }
        let row = result.into_collection(CollectionKind::Vector)?;
        if row.len() != 3 {
            return Err(GerbilBriefSettingsError::Projection);
        }
        let nodes = row.element(2)?.into_collection(CollectionKind::Vector)?;
        if nodes.len() != 2 {
            return Err(GerbilBriefSettingsError::Projection);
        }
        let node = |index| match nodes.element(index)?.as_ref().scalar()? {
            SchemeScalar::Fixnum(0) => Ok(BriefNode::BriefNote),
            SchemeScalar::Fixnum(1) => Ok(BriefNode::ReleaseNote),
            SchemeScalar::Fixnum(2) => Ok(BriefNode::CreateDraft),
            _ => Err(GerbilBriefSettingsError::Projection),
        };
        Ok(Some(BriefProposal {
            title: row.element(0)?.into_string()?.to_string().into_result()?,
            audience: row.element(1)?.into_string()?.to_string().into_result()?,
            nodes: [node(0)?, node(1)?],
        }))
    }
}

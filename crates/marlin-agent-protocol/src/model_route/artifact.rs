//! Artifact route projections for `model_route` admission requests.

use serde::{Deserialize, Serialize};

use super::{
    ModelRouteAdmissionRequest, ModelRouteArtifactRef, ModelRouteEvidenceProfile,
    ModelRouteModality, ModelRoutePrecisionTier, ModelRoutePrivacyTier, ModelRouteRequest,
    ModelRouteSourceKind, ModelRouteTaskKind,
};

/// Typed artifact route projection before runtime admission.
///
/// This keeps local source inspection outside the protocol boundary. Callers
/// compute facts such as hashes and suffixes in their own workspace, then pass
/// those facts here as typed admission evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRouteArtifactProjection {
    pub route_request: ModelRouteRequest,
    pub task_kind: ModelRouteTaskKind,
    pub modality: ModelRouteModality,
    pub source_kind: ModelRouteSourceKind,
    pub precision_tier: ModelRoutePrecisionTier,
    pub privacy_tier: ModelRoutePrivacyTier,
    pub latency_budget_ms: u64,
    pub evidence_profile: ModelRouteEvidenceProfile,
    pub artifact_refs: Vec<ModelRouteArtifactRef>,
}

impl ModelRouteArtifactProjection {
    pub fn new(
        route_request: ModelRouteRequest,
        task_kind: impl Into<ModelRouteTaskKind>,
        modality: impl Into<ModelRouteModality>,
        source_kind: impl Into<ModelRouteSourceKind>,
    ) -> Self {
        Self {
            route_request,
            task_kind: task_kind.into(),
            modality: modality.into(),
            source_kind: source_kind.into(),
            precision_tier: ModelRoutePrecisionTier::new("high"),
            privacy_tier: ModelRoutePrivacyTier::new("private"),
            latency_budget_ms: 60_000,
            evidence_profile: ModelRouteEvidenceProfile::new("default"),
            artifact_refs: Vec::new(),
        }
    }

    pub fn image_document_extract(route_request: ModelRouteRequest) -> Self {
        Self::new(route_request, "attachment-extract", "image", "attachment")
            .with_evidence_profile("image-document-markdown")
    }

    pub fn with_precision_tier(
        mut self,
        precision_tier: impl Into<ModelRoutePrecisionTier>,
    ) -> Self {
        self.precision_tier = precision_tier.into();
        self
    }

    pub fn with_privacy_tier(mut self, privacy_tier: impl Into<ModelRoutePrivacyTier>) -> Self {
        self.privacy_tier = privacy_tier.into();
        self
    }

    pub fn with_latency_budget_ms(mut self, latency_budget_ms: u64) -> Self {
        self.latency_budget_ms = latency_budget_ms;
        self
    }

    pub fn with_evidence_profile(
        mut self,
        evidence_profile: impl Into<ModelRouteEvidenceProfile>,
    ) -> Self {
        self.evidence_profile = evidence_profile.into();
        self
    }

    pub fn with_artifact_ref(mut self, artifact_ref: impl Into<ModelRouteArtifactRef>) -> Self {
        self.artifact_refs.push(artifact_ref.into());
        self
    }

    pub fn with_source_sha256_ref(self, source_sha256: impl AsRef<str>) -> Self {
        self.with_artifact_ref(format!("source-sha256:{}", source_sha256.as_ref()))
    }

    pub fn with_source_suffix_ref(self, source_suffix: impl AsRef<str>) -> Self {
        self.with_artifact_ref(format!("source-suffix:{}", source_suffix.as_ref()))
    }

    pub fn with_backend_profile_ref(self, backend_profile: impl AsRef<str>) -> Self {
        self.with_artifact_ref(format!("backend-profile:{}", backend_profile.as_ref()))
    }

    pub fn into_admission_request(self) -> ModelRouteAdmissionRequest {
        ModelRouteAdmissionRequest::artifact(
            self.route_request,
            self.task_kind,
            self.modality,
            self.source_kind,
        )
        .with_precision_tier(self.precision_tier)
        .with_privacy_tier(self.privacy_tier)
        .with_latency_budget_ms(self.latency_budget_ms)
        .with_evidence_profile(self.evidence_profile)
        .with_artifact_refs(self.artifact_refs)
    }
}

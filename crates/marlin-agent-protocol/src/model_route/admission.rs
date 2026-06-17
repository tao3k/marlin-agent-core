//! Model route admission request and response contracts.

use serde::{Deserialize, Serialize};

use super::{
    ModelRouteArtifactRef, ModelRouteDecision, ModelRouteEvidenceProfile, ModelRouteModality,
    ModelRoutePrecisionTier, ModelRoutePrivacyTier, ModelRouteRequest, ModelRouteSourceKind,
    ModelRouteTaskKind,
};

/// Stable schema id for model route admission responses.
pub const MODEL_ROUTE_ADMISSION_SCHEMA_ID: &str = "marlin.model_route.admission.v1";

/// Routing mode that produced the model route admission decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelRouteAdmissionMode {
    Deterministic,
}

impl ModelRouteAdmissionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
        }
    }
}

/// Intent facts admitted into the model route decision plane.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRouteIntent {
    pub task_kind: ModelRouteTaskKind,
    pub modality: ModelRouteModality,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<ModelRouteSourceKind>,
    pub precision_tier: ModelRoutePrecisionTier,
    pub privacy_tier: ModelRoutePrivacyTier,
    pub latency_budget_ms: u64,
    pub evidence_profile: ModelRouteEvidenceProfile,
    pub artifact_refs: Vec<ModelRouteArtifactRef>,
}

/// User-facing admission request for turning route facts into a model decision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRouteAdmissionRequest {
    pub route_request: ModelRouteRequest,
    pub task_kind: ModelRouteTaskKind,
    pub modality: ModelRouteModality,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<ModelRouteSourceKind>,
    pub precision_tier: ModelRoutePrecisionTier,
    pub privacy_tier: ModelRoutePrivacyTier,
    pub latency_budget_ms: u64,
    pub evidence_profile: ModelRouteEvidenceProfile,
    #[serde(default)]
    pub artifact_refs: Vec<ModelRouteArtifactRef>,
}

impl ModelRouteAdmissionRequest {
    pub fn new(
        route_request: ModelRouteRequest,
        task_kind: impl Into<ModelRouteTaskKind>,
        modality: impl Into<ModelRouteModality>,
    ) -> Self {
        Self {
            route_request,
            task_kind: task_kind.into(),
            modality: modality.into(),
            source_kind: None,
            precision_tier: default_precision_tier(),
            privacy_tier: default_privacy_tier(),
            latency_budget_ms: 60_000,
            evidence_profile: ModelRouteEvidenceProfile::new("default"),
            artifact_refs: Vec::new(),
        }
    }

    pub fn chat(route_request: ModelRouteRequest) -> Self {
        Self::new(
            route_request,
            default_chat_task_kind(),
            default_chat_modality(),
        )
        .with_evidence_profile(default_chat_evidence_profile())
    }

    pub fn artifact(
        route_request: ModelRouteRequest,
        task_kind: impl Into<ModelRouteTaskKind>,
        modality: impl Into<ModelRouteModality>,
        source_kind: impl Into<ModelRouteSourceKind>,
    ) -> Self {
        Self::new(route_request, task_kind, modality).with_source_kind(source_kind)
    }

    pub fn with_source_kind(mut self, source_kind: impl Into<ModelRouteSourceKind>) -> Self {
        self.source_kind = Some(source_kind.into());
        self
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

    pub fn with_artifact_refs<I, R>(mut self, artifact_refs: I) -> Self
    where
        I: IntoIterator<Item = R>,
        R: Into<ModelRouteArtifactRef>,
    {
        self.artifact_refs
            .extend(artifact_refs.into_iter().map(Into::into));
        self
    }

    pub fn intent(&self) -> ModelRouteIntent {
        ModelRouteIntent {
            task_kind: normalized_task_kind(&self.task_kind, default_chat_task_kind()),
            modality: normalized_modality(&self.modality, default_chat_modality()),
            source_kind: self.source_kind.as_ref().map(normalized_source_kind),
            precision_tier: normalized_precision_tier(
                &self.precision_tier,
                default_precision_tier(),
            ),
            privacy_tier: normalized_privacy_tier(&self.privacy_tier, default_privacy_tier()),
            latency_budget_ms: self.latency_budget_ms,
            evidence_profile: normalized_evidence_profile(
                &self.evidence_profile,
                default_chat_evidence_profile(),
            ),
            artifact_refs: self.artifact_refs.clone(),
        }
    }
}

/// Admission response returned after routing has selected a model endpoint.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRouteAdmissionResponse {
    pub schema_id: String,
    pub model_routing_mode: ModelRouteAdmissionMode,
    pub intent: ModelRouteIntent,
    pub decision: ModelRouteDecision,
}

impl ModelRouteAdmissionResponse {
    pub fn deterministic(intent: ModelRouteIntent, decision: ModelRouteDecision) -> Self {
        Self {
            schema_id: MODEL_ROUTE_ADMISSION_SCHEMA_ID.to_owned(),
            model_routing_mode: ModelRouteAdmissionMode::Deterministic,
            intent,
            decision,
        }
    }
}

fn default_chat_task_kind() -> ModelRouteTaskKind {
    ModelRouteTaskKind::new("chat")
}

fn default_chat_modality() -> ModelRouteModality {
    ModelRouteModality::new("text")
}

fn default_precision_tier() -> ModelRoutePrecisionTier {
    ModelRoutePrecisionTier::new("high")
}

fn default_privacy_tier() -> ModelRoutePrivacyTier {
    ModelRoutePrivacyTier::new("private")
}

fn default_chat_evidence_profile() -> ModelRouteEvidenceProfile {
    ModelRouteEvidenceProfile::new("local-knowledge-chat")
}

fn normalized_task_kind(
    value: &ModelRouteTaskKind,
    default: ModelRouteTaskKind,
) -> ModelRouteTaskKind {
    normalized_semantic(value.as_str(), default, ModelRouteTaskKind::new)
}

fn normalized_modality(
    value: &ModelRouteModality,
    default: ModelRouteModality,
) -> ModelRouteModality {
    normalized_semantic(value.as_str(), default, ModelRouteModality::new)
}

fn normalized_source_kind(value: &ModelRouteSourceKind) -> ModelRouteSourceKind {
    normalized_semantic(
        value.as_str(),
        ModelRouteSourceKind::new("artifact"),
        ModelRouteSourceKind::new,
    )
}

fn normalized_precision_tier(
    value: &ModelRoutePrecisionTier,
    default: ModelRoutePrecisionTier,
) -> ModelRoutePrecisionTier {
    normalized_semantic(value.as_str(), default, ModelRoutePrecisionTier::new)
}

fn normalized_privacy_tier(
    value: &ModelRoutePrivacyTier,
    default: ModelRoutePrivacyTier,
) -> ModelRoutePrivacyTier {
    normalized_semantic(value.as_str(), default, ModelRoutePrivacyTier::new)
}

fn normalized_evidence_profile(
    value: &ModelRouteEvidenceProfile,
    default: ModelRouteEvidenceProfile,
) -> ModelRouteEvidenceProfile {
    normalized_semantic(value.as_str(), default, ModelRouteEvidenceProfile::new)
}

fn normalized_semantic<T>(value: &str, default: T, build: impl FnOnce(String) -> T) -> T {
    let normalized = value.trim();
    if normalized.is_empty() {
        default
    } else {
        build(normalized.to_owned())
    }
}

use marlin_agent_protocol::{ModelRouteArtifactProjection, ModelRouteRequest};

#[test]
fn image_document_artifact_projection_builds_typed_admission_request() {
    let request = ModelRouteArtifactProjection::image_document_extract(
        ModelRouteRequest::command(["marlin", "extract"]).with_command_kind("attachment-extract"),
    )
    .with_source_sha256_ref("abc123")
    .with_source_suffix_ref("png")
    .with_backend_profile_ref("hosted-vlm-image")
    .into_admission_request();

    let intent = request.intent();

    assert_eq!(intent.task_kind.as_str(), "attachment-extract");
    assert_eq!(intent.modality.as_str(), "image");
    assert_eq!(
        intent.source_kind.as_ref().expect("source kind").as_str(),
        "attachment"
    );
    assert_eq!(intent.precision_tier.as_str(), "high");
    assert_eq!(intent.privacy_tier.as_str(), "private");
    assert_eq!(intent.latency_budget_ms, 60_000);
    assert_eq!(intent.evidence_profile.as_str(), "image-document-markdown");
    assert_eq!(
        intent
            .artifact_refs
            .iter()
            .map(|reference| reference.as_str())
            .collect::<Vec<_>>(),
        vec![
            "source-sha256:abc123",
            "source-suffix:png",
            "backend-profile:hosted-vlm-image",
        ]
    );
}

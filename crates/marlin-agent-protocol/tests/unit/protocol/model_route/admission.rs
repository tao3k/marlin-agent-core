use marlin_agent_protocol::{ModelRouteAdmissionRequest, ModelRouteRequest, ModelRouteSourceKind};

#[test]
fn model_route_admission_request_projects_artifact_source_intent() {
    let request = ModelRouteAdmissionRequest::artifact(
        ModelRouteRequest::command(["marlin", "extract"]).with_command_kind("attachment-extract"),
        "attachment-extract",
        "image",
        ModelRouteSourceKind::new("attachment"),
    )
    .with_precision_tier("high")
    .with_privacy_tier("private")
    .with_evidence_profile("image-document-markdown")
    .with_latency_budget_ms(60_000)
    .with_artifact_refs([
        "source-sha256:abc123",
        "source-suffix:png",
        "backend-profile:hosted-vlm-image",
    ]);

    let intent = request.intent();

    assert_eq!(intent.task_kind.as_str(), "attachment-extract");
    assert_eq!(intent.modality.as_str(), "image");
    assert_eq!(
        intent.source_kind.as_ref().expect("source kind").as_str(),
        "attachment"
    );
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

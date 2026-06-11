use super::support::loop_graph_artifact;
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCompileRequest, GerbilCompileResponse, GerbilSource,
    GerbilWorkspaceContractFacts,
};
use marlin_org_model::{
    OrgContractRegistry, OrgContractResolutionReport, OrgContractValidationReport,
};

#[test]
fn command_protocol_round_trips_json_contract() {
    let request = GerbilCompileRequest {
        source: GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
        expected: GerbilArtifactKind::LoopGraph,
        contract_facts: Some(GerbilWorkspaceContractFacts {
            registry: OrgContractRegistry::default(),
            resolutions: OrgContractResolutionReport::default(),
            validations: OrgContractValidationReport::default(),
        }),
    };

    let encoded = serde_json::to_string(&request).expect("request should encode as json");
    assert!(encoded.contains("\"expected\":\"LoopGraph\""));

    let decoded: GerbilCompileRequest =
        serde_json::from_str(&encoded).expect("request should decode from json");

    assert_eq!(decoded, request);
    assert!(decoded.contract_facts.is_some());
}

#[test]
fn command_response_carries_typed_artifact() {
    let response = GerbilCompileResponse {
        artifact: loop_graph_artifact("response-loop"),
    };

    assert_eq!(response.artifact.kind(), GerbilArtifactKind::LoopGraph);
}

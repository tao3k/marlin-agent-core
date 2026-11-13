// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_protocol::{ModelEndpoint, ModelEndpointContractError};

#[test]
fn model_endpoint_keeps_real_provider_and_model_names() {
    let endpoint = ModelEndpoint::new("anthropic", "claude-opus-4-8").with_alias("deep-review");

    assert_eq!(endpoint.provider.as_str(), "anthropic");
    assert_eq!(endpoint.model.as_str(), "claude-opus-4-8");
    assert_eq!(
        endpoint.alias.as_ref().expect("alias").as_str(),
        "deep-review"
    );
    assert_eq!(
        endpoint.litellm_model_id().as_str(),
        "anthropic/claude-opus-4-8"
    );
}

#[test]
fn model_endpoint_rejects_provider_and_model_identity_confusion() {
    assert_eq!(
        ModelEndpoint::new("openai", "model-latest").validate_contract(),
        Err(ModelEndpointContractError::OpenAiModelMustBeGpt {
            model: "model-latest".into()
        })
    );
    assert_eq!(
        ModelEndpoint::new("anthropic", "anthropic").validate_contract(),
        Err(ModelEndpointContractError::ModelLooksLikeProvider {
            provider: "anthropic".into(),
            model: "anthropic".into()
        })
    );
    assert_eq!(
        ModelEndpoint::new("claude-opus-4-8", "claude-opus-4-8").validate_contract(),
        Err(ModelEndpointContractError::ProviderLooksLikeModel {
            provider: "claude-opus-4-8".into()
        })
    );
    assert_eq!(
        ModelEndpoint::new("openai", "gpt-5-mini").validate_contract(),
        Ok(())
    );
    assert_eq!(
        ModelEndpoint::new("anthropic", "claude-opus-4-8").validate_contract(),
        Ok(())
    );
}

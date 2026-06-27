//! No-write model probe receipts for live and scripted gateway checks.

use serde::{Deserialize, Serialize};

use marlin_agent_protocol::{
    LiteLlmModelId, ModelEndpoint, ModelGateway, ModelGatewayCompletionOptions, ModelGatewayError,
    ModelGatewayMessageRole, ModelGatewayRequest, ModelName, ModelProviderId,
    system_gateway_message, user_gateway_message,
};

/// Stable schema id for no-write model probe receipts.
pub const MODEL_NO_WRITE_PROBE_RECEIPT_SCHEMA_ID: &str = "marlin.model.no-write-probe-receipt.v1";

macro_rules! define_probe_string_id {
    ($type_name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $type_name(String);

        impl $type_name {
            /// Creates a new typed string value.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns the inner string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $type_name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $type_name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

define_probe_string_id!(
    ModelNoWriteProbeSchemaId,
    "Stable schema id for a no-write model probe receipt."
);
define_probe_string_id!(
    ModelNoWriteProbeMarker,
    "Expected marker requested by a no-write model probe."
);
define_probe_string_id!(
    ModelNoWriteProbeCompletionId,
    "Provider completion id observed by a no-write model probe."
);
define_probe_string_id!(
    ModelNoWriteProbeContentDigest,
    "Digest of assistant content observed by a no-write model probe."
);
define_probe_string_id!(
    ModelNoWriteProbeFailureMessage,
    "Failure message observed by a no-write model probe."
);

/// Number of choices returned by a no-write model probe.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelNoWriteProbeChoiceCount(usize);

impl ModelNoWriteProbeChoiceCount {
    /// Creates a typed choice count.
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the raw count.
    pub fn get(self) -> usize {
        self.0
    }
}

/// Number of assistant content bytes observed by a no-write model probe.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelNoWriteProbeByteCount(usize);

impl ModelNoWriteProbeByteCount {
    /// Creates a typed byte count.
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the raw count.
    pub fn get(self) -> usize {
        self.0
    }
}

/// Terminal status for a no-write model probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelNoWriteProbeStatus {
    Completed,
    MarkerMissing,
    Failed,
}

/// Request for a model probe that must not use tools or filesystem writes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelNoWriteProbeRequest {
    endpoint: ModelEndpoint,
    expected_marker: ModelNoWriteProbeMarker,
    system_prompt: String,
    user_prompt: String,
    options: Option<ModelGatewayCompletionOptions>,
}

impl ModelNoWriteProbeRequest {
    /// Creates a marker probe with conservative default prompts.
    pub fn new(endpoint: ModelEndpoint, expected_marker: impl Into<String>) -> Self {
        let expected_marker = ModelNoWriteProbeMarker::new(expected_marker);
        Self {
            endpoint,
            system_prompt: "Reply only with the requested marker. Do not call tools.".to_owned(),
            user_prompt: format!("Return exactly: {}", expected_marker.as_str()),
            expected_marker,
            options: Some(ModelGatewayCompletionOptions {
                max_tokens: Some(32),
                temperature: Some(0.0),
                ..Default::default()
            }),
        }
    }

    /// Overrides the system prompt.
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = system_prompt.into();
        self
    }

    /// Overrides the user prompt.
    pub fn with_user_prompt(mut self, user_prompt: impl Into<String>) -> Self {
        self.user_prompt = user_prompt.into();
        self
    }

    /// Overrides completion options.
    pub fn with_options(mut self, options: ModelGatewayCompletionOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Returns the endpoint selected for this probe.
    pub fn endpoint(&self) -> &ModelEndpoint {
        &self.endpoint
    }

    /// Returns the expected marker.
    pub fn expected_marker(&self) -> &ModelNoWriteProbeMarker {
        &self.expected_marker
    }

    fn gateway_request(&self) -> ModelGatewayRequest {
        let request = ModelGatewayRequest::new(
            self.endpoint.clone(),
            vec![
                system_gateway_message(self.system_prompt.clone()),
                user_gateway_message(self.user_prompt.clone()),
            ],
        );
        if let Some(options) = self.options.clone() {
            request.with_options(options)
        } else {
            request
        }
    }
}

/// Receipt produced by a no-write model probe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelNoWriteProbeReceipt {
    pub schema_id: ModelNoWriteProbeSchemaId,
    pub provider: ModelProviderId,
    pub model: ModelName,
    pub litellm_model_id: LiteLlmModelId,
    pub expected_marker: ModelNoWriteProbeMarker,
    pub status: ModelNoWriteProbeStatus,
    pub marker_present: bool,
    pub completion_id: Option<ModelNoWriteProbeCompletionId>,
    pub completion_model: Option<ModelName>,
    pub choice_count: ModelNoWriteProbeChoiceCount,
    pub assistant_content_digest: Option<ModelNoWriteProbeContentDigest>,
    pub assistant_content_bytes: ModelNoWriteProbeByteCount,
    pub finish_reason: Option<String>,
    pub failure_message: Option<ModelNoWriteProbeFailureMessage>,
    pub tool_handoff_allowed: bool,
    pub filesystem_write_allowed: bool,
}

impl ModelNoWriteProbeReceipt {
    fn failed(request: &ModelNoWriteProbeRequest, error: ModelGatewayError) -> Self {
        Self {
            schema_id: ModelNoWriteProbeSchemaId::new(MODEL_NO_WRITE_PROBE_RECEIPT_SCHEMA_ID),
            provider: request.endpoint.provider.clone(),
            model: request.endpoint.model.clone(),
            litellm_model_id: request.endpoint.litellm_model_id(),
            expected_marker: request.expected_marker.clone(),
            status: ModelNoWriteProbeStatus::Failed,
            marker_present: false,
            completion_id: None,
            completion_model: None,
            choice_count: ModelNoWriteProbeChoiceCount::new(0),
            assistant_content_digest: None,
            assistant_content_bytes: ModelNoWriteProbeByteCount::new(0),
            finish_reason: None,
            failure_message: Some(ModelNoWriteProbeFailureMessage::new(error.to_string())),
            tool_handoff_allowed: false,
            filesystem_write_allowed: false,
        }
    }
}

/// Runs a provider-neutral no-write probe through a model gateway.
pub async fn run_model_no_write_probe<G>(
    gateway: &G,
    request: ModelNoWriteProbeRequest,
) -> ModelNoWriteProbeReceipt
where
    G: ModelGateway + ?Sized,
{
    let response = match gateway.complete(request.gateway_request()).await {
        Ok(response) => response,
        Err(error) => return ModelNoWriteProbeReceipt::failed(&request, error),
    };
    let first_assistant_choice = response
        .choices
        .iter()
        .find(|choice| choice.message.role == ModelGatewayMessageRole::Assistant);
    let assistant_content = first_assistant_choice
        .map(|choice| choice.message.content.as_str())
        .unwrap_or_default();
    let marker_present = assistant_content.contains(request.expected_marker.as_str());

    ModelNoWriteProbeReceipt {
        schema_id: ModelNoWriteProbeSchemaId::new(MODEL_NO_WRITE_PROBE_RECEIPT_SCHEMA_ID),
        provider: request.endpoint.provider,
        model: request.endpoint.model,
        litellm_model_id: response.model.clone().into(),
        expected_marker: request.expected_marker,
        status: if marker_present {
            ModelNoWriteProbeStatus::Completed
        } else {
            ModelNoWriteProbeStatus::MarkerMissing
        },
        marker_present,
        completion_id: Some(ModelNoWriteProbeCompletionId::new(response.id)),
        completion_model: Some(ModelName::new(response.model)),
        choice_count: ModelNoWriteProbeChoiceCount::new(response.choices.len()),
        assistant_content_digest: (!assistant_content.is_empty())
            .then(|| ModelNoWriteProbeContentDigest::new(stable_text_digest(assistant_content))),
        assistant_content_bytes: ModelNoWriteProbeByteCount::new(assistant_content.len()),
        finish_reason: first_assistant_choice.and_then(|choice| choice.finish_reason.clone()),
        failure_message: None,
        tool_handoff_allowed: false,
        filesystem_write_allowed: false,
    }
}

fn stable_text_digest(text: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;

    let mut value = FNV_OFFSET;
    for byte in text.as_bytes() {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(FNV_PRIME);
    }
    format!("fnv1a64:{value:016x}")
}

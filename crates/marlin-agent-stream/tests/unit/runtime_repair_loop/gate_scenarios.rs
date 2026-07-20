//! Exercises typed live-LLM gate receipts independently from the repair state machine.

use super::{
    LIVE_LLM_GATE_ENV, LIVE_LLM_MODEL_ENV, LIVE_LLM_PROVIDER_API_KEY_ENV, LIVE_LLM_PROVIDER_ENV,
    RUNTIME_LIVE_REPAIR_CASE_ID, RuntimeLiveRepairGateStatus,
    runtime_live_repair_gate_receipt_from_lookup,
};

#[test]
fn live_runtime_repair_gate_receipt_reports_disabled_without_gate() {
    let receipt = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_PROVIDER_ENV => Some("anthropic".to_owned()),
        LIVE_LLM_MODEL_ENV => Some("claude-sonnet-4".to_owned()),
        "ANTHROPIC_API_KEY" => Some("redacted".to_owned()),
        _ => None,
    });

    assert_eq!(receipt.case_id, RUNTIME_LIVE_REPAIR_CASE_ID);
    assert_eq!(receipt.status, RuntimeLiveRepairGateStatus::Disabled);
    assert_eq!(receipt.gate_env, LIVE_LLM_GATE_ENV);
    assert_eq!(
        receipt.denial_reason.as_deref(),
        Some("live LLM gate is disabled")
    );
    assert!(receipt.required_provider_key_envs.is_empty());
    assert!(!receipt.provider_api_key_present);
}

#[test]
fn live_runtime_repair_gate_receipt_reports_missing_provider_and_model() {
    let missing_provider = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_GATE_ENV => Some("1".to_owned()),
        LIVE_LLM_MODEL_ENV => Some("claude-sonnet-4".to_owned()),
        _ => None,
    });
    assert_eq!(
        missing_provider.status,
        RuntimeLiveRepairGateStatus::MissingProvider
    );
    assert_eq!(
        missing_provider.denial_reason.as_deref(),
        Some("MARLIN_LIVE_LLM_PROVIDER is required when live LLM gate is enabled")
    );

    let missing_model = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_GATE_ENV => Some("1".to_owned()),
        LIVE_LLM_PROVIDER_ENV => Some("anthropic".to_owned()),
        _ => None,
    });
    assert_eq!(
        missing_model.status,
        RuntimeLiveRepairGateStatus::MissingModel
    );
    assert_eq!(
        missing_model.denial_reason.as_deref(),
        Some("MARLIN_LIVE_LLM_MODEL is required when live LLM gate is enabled")
    );
}

#[test]
fn live_runtime_repair_gate_receipt_reports_missing_provider_key() {
    let receipt = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_GATE_ENV => Some("1".to_owned()),
        LIVE_LLM_PROVIDER_ENV => Some("anthropic".to_owned()),
        LIVE_LLM_MODEL_ENV => Some("claude-sonnet-4".to_owned()),
        _ => None,
    });

    assert_eq!(
        receipt.status,
        RuntimeLiveRepairGateStatus::MissingProviderKey
    );
    assert_eq!(
        receipt.required_provider_key_envs,
        vec!["ANTHROPIC_API_KEY".to_owned()]
    );
    assert!(!receipt.provider_api_key_present);
    assert_eq!(
        receipt.denial_reason.as_deref(),
        Some("live LLM provider credentials are missing")
    );
}

#[test]
fn live_runtime_repair_gate_receipt_reports_enabled_with_override_key() {
    let receipt = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_GATE_ENV => Some("yes".to_owned()),
        LIVE_LLM_PROVIDER_ENV => Some("deepseek".to_owned()),
        LIVE_LLM_MODEL_ENV => Some("deepseek-chat".to_owned()),
        LIVE_LLM_PROVIDER_API_KEY_ENV => Some("MARLIN_TEST_DEEPSEEK_KEY".to_owned()),
        "MARLIN_TEST_DEEPSEEK_KEY" => Some("redacted".to_owned()),
        _ => None,
    });

    assert_eq!(receipt.status, RuntimeLiveRepairGateStatus::Enabled);
    assert_eq!(receipt.provider.as_deref(), Some("deepseek"));
    assert_eq!(receipt.model.as_deref(), Some("deepseek-chat"));
    assert_eq!(
        receipt.provider_api_key_env_override.as_deref(),
        Some("MARLIN_TEST_DEEPSEEK_KEY")
    );
    assert_eq!(
        receipt.required_provider_key_envs,
        vec!["MARLIN_TEST_DEEPSEEK_KEY".to_owned()]
    );
    assert!(receipt.provider_api_key_present);
    assert!(receipt.denial_reason.is_none());
}

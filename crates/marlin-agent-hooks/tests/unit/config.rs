use std::sync::Arc;

use marlin_agent_hooks::{
    HookConfigurationEnvelope, HookInvocation, HookRegistration, HookRegistry,
};
use marlin_agent_protocol::{
    HookAgentScope, HookEventName, HookHandlerType, HookPolicyExtensionKind, HookPolicyMode,
    HookRegistryUpdateKind, HookRunSummary, HookScope, HookSource, HookTrustStatus,
};
use marlin_agent_runtime::{HookRuntime, RuntimeContext, RuntimeFuture, TokioAgentRuntime};

#[test]
fn hook_configuration_envelope_loads_policy_and_registration_defaults() {
    let config = HookConfigurationEnvelope::from_toml_str(
        r#"
version = "hooks-v1"

[policy]
mode = "EnforceTrusted"

[policy.extension]
kind = "GerbilScheme"
module = "marlin/hooks/policy"
procedure = "decide-hook-policy"

[registration_defaults]
scope = "Thread"
agent_scope = "SubAgent"
source = "Project"
trust = "Trusted"
"#,
    )
    .expect("hook config parses");

    assert_eq!(
        config.version.as_ref().map(|version| version.as_str()),
        Some("hooks-v1")
    );
    assert_eq!(config.policy.mode, HookPolicyMode::EnforceTrusted);
    assert_eq!(
        config.policy.extension.kind,
        HookPolicyExtensionKind::GerbilScheme
    );
    assert_eq!(
        config
            .policy
            .extension
            .module
            .as_ref()
            .map(|module| module.as_str()),
        Some("marlin/hooks/policy")
    );
    assert_eq!(
        config
            .policy
            .extension
            .procedure
            .as_ref()
            .map(|procedure| procedure.as_str()),
        Some("decide-hook-policy")
    );
    assert_eq!(config.registration_defaults.scope, HookScope::Thread);
    assert_eq!(
        config.registration_defaults.agent_scope,
        HookAgentScope::SubAgent
    );
    assert_eq!(config.registration_defaults.source, HookSource::Project);
    assert_eq!(config.registration_defaults.trust, HookTrustStatus::Trusted);
}

#[test]
fn hook_configuration_envelope_reports_reload_receipts() {
    let config = HookConfigurationEnvelope::from_toml_str(
        r#"
version = "hooks-v2"

[policy]
mode = "ObserveOnly"

[registration_defaults]
scope = "Turn"
agent_scope = "CustomerAgent"
source = "User"
trust = "Managed"
"#,
    )
    .expect("hook config parses");
    let registry = HookRegistry::new().with_registration(config.apply_registration_defaults(
        HookRegistration::new(
            "customer",
            HookEventName::SubAgentStart,
            HookHandlerType::Agent,
            Arc::new(ConfigHook),
        ),
    ));

    let receipt = config.reload_receipt(&registry);

    assert_eq!(
        receipt
            .configuration_version
            .as_ref()
            .map(|version| version.as_str()),
        Some("hooks-v2")
    );
    assert_eq!(receipt.policy_mode, HookPolicyMode::ObserveOnly);
    assert_eq!(
        receipt.registration_default_agent_scope,
        HookAgentScope::CustomerAgent
    );
    assert_eq!(receipt.registration_count, 1);
}

#[tokio::test]
async fn hook_configuration_envelope_builds_typed_dispatcher_policy() {
    let config = HookConfigurationEnvelope::from_toml_str(
        r#"
[policy]
mode = "EnforceTrusted"

[registration_defaults]
scope = "Thread"
agent_scope = "SubAgent"
source = "Project"
trust = "Trusted"
"#,
    )
    .expect("hook config parses");
    let registration = config.apply_registration_defaults(HookRegistration::new(
        "configured",
        HookEventName::PreToolUse,
        HookHandlerType::Command,
        Arc::new(ConfigHook),
    ));
    let dispatcher = config.into_dispatcher(HookRegistry::new().with_registration(registration));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = dispatcher
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::SubAgent),
        )
        .await;

    assert_eq!(dispatcher.policy().mode(), &HookPolicyMode::EnforceTrusted);
    assert_eq!(report.policy.allowed_count, 1);
    assert_eq!(report.policy.rejected_count, 0);
    assert_eq!(
        report.policy.decisions[0].agent_scope,
        HookAgentScope::SubAgent
    );
    assert_eq!(report.runs[0].agent_scope, HookAgentScope::SubAgent);
    assert_eq!(report.runs[0].trust, HookTrustStatus::Trusted);
    assert!(report.is_success());
}

#[tokio::test]
async fn hook_configuration_reload_registers_defaults_and_dispatches() {
    let config = HookConfigurationEnvelope::from_toml_str(
        r#"
version = "hooks-v3"

[policy]
mode = "EnforceTrusted"

[policy.extension]
kind = "GerbilScheme"
module = "marlin/hooks/policy"
procedure = "decide-hook-policy"

[registration_defaults]
scope = "Thread"
agent_scope = "SubAgent"
source = "Project"
trust = "Trusted"
"#,
    )
    .expect("hook config parses");
    let mut registry = HookRegistry::new();
    let update = registry.register_with_receipt(
        config.apply_registration_defaults(HookRegistration::new(
            "configured-dynamic",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            Arc::new(ConfigHook),
        )),
        config.version.clone(),
    );

    assert_eq!(update.kind, HookRegistryUpdateKind::Registered);
    assert_eq!(update.agent_scope, HookAgentScope::SubAgent);
    assert_eq!(update.registration_count, 1);
    assert_eq!(
        update
            .configuration_version
            .as_ref()
            .map(|version| version.as_str()),
        Some("hooks-v3")
    );

    let reload = config.reload_receipt(&registry);
    assert_eq!(
        reload
            .configuration_version
            .as_ref()
            .map(|version| version.as_str()),
        Some("hooks-v3")
    );
    assert_eq!(reload.policy_mode, HookPolicyMode::EnforceTrusted);
    assert_eq!(
        reload.policy_extension.kind,
        HookPolicyExtensionKind::GerbilScheme
    );
    assert_eq!(reload.registration_default_scope, HookScope::Thread);
    assert_eq!(
        reload.registration_default_agent_scope,
        HookAgentScope::SubAgent
    );
    assert_eq!(reload.registration_count, 1);

    let dispatcher = config.into_dispatcher(registry);
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let report = dispatcher
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::SubAgent),
        )
        .await;

    assert_eq!(report.policy.mode, HookPolicyMode::EnforceTrusted);
    assert_eq!(
        report.policy.extension.kind,
        HookPolicyExtensionKind::GerbilScheme
    );
    assert_eq!(report.policy.allowed_count, 1);
    assert_eq!(report.policy.rejected_count, 0);
    assert_eq!(report.runs[0].scope, HookScope::Thread);
    assert_eq!(report.runs[0].agent_scope, HookAgentScope::SubAgent);
    assert_eq!(report.runs[0].source, HookSource::Project);
    assert_eq!(report.runs[0].trust, HookTrustStatus::Trusted);
    assert!(report.is_success());
}

#[test]
fn hook_configuration_envelope_reports_toml_errors() {
    let error = HookConfigurationEnvelope::from_toml_str(
        r#"
[policy]
mode = "invalid"
"#,
    )
    .expect_err("invalid policy mode should fail");

    assert!(error.to_string().contains("failed to parse hook `TOML`"));
}

#[derive(Clone, Debug)]
struct ConfigHook;

impl HookRuntime for ConfigHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            HookRunSummary::running(
                "configured-run",
                request.event_name,
                HookHandlerType::Command,
            )
            .completed()
        })
    }
}

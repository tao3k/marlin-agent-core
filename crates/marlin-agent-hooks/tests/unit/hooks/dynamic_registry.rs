use marlin_agent_hooks::{HookRegistration, HookRegistry};
use marlin_agent_protocol::{
    HookAgentScope, HookConfigurationVersion, HookEventName, HookHandlerType,
    HookRegistryUpdateKind, HookRunId,
};

use super::support::SummaryHook;

#[test]
fn registry_dynamic_updates_return_typed_receipts() {
    let mut registry = HookRegistry::new();
    let registered = registry.register_with_receipt(
        HookRegistration::new(
            "customer-hook",
            HookEventName::SubAgentStart,
            HookHandlerType::Agent,
            std::sync::Arc::new(SummaryHook::new("customer-run")),
        )
        .with_agent_scope(HookAgentScope::CustomerAgent),
        Some(HookConfigurationVersion::new("hooks-v1")),
    );

    assert_eq!(registered.kind, HookRegistryUpdateKind::Registered);
    assert_eq!(registered.agent_scope, HookAgentScope::CustomerAgent);
    assert_eq!(registered.registration_count, 1);
    assert_eq!(
        registered
            .configuration_version
            .as_ref()
            .map(|version| version.as_str()),
        Some("hooks-v1")
    );

    let disabled = registry
        .set_enabled(
            &HookRunId::new("customer-hook"),
            false,
            Some(HookConfigurationVersion::new("hooks-v2")),
        )
        .expect("hook can be disabled");
    assert_eq!(disabled.kind, HookRegistryUpdateKind::Disabled);
    assert!(!disabled.enabled);

    let removed = registry
        .unregister(
            &HookRunId::new("customer-hook"),
            Some(HookConfigurationVersion::new("hooks-v3")),
        )
        .expect("hook can be unregistered");
    assert_eq!(removed.kind, HookRegistryUpdateKind::Unregistered);
    assert_eq!(removed.registration_count, 0);
    assert!(registry.registrations().is_empty());
}

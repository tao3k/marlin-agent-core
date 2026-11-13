// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_telemetry::{
    TracingSubscriberConfig, TracingSubscriberConfigError, TracingSubscriberFormat,
    TracingSubscriberScope,
};

#[test]
fn tracing_subscriber_config_validates_receipt() {
    let receipt = TracingSubscriberConfig::new()
        .with_env_filter("marlin_agent_runtime=debug,info")
        .with_format(TracingSubscriberFormat::Json)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .validate()
        .expect("subscriber config should validate");

    assert_eq!(receipt.env_filter(), "marlin_agent_runtime=debug,info");
    assert_eq!(receipt.format(), TracingSubscriberFormat::Json);
    assert_eq!(receipt.format().as_str(), "json");
    assert!(receipt.target_enabled());
    assert!(receipt.thread_ids_enabled());
    assert_eq!(receipt.scope(), TracingSubscriberScope::Validated);
}

#[test]
fn tracing_subscriber_config_rejects_invalid_filter() {
    let error = TracingSubscriberConfig::new()
        .with_env_filter("=info")
        .validate()
        .expect_err("invalid filter should be rejected");

    assert!(matches!(
        error,
        TracingSubscriberConfigError::InvalidEnvFilter { .. }
    ));
}

#[test]
fn tracing_subscriber_uses_scoped_default_without_global_install() {
    let scoped = TracingSubscriberConfig::new()
        .with_env_filter("runtime.test=info,info")
        .with_format(TracingSubscriberFormat::Compact)
        .with_scoped_default(|receipt| {
            tracing::info!(
                target: "runtime.test",
                runtime_kind = "provider",
                "scoped subscriber observed"
            );
            (receipt.scope(), receipt.format())
        })
        .expect("scoped subscriber should be installed for closure");

    assert_eq!(
        scoped,
        (
            TracingSubscriberScope::ScopedDefault,
            TracingSubscriberFormat::Compact,
        )
    );
    assert_eq!(
        TracingSubscriberScope::ScopedDefault.as_str(),
        "scoped-default"
    );
}

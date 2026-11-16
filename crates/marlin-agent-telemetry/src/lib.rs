// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Application-owned tracing subscribers and OpenTelemetry export.

#[cfg(target_os = "macos")]
mod host_observer;
#[cfg(target_os = "macos")]
pub mod host_watchdog;
mod otlp;
#[cfg(target_os = "macos")]
mod process_memory;
mod subscriber;

#[cfg(target_os = "macos")]
pub use host_observer::{run_host_observer_if_requested, start_host_observer};
pub use otlp::{
    OpenTelemetry, OpenTelemetryConfigError, OpenTelemetryShutdownError, TelemetryServiceName,
    TraceTarget,
};
#[cfg(target_os = "macos")]
pub use process_memory::MacOsProcessMemorySampler;
pub use subscriber::{
    TracingSubscriberConfig, TracingSubscriberConfigError, TracingSubscriberFormat,
    TracingSubscriberReceipt, TracingSubscriberScope,
};

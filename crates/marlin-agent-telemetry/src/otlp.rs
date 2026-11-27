// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! OTLP trace export and provider lifecycle.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    error::{OTelSdkError, OTelSdkResult},
    trace::{SdkTracerProvider, SpanData, SpanExporter},
};
use tracing::{Dispatch, dispatcher::SetGlobalDefaultError};
use tracing_subscriber::{Layer, filter::filter_fn, layer::SubscriberExt};

/// OpenTelemetry resource service name.
pub struct TelemetryServiceName(&'static str);

impl TelemetryServiceName {
    /// Select the service name reported by the trace resource.
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }
}

/// The tracing metadata target admitted to the exporter.
pub struct TraceTarget(&'static str);

impl TraceTarget {
    /// Select the exact target whose spans are eligible for export.
    pub const fn new(target: &'static str) -> Self {
        Self(target)
    }
}

/// Owns the trace provider and its bounded background export queue.
#[must_use = "retain the provider until the application can shut down telemetry"]
pub struct OpenTelemetry {
    provider: SdkTracerProvider,
    dispatch: Dispatch,
    failed_batches: Arc<AtomicU64>,
}

/// Invalid OTLP configuration or exporter construction failure.
#[derive(Debug, thiserror::Error)]
pub enum OpenTelemetryConfigError {
    #[error("expected an absolute HTTP or HTTPS traces endpoint without user information")]
    InvalidEndpoint,
    #[error("failed to configure the OTLP/HTTP exporter: {0}")]
    Exporter(#[from] opentelemetry_otlp::ExporterBuildError),
}

/// Failure to stop the provider or export one or more of its batches.
#[derive(Debug, thiserror::Error)]
pub enum OpenTelemetryShutdownError {
    #[error("failed to stop the trace provider: {0}")]
    Provider(#[from] OTelSdkError),
    #[error("failed to export {0} OTLP trace batch(es)")]
    ExportFailed(u64),
}

#[derive(Debug)]
struct ObservedExporter {
    inner: opentelemetry_otlp::SpanExporter,
    failed_batches: Arc<AtomicU64>,
}

impl SpanExporter for ObservedExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let result = self.inner.export(batch).await;
        if result.is_err() {
            self.failed_batches.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    fn shutdown(&mut self) -> OTelSdkResult {
        self.inner.shutdown()
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.inner.set_resource(resource);
    }
}

const MAX_OTLP_ENDPOINT_BYTES: usize = 2048;
const MAX_TELEMETRY_LABEL_BYTES: usize = 256;

fn is_loopback_otlp_host(host: &str) -> bool {
    let trimmed = host.trim_start_matches('[').trim_end_matches(']');
    if trimmed.eq_ignore_ascii_case("localhost") {
        return true;
    }
    trimmed
        .parse::<std::net::IpAddr>()
        .is_ok_and(|ip| ip.is_loopback())
}

fn validate_telemetry_label(label: &str) -> Result<(), OpenTelemetryConfigError> {
    if label.trim().is_empty()
        || label.len() > MAX_TELEMETRY_LABEL_BYTES
        || label.chars().any(char::is_control)
    {
        return Err(OpenTelemetryConfigError::InvalidEndpoint);
    }
    Ok(())
}

fn validate_otlp_endpoint(traces_endpoint: &str) -> Result<(), OpenTelemetryConfigError> {
    if traces_endpoint.is_empty()
        || traces_endpoint.len() > MAX_OTLP_ENDPOINT_BYTES
        || traces_endpoint
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '?' | '#' | '@'))
        || traces_endpoint.to_ascii_lowercase().contains("%40")
    {
        return Err(OpenTelemetryConfigError::InvalidEndpoint);
    }
    let endpoint = traces_endpoint
        .parse::<http::Uri>()
        .map_err(|_| OpenTelemetryConfigError::InvalidEndpoint)?;
    let Some(authority) = endpoint.authority() else {
        return Err(OpenTelemetryConfigError::InvalidEndpoint);
    };
    let host = authority.host();
    if host.is_empty() || authority.as_str().contains('@') {
        return Err(OpenTelemetryConfigError::InvalidEndpoint);
    }
    match endpoint.scheme_str() {
        Some("https") => Ok(()),
        Some("http") if is_loopback_otlp_host(host) => Ok(()),
        _ => Err(OpenTelemetryConfigError::InvalidEndpoint),
    }
}

impl OpenTelemetry {
    /// Connect an OTLP/HTTP protobuf traces endpoint using the SDK's blocking client.
    pub fn otlp_http(
        service_name: TelemetryServiceName,
        span_target: TraceTarget,
        traces_endpoint: &str,
    ) -> Result<Self, OpenTelemetryConfigError> {
        validate_telemetry_label(service_name.0)?;
        validate_telemetry_label(span_target.0)?;
        validate_otlp_endpoint(traces_endpoint)?;
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(traces_endpoint)
            .with_timeout(Duration::from_secs(3))
            .build()?;
        Ok(Self::with_exporter(service_name, span_target, exporter))
    }

    fn with_exporter(
        service_name: TelemetryServiceName,
        span_target: TraceTarget,
        exporter: opentelemetry_otlp::SpanExporter,
    ) -> Self {
        let failed_batches = Arc::new(AtomicU64::new(0));
        let provider = SdkTracerProvider::builder()
            .with_resource(
                Resource::builder_empty()
                    .with_service_name(service_name.0)
                    .build(),
            )
            .with_batch_exporter(ObservedExporter {
                inner: exporter,
                failed_batches: Arc::clone(&failed_batches),
            })
            .build();
        let layer = tracing_opentelemetry::layer()
            .with_tracer(provider.tracer("marlin-agent-telemetry"))
            .with_location(false)
            .with_threads(false)
            .with_filter(filter_fn(move |metadata| {
                metadata.is_span() && metadata.target() == span_target.0
            }));
        let dispatch = Dispatch::new(tracing_subscriber::registry().with(layer));
        Self {
            provider,
            dispatch,
            failed_batches,
        }
    }

    /// Borrow the dispatcher for a scoped subscriber.
    pub fn dispatch(&self) -> &Dispatch {
        &self.dispatch
    }

    /// Install once at the application entry point before starting worker threads.
    pub fn install_global(&self) -> Result<(), SetGlobalDefaultError> {
        tracing::dispatcher::set_global_default(self.dispatch.clone())
    }

    /// Drain accepted spans and stop the SDK exporter after application work ends.
    pub fn shutdown(self) -> Result<(), OpenTelemetryShutdownError> {
        self.provider
            .shutdown_with_timeout(Duration::from_secs(5))?;
        // The SDK confirms the worker stopped even when an export returned an error.
        let failures = self.failed_batches.load(Ordering::Relaxed);
        if failures == 0 {
            Ok(())
        } else {
            Err(OpenTelemetryShutdownError::ExportFailed(failures))
        }
    }
}

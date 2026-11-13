// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use marlin_agent_telemetry::{
    OpenTelemetry, OpenTelemetryConfigError, OpenTelemetryShutdownError, TelemetryServiceName,
    TraceTarget,
};
use opentelemetry_proto::tonic::{
    collector::trace::v1::ExportTraceServiceRequest, common::v1::any_value::Value,
};
use prost::Message;

#[test]
fn rejects_implicit_and_non_http_endpoints() {
    for endpoint in [
        "",
        "/v1/traces",
        "ftp://localhost/v1/traces",
        "http://user@localhost/v1/traces",
    ] {
        assert!(matches!(
            OpenTelemetry::otlp_http(
                TelemetryServiceName::new("marlin-test"),
                TraceTarget::new("marlin.runtime"),
                endpoint,
            ),
            Err(OpenTelemetryConfigError::InvalidEndpoint)
        ));
    }
}

fn start_collector(response: &'static [u8]) -> (String, JoinHandle<ExportTraceServiceRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/v1/traces", listener.local_addr().unwrap());
    let collector = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "missing OTLP request");
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("accept OTLP request: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = BufReader::new(&mut stream);
        let mut line = String::new();
        request.read_line(&mut line).unwrap();
        assert_eq!(line.trim_end(), "POST /v1/traces HTTP/1.1");
        let mut length = None;
        let mut protobuf = false;
        loop {
            line.clear();
            assert_ne!(request.read_line(&mut line).unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = Some(value.trim().parse::<usize>().unwrap());
                }
                if name.eq_ignore_ascii_case("content-type") {
                    protobuf = value.trim() == "application/x-protobuf";
                }
            }
        }
        assert!(protobuf);
        let mut payload = vec![0; length.expect("content length")];
        request.read_exact(&mut payload).unwrap();
        stream.write_all(response).unwrap();
        ExportTraceServiceRequest::decode(payload.as_slice()).unwrap()
    });
    (endpoint, collector)
}

#[test]
fn shutdown_exports_only_selected_spans_as_otlp_http() {
    let (endpoint, collector) = start_collector(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");

    let telemetry = OpenTelemetry::otlp_http(
        TelemetryServiceName::new("marlin-test"),
        TraceTarget::new("marlin.runtime"),
        &endpoint,
    )
    .unwrap();
    tracing::dispatcher::with_default(telemetry.dispatch(), || {
        let span =
            tracing::info_span!(target: "marlin.runtime", "runtime.task", runtime_kind = "tool");
        span.in_scope(|| tracing::info!(target: "marlin.runtime", "local event"));
        drop(tracing::info_span!(target: "runtime.process", "process", pid = 123));
    });
    telemetry.shutdown().unwrap();

    let request = collector.join().unwrap();
    let [resource] = request.resource_spans.as_slice() else {
        panic!("expected one service")
    };
    let attribute = |key: &str, value: &str| opentelemetry_proto::tonic::common::v1::KeyValue {
        key: key.to_owned(),
        value: Some(opentelemetry_proto::tonic::common::v1::AnyValue {
            value: Some(Value::StringValue(value.to_owned())),
        }),
    };
    assert!(
        resource
            .resource
            .as_ref()
            .unwrap()
            .attributes
            .contains(&attribute("service.name", "marlin-test"))
    );
    let [scope] = resource.scope_spans.as_slice() else {
        panic!("expected one instrumentation scope")
    };
    let [span] = scope.spans.as_slice() else {
        panic!("expected only the selected Runtime span")
    };
    assert_eq!(span.name, "runtime.task");
    assert!(span.attributes.contains(&attribute("runtime_kind", "tool")));
    assert!(
        span.attributes
            .iter()
            .all(|field| !field.key.starts_with("code.") && !field.key.starts_with("thread."))
    );
    assert!(span.events.is_empty());
}

#[test]
fn rate_limited_collector_does_not_extend_shutdown_to_retry_after() {
    let (endpoint, collector) = start_collector(
        b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 60\r\nContent-Length: 0\r\n\r\n",
    );
    let telemetry = OpenTelemetry::otlp_http(
        TelemetryServiceName::new("marlin-test"),
        TraceTarget::new("marlin.runtime"),
        &endpoint,
    )
    .unwrap();
    tracing::dispatcher::with_default(telemetry.dispatch(), || {
        drop(tracing::info_span!(target: "marlin.runtime", "runtime.task"));
    });

    let shutdown = Instant::now();
    let error = telemetry
        .shutdown()
        .expect_err("collector refused the final batch");
    assert!(matches!(error, OpenTelemetryShutdownError::ExportFailed(1)));
    assert!(shutdown.elapsed() < Duration::from_secs(6));
    assert_eq!(collector.join().unwrap().resource_spans.len(), 1);
}

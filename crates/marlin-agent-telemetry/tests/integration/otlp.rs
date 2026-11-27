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

#[cfg(unix)]
#[test]
fn managed_process_lifecycle_and_restart_reconciliation_export_otlp_spans() {
    use marlin_agent_runtime::{
        AsyncManagedChildProcess, FileProcessLedgerStore, ManagedChildProcessSpec,
        ProcessGracefulShutdownPolicy, RuntimeHostRunId, TokioAgentRuntime,
        TokioAgentRuntimeBuildRequest,
        observability::{SPAN_RUNTIME_PROCESS, SPAN_RUNTIME_PROCESS_RECONCILE, TARGET_RUNTIME_SPAN},
        process::RuntimeProcessKind,
    };
    use std::sync::Arc;

    let (endpoint, collector) = start_collector(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
    let telemetry = OpenTelemetry::otlp_http(
        TelemetryServiceName::new("marlin-process-test"),
        TraceTarget::new(TARGET_RUNTIME_SPAN),
        &endpoint,
    )
    .unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let ledger = Arc::new(FileProcessLedgerStore::new(
        tmp.path().join("managed-processes.json"),
    ));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let leaked_pid = tracing::dispatcher::with_default(telemetry.dispatch(), || {
        rt.block_on(async {
            let (runtime1, _events1) = TokioAgentRuntime::from_build_request(
                TokioAgentRuntimeBuildRequest::runtime_root(8)
                    .with_host_run_id(RuntimeHostRunId::new("otel-boot-1"))
                    .with_process_ledger(ledger.clone()),
            );

            // 1. Managed process with normal clean exit emits `runtime.process` span.
            let mut ok_cmd = tokio::process::Command::new("/usr/bin/true");
            ok_cmd.stdout(std::process::Stdio::null());
            let ok_child = AsyncManagedChildProcess::spawn_with_spec(
                &runtime1.context(),
                ok_cmd,
                ManagedChildProcessSpec::new(RuntimeProcessKind::Tool, "otel-clean-tool"),
            )
            .await
            .unwrap();
            assert!(ok_child.wait().await.unwrap().success());

            // 2. Spawn another process and leak it to simulate crash, then reconcile on boot #2.
            let mut leaked_cmd = tokio::process::Command::new("/bin/sleep");
            leaked_cmd.arg("30");
            let leaked = AsyncManagedChildProcess::spawn_with_spec(
                &runtime1.context(),
                leaked_cmd,
                ManagedChildProcessSpec::new(RuntimeProcessKind::Provider, "otel-leaked-provider"),
            )
            .await
            .unwrap();
            let pid = leaked.pid();
            std::mem::forget(leaked);

            let (runtime2, _events2) = TokioAgentRuntime::from_build_request(
                TokioAgentRuntimeBuildRequest::runtime_root(8)
                    .with_host_run_id(RuntimeHostRunId::new("otel-boot-2"))
                    .with_process_ledger(ledger.clone()),
            );
            let report = runtime2.reconcile_persisted_processes().unwrap();
            assert_eq!(report.survived.len(), 1);
            let reap = runtime2
                .process_registry()
                .reap_survived_process(
                    pid,
                    &ProcessGracefulShutdownPolicy::new(Duration::from_secs(2)),
                    2000,
                )
                .unwrap()
                .unwrap();
            assert!(reap.confirmed_absent);
            #[cfg(unix)]
            // SAFETY: reap the forgotten test child so it does not linger as a zombie.
            unsafe {
                let _ = libc::waitpid(pid as libc::pid_t, std::ptr::null_mut(), 0);
            }
            pid
        })
    });

    telemetry.shutdown().unwrap();
    let request = collector.join().unwrap();
    let spans: Vec<_> = request
        .resource_spans
        .iter()
        .flat_map(|rs| rs.scope_spans.iter())
        .flat_map(|ss| ss.spans.iter())
        .collect();

    let has_str_attr = |span: &opentelemetry_proto::tonic::trace::v1::Span, key: &str, val: &str| {
        span.attributes.iter().any(|kv| {
            kv.key == key
                && matches!(
                    kv.value.as_ref().and_then(|v| v.value.as_ref()),
                    Some(Value::StringValue(s)) if s == val
                )
        })
    };
    let has_bool_attr = |span: &opentelemetry_proto::tonic::trace::v1::Span, key: &str, val: bool| {
        span.attributes.iter().any(|kv| {
            kv.key == key
                && matches!(
                    kv.value.as_ref().and_then(|v| v.value.as_ref()),
                    Some(Value::BoolValue(b)) if *b == val
                )
        })
    };

    let process_span = spans
        .iter()
        .find(|s| s.name == SPAN_RUNTIME_PROCESS && has_str_attr(s, "process_handle", "otel-clean-tool"))
        .expect("OTLP payload must include runtime.process span");
    assert!(has_str_attr(process_span, "termination_stage", "exited_cleanly"));
    assert!(has_bool_attr(process_span, "escalated_to_sigkill", false));

    let reconcile_span = spans
        .iter()
        .find(|s| s.name == SPAN_RUNTIME_PROCESS_RECONCILE)
        .expect("OTLP payload must include runtime.process.reconcile span");
    assert!(has_bool_attr(reconcile_span, "orphan_alive", true));
    assert!(has_str_attr(reconcile_span, "termination_stage", "survived_restart"));
    assert!(has_str_attr(reconcile_span, "origin_host_run_id", "otel-boot-1"));
    assert!(leaked_pid > 0);
}

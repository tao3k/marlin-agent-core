// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM36 ADV-L4 confinement tests for OTLP remote TLS / endpoint hygiene,
//! host observer sidecar confinement, and host watchdog future-timestamp resistance.

use marlin_agent_telemetry::{
    OpenTelemetry, OpenTelemetryConfigError, TelemetryServiceName, TraceTarget,
};

#[test]
fn sm36_otlp_refuses_remote_cleartext_http_smuggled_uris_and_forged_labels() {
    for hostile_endpoint in [
        "http://evil-collector.example:4318/v1/traces",
        "http://10.0.0.5:4318/v1/traces",
        "http://169.254.169.254/latest/meta-data",
        "http://:4318/v1/traces",
        "https://user%40evil.example/v1/traces",
        "https://collector.example/v1/traces?api_key=leak",
        "https://collector.example/v1/traces#frag",
        "https://collector.example/\r\nX-Inject: 1",
    ] {
        let err = OpenTelemetry::otlp_http(
            TelemetryServiceName::new("marlin-test"),
            TraceTarget::new("marlin.runtime"),
            hostile_endpoint,
        )
        .err()
        .unwrap_or_else(|| {
            panic!("SM36: hostile OTLP endpoint must be refused: {hostile_endpoint}")
        });
        assert!(matches!(err, OpenTelemetryConfigError::InvalidEndpoint));
    }

    for (svc, target) in [
        ("", "marlin.runtime"),
        ("   ", "marlin.runtime"),
        ("marlin\r\nforged", "marlin.runtime"),
        ("marlin-test", ""),
        ("marlin-test", "marlin\0runtime"),
    ] {
        assert!(
            OpenTelemetry::otlp_http(
                TelemetryServiceName::new(svc),
                TraceTarget::new(target),
                "http://127.0.0.1:4318/v1/traces",
            )
            .is_err(),
            "SM36: invalid telemetry label ({svc:?}, {target:?}) must fail closed"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn sm36_watchdog_resists_future_timestamp_blinding_and_invalid_attach() {
    use marlin_agent_telemetry::host_watchdog::{
        HostHeartbeatSource, HostObservation, HostObserver, HostPulse,
    };
    use std::os::unix::net::UnixDatagram;
    use std::time::Duration;

    let (_, rx_zero_pid) = HostPulse::channel().unwrap();
    assert!(
        HostObserver::attach(0, rx_zero_pid, Duration::from_millis(20)).is_err(),
        "SM36: HostObserver::attach must reject pid == 0"
    );

    let (_, rx_zero_timeout) = HostPulse::channel().unwrap();
    assert!(
        HostObserver::attach(std::process::id(), rx_zero_timeout, Duration::ZERO).is_err(),
        "SM36: HostObserver::attach must reject zero timeout"
    );

    let (raw_tx, raw_rx) = UnixDatagram::pair().unwrap();
    raw_tx.set_nonblocking(true).unwrap();
    let mut observer =
        HostObserver::attach(std::process::id(), raw_rx, Duration::from_millis(25)).unwrap();

    // Send a legitimate initial Tui heartbeat followed by a forged u64::MAX blinding datagram.
    let (legit_pulse, legit_rx) = HostPulse::channel().unwrap();
    legit_pulse.pulse(HostHeartbeatSource::Tui);
    let mut legit_bytes = [0u8; 9];
    assert_eq!(legit_rx.recv(&mut legit_bytes).unwrap(), 9);
    raw_tx.send(&legit_bytes).unwrap();

    let mut future_blinding = [1u8; 9];
    future_blinding[1..].copy_from_slice(&u64::MAX.to_le_bytes());
    raw_tx.send(&future_blinding).unwrap();

    let mut observations = Vec::new();
    observer.sample(&mut |obs| observations.push(obs)).unwrap();
    assert!(observations.contains(&HostObservation::HeartbeatStarted(
        HostHeartbeatSource::Tui
    )));

    std::thread::sleep(Duration::from_millis(60));
    observer.sample(&mut |obs| observations.push(obs)).unwrap();
    assert!(
        observations.contains(&HostObservation::HeartbeatMissed(
            HostHeartbeatSource::Tui
        )),
        "SM36: forged u64::MAX future timestamp must NOT blind HeartbeatMissed detection, got: {observations:?}"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn sm36_host_observer_refuses_symlinked_parent_report_path() {
    use marlin_agent_telemetry::start_host_observer;
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let link_dir = tmp.path().join("escaped");
    symlink(outside.path(), &link_dir).unwrap();
    let report_path = link_dir.join("observer.jsonl");

    assert!(
        start_host_observer(&report_path).is_err(),
        "SM36: start_host_observer must reject report paths traversing a symlinked parent"
    );
    assert!(
        !outside.path().join("observer.jsonl").exists(),
        "SM36: no observer report file may be created outside the intended directory"
    );
}

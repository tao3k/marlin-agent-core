// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Process arguments, telemetry lifetime, and terminal client startup.

use std::{
    env,
    io::{self, Write},
};

use marlin_agent_telemetry::{OpenTelemetry, TelemetryServiceName, TraceTarget};

use super::runner;
use crate::cli::{CliAction, list_sessions, parse_cli};

fn start_telemetry() -> Result<Option<OpenTelemetry>, Box<dyn std::error::Error>> {
    let endpoint = match env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT") {
        Ok(endpoint) => endpoint,
        Err(env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let telemetry = OpenTelemetry::otlp_http(
        TelemetryServiceName::new("marlin-tui"),
        TraceTarget::new(marlin_agent_runtime::observability::TARGET_RUNTIME_SPAN),
        &endpoint,
    )?;
    telemetry.install_global()?;
    Ok(Some(telemetry))
}

/// Start the terminal client with the process argument list.
pub fn run_cli() {
    #[cfg(target_os = "macos")]
    if marlin_agent_telemetry::run_host_observer_if_requested() {
        return;
    }
    let args = match parse_cli(env::args_os().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let telemetry = match start_telemetry() {
        Ok(telemetry) => telemetry,
        Err(error) => {
            eprintln!("marlin-tui: telemetry configuration: {error}");
            std::process::exit(2);
        }
    };
    let result = match args.action {
        CliAction::Open { session } => runner::run(args.config, session),
        CliAction::List { after } => list_sessions(&args.config, after).map(|()| None),
        #[cfg(unix)]
        CliAction::EnrollContext { remote } => {
            crate::cli::enroll_context(&args.config, remote).map(|()| None)
        }
    };
    if let Some(telemetry) = telemetry
        && let Err(error) = telemetry.shutdown()
    {
        eprintln!("marlin-tui: telemetry shutdown: {error}");
    }
    match result {
        Ok(Some(code)) => std::process::exit(code),
        Ok(None) => {}
        Err(error) => {
            let _ = writeln!(io::stderr(), "marlin-tui: {error}");
            std::process::exit(1);
        }
    }
}

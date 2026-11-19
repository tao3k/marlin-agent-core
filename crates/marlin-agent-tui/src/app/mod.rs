// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! A terminal client for Marlin's typed model gateway.

use std::{
    env,
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Instant,
};

use super::{
    cli::{CliAction, list_sessions, parse_cli},
    command::Command,
    composer::Composer,
    config,
    context_budget::ContextBudget,
    edge_gateway::{RuntimeStreamGateway, SharedGateway},
    event_queue::{QueuedReply, Reply, ReplyError, ReplySender, reply_channel},
    frame_schedule::FrameSchedule,
    input::{BracketedPaste, InputDecoder, MouseMotion, TerminalInput, WheelAccumulator},
    journal_gateway::JournalGateway,
    model_catalog::ModelCatalog,
    output_limit::{MAX_COMPLETION_HEAP_BYTES, MAX_RESPONSE_BYTES, OutputLimit},
    prompt_history::{HistorySearch, PromptHistory},
    render::{draw_with_snapshot, frame_snapshot, mouse_over_minibuffer},
    replay::{ReplayOutcome, ReplayTurn, load_session},
    session_browser::{OpenedView, SessionBrowser, SessionPageRequestError},
    signals::SignalMonitor,
    ui_host::{UiHost, UiKeyAction},
    viewport::{ScrollAnchor, Transcript, TranscriptViewportCache},
    workspace::{WorkspaceStatus, parse_query},
};
use marlin_agent_protocol::{
    ModelAlias, ModelEndpoint, ModelGatewayError, ModelGatewayMessage, ModelGatewayMessageRole,
    ModelGatewayRequest, ModelStreamInterruption, user_gateway_message,
};
use marlin_agent_runtime::{RuntimeEdgePolicy, RuntimeEdgePolicyReceipt};
use marlin_agent_sessions::{ConversationCheckpointChannel, ConversationTurnId};
use marlin_agent_storage::{
    AgentId, ProjectId, SessionId, TursoAgentStorage, TursoAgentStorageConfig,
    TursoBatchTransactionMode, TursoOptimizationProfile,
};
use marlin_agent_stream::{
    ModelStreamEvent, ModelStreamingGateway, matches_observed_anthropic_thinking,
};
use marlin_agent_telemetry::{OpenTelemetry, TelemetryServiceName, TraceTarget};
use marlin_agent_ui::{
    UiBrowseDirection, UiBufferId, UiCommand, UiCommandName, UiContent, UiEvent, UiOpenFacts,
    UiTranscriptPolicy, UiTransientId, UiWorkbench,
};
use termion::event::Key;

mod echo_area;
#[path = "editor.rs"]
mod editor;
#[path = "envrc.rs"]
mod envrc;
#[cfg(feature = "linked-ui")]
mod git_context;
#[path = "presentation.rs"]
mod presentation;
#[path = "restore.rs"]
mod restore;
#[path = "runner.rs"]
mod runner;
#[path = "runtime_probe.rs"]
mod runtime_probe;
#[path = "state.rs"]
mod state;
#[path = "terminal_owner.rs"]
mod terminal_owner;
#[path = "tool_review.rs"]
mod tool_review;
#[path = "turn.rs"]
mod turn;
#[path = "ui_events.rs"]
mod ui_events;

pub(crate) use presentation::safe_text;
pub(crate) use presentation::{
    BufferView, GatewayObservation, MessageEvent, MessageSeverity, MessageSource, ModelineActivity,
};
pub(crate) use state::{
    App, INPUT_QUEUE_ITEMS, MAX_TURN_SEGMENTS, PendingTurn, REPLY_BATCH_LIMIT, REPLY_QUEUE_BYTES,
    REPLY_QUEUE_ITEMS, RUNTIME_EVENT_BATCH_LIMIT,
};
use terminal_owner::TerminalOwner;

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

pub(crate) fn process_replies(
    app: &mut App,
    receiver: &mut tokio::sync::mpsc::Receiver<QueuedReply>,
    limit: usize,
) -> usize {
    let mut processed = 0;
    while processed < limit {
        let Ok(event) = receiver.try_recv() else {
            break;
        };
        let (id, reply) = event.into_parts();
        app.receive(id, reply);
        processed += 1;
    }
    processed
}

fn publish_error(
    app: &mut App,
    source: MessageSource,
    summary: &str,
    error: &impl std::fmt::Display,
) {
    let detail = error.to_string();
    app.publish_message(MessageEvent {
        source,
        severity: MessageSeverity::Error,
        summary,
        detail: &detail,
    });
}

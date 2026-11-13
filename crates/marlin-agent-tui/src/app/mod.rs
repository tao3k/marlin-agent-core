// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! A terminal client for Marlin's typed model gateway.

use std::{
    io,
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
    render::{draw, frame_regions, mouse_over_minibuffer},
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
use marlin_agent_ui::{
    UiBrowseDirection, UiBufferId, UiCommand, UiCommandName, UiContent, UiEvent, UiOpenFacts,
    UiTranscriptPolicy, UiTransientId, UiWorkbench,
};
use termion::event::Key;

#[path = "presentation.rs"]
mod presentation;
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

#[cfg(test)]
#[cfg_attr(test, allow(unused_imports))]
pub(crate) use runner::process_replies;
// The standalone unit harness imports this module without the binary facade.
#[cfg_attr(test, allow(unused_imports))]
pub use runner::run_cli;

#[cfg(test)]
#[cfg_attr(test, allow(unused_imports))]
pub(crate) use presentation::safe_text;
pub(crate) use presentation::{
    BufferView, GatewayObservation, MessageEvent, MessageSeverity, MessageSource, ModelineActivity,
};
pub(crate) use state::{
    App, INPUT_QUEUE_ITEMS, MAX_TURN_SEGMENTS, PendingTurn, REPLY_BATCH_LIMIT, REPLY_QUEUE_BYTES,
    REPLY_QUEUE_ITEMS, RUNTIME_EVENT_BATCH_LIMIT,
};
use terminal_owner::TerminalOwner;

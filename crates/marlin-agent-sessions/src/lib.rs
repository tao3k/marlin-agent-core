// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Session identity, context visibility, and isolation contracts for agent runtimes.

mod context;
mod conversation;
mod id;
mod identity;
mod runtime;
mod storage;

pub use context::{
    ContextExpansionPolicy, ContextNamespace, ContextVisibility, SessionIsolationPolicy,
};
pub use conversation::{
    CONVERSATION_MAX_CHECKPOINT_SEGMENTS, ConversationCheckpointChannel,
    ConversationCheckpointSpan, ConversationEvent, ConversationEventEntry, ConversationEventReader,
    ConversationJournal, ConversationJournalError, ConversationPendingToolRequest,
    ConversationSubmissionId, ConversationTerminalObservation, ConversationToolExecutionClaim,
    ConversationToolRequestKey, ConversationToolReviewDecision, ConversationToolReviewReceipt,
    ConversationToolReviewRequest, ConversationToolReviewStatus, ConversationTurn,
    ConversationTurnId, ConversationTurnOutcome, ConversationTurnStart,
    ConversationUncertaintyReason,
};
pub use id::{SessionId, SessionIdError, SessionKind};
pub use identity::{AgentSessionContext, SessionIdentity, SessionIsolationReceipt};
pub use runtime::{
    RuntimeBlockingBridgePolicy, RuntimeBlockingBridgeStrategy, RuntimeFanoutJoinPolicy,
    RuntimeTaskTrackerPolicy, SessionRuntimeSnapshot, TokioRuntimeDiagnosticsPolicy,
    TokioRuntimeFlavor, TokioRuntimePolicy, TokioRuntimePolicyReceipt,
};
pub use storage::{
    SessionStorageAdapter, SessionStorageEvent, SessionStorageEventBuilder,
    SessionStorageProjectionError,
};

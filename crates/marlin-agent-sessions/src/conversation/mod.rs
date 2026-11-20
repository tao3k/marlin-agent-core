// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Ordered conversation turns over the runtime-owned session event store.

mod events;
mod journal;
mod tool_review;
mod turn;

pub use events::{
    ConversationCancellationEvidence, ConversationEvent, ConversationEventEntry,
    ConversationEventReader, ConversationRetirementStatus, ConversationTail, ConversationTailState,
};
pub use journal::ConversationJournal;
pub use tool_review::{
    ConversationToolCallResult, ConversationToolExecutionClaim, ConversationToolRequestKey,
    ConversationToolResultOutcome, ConversationToolResultsReceipt, ConversationToolReviewDecision,
    ConversationToolReviewReceipt, ConversationToolReviewRequest, ConversationToolReviewStatus,
};
pub use turn::{
    CONVERSATION_MAX_CHECKPOINT_SEGMENTS, ConversationCheckpointChannel,
    ConversationCheckpointSpan, ConversationJournalError, ConversationPendingToolRequest,
    ConversationSubmissionId, ConversationTerminalObservation, ConversationToolContinuationStart,
    ConversationTurn, ConversationTurnId, ConversationTurnOutcome, ConversationTurnSource,
    ConversationTurnStart, ConversationUncertaintyReason,
};

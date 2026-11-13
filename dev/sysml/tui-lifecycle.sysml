// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Target TUI turn lifecycle, including failure and late events.
package MarlinTuiLifecycle {
    private import MarlinTuiArchitecture::*;

    attribute def Submitted;
    attribute def Admitted;
    attribute def ChunkReceived;
    attribute def Completed;
    attribute def Failed;
    attribute def Denied;
    attribute def CancelRequested;
    attribute def Retired;

    state def <'MAR-TUI-L01'> TurnLifecycle {
        doc /* Runtime owns terminal outcome. Partial display is provisional;
               cancellation requests retirement and late events cannot commit. */
        first start then editing;
        state editing;
        state admitting;
        state streaming;
        state cancelling;
        state completed;
        state failed;
        state denied;
        state cancelled;
        transition submit first editing accept Submitted then admitting;
        transition acceptTurn first admitting accept Admitted then streaming;
        transition denyTurn first admitting accept Denied then denied;
        transition failAdmission first admitting accept Failed then failed;
        transition cancelAdmission first admitting accept CancelRequested then cancelling;
        transition displayChunk first streaming accept ChunkReceived then streaming;
        transition finish first streaming accept Completed then completed;
        transition failTurn first streaming accept Failed then failed;
        transition cancelTurn first streaming accept CancelRequested then cancelling;
        transition finishCancellation first cancelling accept Retired then cancelled;
    }

    dependency turnOwnedByRuntime from TurnLifecycle to target::runtime;
    dependency turnProjectedByClient from TurnLifecycle to target::terminalClient::projection;
    dependency turnEvidenceRetained from TurnLifecycle to target::sessionStore;

    attribute def HandoffRequested;
    attribute def HandoffFinished;
    attribute def SuspendRequested;
    attribute def ForegroundRestored;
    attribute def ShutdownRequested;
    attribute def ExternalTermination;

    state def <'MAR-TUI-L02'> TerminalOwnershipLifecycle {
        doc /* Planned single-owner terminal protocol. Suspension and child
               handoff first release input and restore host modes. Resume
               rechecks foreground ownership and current terminal size. Unix
               termination signals observed in the active loop request local
               cleanup; this does not prove remote retirement. */
        first start then active;
        state active;
        state handedOff;
        state suspended;
        state closed;
        transition handoff first active accept HandoffRequested then handedOff;
        transition reclaim first handedOff accept HandoffFinished then active;
        transition suspend first active accept SuspendRequested then suspended;
        transition resume first suspended accept ForegroundRestored then active;
        transition close first active accept ShutdownRequested then closed;
        transition signalClose first active accept ExternalTermination then closed;
    }

    dependency terminalOwnedByClient from TerminalOwnershipLifecycle to target::terminalClient::terminalOwnership;

    attribute def EvidenceAccepted;
    attribute def EvidenceCommitted;
    attribute def EvidenceWriteRejected;
    attribute def EvidenceWriteUncertain;
    attribute def EvidenceReconciledCommitted;
    attribute def EvidenceReconciledNotCommitted;

    state def <'MAR-TUI-L06'> EvidenceDeliveryLifecycle {
        doc /* Target durable evidence admission. Accepted is provisional;
               a write timeout alone cannot establish whether storage committed.
               Only a successful commit or reconciliation permits acknowledgement.
               Rejection and uncertainty never imply successful replayability. */
        first start then awaiting;
        state awaiting;
        state pending;
        state committed;
        state rejected;
        state uncertain;
        transition acceptEvidence first awaiting accept EvidenceAccepted then pending;
        transition commitEvidence first pending accept EvidenceCommitted then committed;
        transition rejectEvidence first pending accept EvidenceWriteRejected then rejected;
        transition uncertainWrite first pending accept EvidenceWriteUncertain then uncertain;
        transition reconcileCommitted first uncertain accept EvidenceReconciledCommitted then committed;
        transition reconcileNotCommitted first uncertain accept EvidenceReconciledNotCommitted then rejected;
    }

    dependency evidenceDeliveryOwnedByRuntime from EvidenceDeliveryLifecycle to target::runtime::evidenceAdmission;
    dependency evidenceDeliveryPersisted from EvidenceDeliveryLifecycle to target::sessionStore;

    attribute def ToolAdvertised;
    attribute def ModelToolRequested;
    attribute def ToolRequestCommitted;
    attribute def ReviewApproved;
    attribute def ReviewRejected;
    attribute def ToolExecutionStarted;
    attribute def ToolExecutionClaimCommitted;
    attribute def ToolResultCommitted;
    attribute def ToolExecutionUncertain;

    state def <'MAR-TUI-L07'> ModelToolReviewLifecycle {
        doc /* A model tool name is offered only with a registered runtime
               handler. One request retains an immutable definition/handler
               snapshot, so a later catalog update cannot authorize a call
               absent from that request. The provider call ID, tool identity and durable
               conversation turn must agree before the Mini Buffer can show
               Ask. A decision receipt is consumed once. A stale receipt,
               provisional request or uncertain result cannot authorize a
               second execution. The runtime commits an execution claim before
               invoking a handler. Recovery after a claimed but unrecorded
               result stays uncertain and never retries silently. The UI binds
               Ask to the Mini Buffer as a
               temporary Frame, retains the conversation draft, and returns
               to its prior Buffer after resolution; it does not add a side
               panel. Scheme owns that Frame's choices and keymap. The UI
               owns presentation, never authority. */
        first start then unavailable;
        state unavailable;
        state offered;
        state requested;
        state awaitingReview;
        state approved;
        state rejected;
        state claimed;
        state executing;
        state settled;
        state uncertain;
        transition offer first unavailable accept ToolAdvertised then offered;
        transition request first offered accept ModelToolRequested then requested;
        transition commitRequest first requested accept ToolRequestCommitted then awaitingReview;
        transition approveRequest first awaitingReview accept ReviewApproved then approved;
        transition rejectRequest first awaitingReview accept ReviewRejected then rejected;
        transition claimApproved first approved accept ToolExecutionClaimCommitted then claimed;
        transition executeClaimed first claimed accept ToolExecutionStarted then executing;
        transition uncertainBeforeObservation first claimed accept ToolExecutionUncertain then uncertain;
        transition persistResult first executing accept ToolResultCommitted then settled;
        transition uncertainResult first executing accept ToolExecutionUncertain then uncertain;
    }

    dependency modelToolReviewOwnedByRuntime from ModelToolReviewLifecycle to target::runtime;
    dependency modelToolReviewPersisted from ModelToolReviewLifecycle to target::sessionStore;
    dependency modelToolReviewProjected from ModelToolReviewLifecycle to target::terminalClient::projection;

    attribute def InterruptKey;
    attribute def OtherInput;
    attribute def ActiveWorkStarted;
    attribute def WorkFinished;

    state def <'MAR-TUI-L05'> InterruptInteraction {
        doc /* Ctrl-C cancels active local work, retaining uncertainty about
               remote retirement. When idle, one interrupt arms exit and the
               next consecutive interrupt closes; other input disarms exit.
               The first idle interrupt does not discard the draft. */
        first start then idle;
        state idle;
        state active;
        state armed;
        state closed;
        transition beginWork first idle accept ActiveWorkStarted then active;
        transition localCancel first active accept InterruptKey then idle;
        transition workFinished first active accept WorkFinished then idle;
        transition armExit first idle accept InterruptKey then armed;
        transition disarmExit first armed accept OtherInput then idle;
        transition confirmExit first armed accept InterruptKey then closed;
    }

    dependency interruptHandledByClient from InterruptInteraction to target::terminalClient;

    attribute def StartClaimed;
    attribute def ClaimConflict;
    attribute def UnfinishedReplayed;
    attribute def TerminalPersisted;
    attribute def TerminalPersistenceFailed;
    attribute def ClientLost;
    attribute def ProjectionMismatch;
    attribute def ReplayShowsFinished;
    attribute def CheckpointPersisted;
    attribute def CheckpointPersistenceFailed;
    attribute def OpenOutcomeUncertain;
    attribute def StreamOutcomeUncertain;
    attribute def UncertaintyPersisted;

    state def <'MAR-TUI-L03'> ConversationAdmissionLifecycle {
        doc /* A fresh claim precedes dispatch. A replayed incomplete turn has
               unknown ownership; without a lease it is neither automatically
               retried nor terminated by a newly opened client. After stream
               open, an unclassified gateway error is an uncertain observation,
               not evidence of retirement. The same applies to unclassified
               open errors; only an explicit pre-dispatch rejection permits
               the failed turn to release admission. The journal's terminal
               write accepts completion or pre-dispatch rejection. After a
               recorded rejection, the prompt is restored if no new draft exists;
               otherwise both the new draft and visible rejected prompt remain.
               A rejection-labelled stream item remains uncertain. Local
               cancellation and legacy failures are replay-only observations.
               A legacy failure or local cancellation blocks new claims even
               if later historical turns exist; reconciliation is not implemented. */
        first start then ready;
        state ready;
        state owned;
        state unowned;
        state conflict;
        state finished;
        state uncertain;
        transition claim first ready accept StartClaimed then owned;
        transition staleClaim first ready accept ClaimConflict then conflict;
        transition replayIncomplete first ready accept UnfinishedReplayed then unowned;
        transition persistTerminal first owned accept TerminalPersisted then finished;
        transition persistPrefix first owned accept CheckpointPersisted then owned;
        transition checkpointFailure first owned accept CheckpointPersistenceFailed then uncertain;
        transition observeOpenUncertainty first owned accept OpenOutcomeUncertain then uncertain;
        transition observeStreamUncertainty first owned accept StreamOutcomeUncertain then uncertain;
        transition recordOpenUncertainty first uncertain accept UncertaintyPersisted then unowned;
        transition persistenceFailure first owned accept TerminalPersistenceFailed then uncertain;
        transition lostClient first owned accept ClientLost then unowned;
        transition projectionFailure first finished accept ProjectionMismatch then uncertain;
        transition reopenFinished first uncertain accept ReplayShowsFinished then ready;
        transition reopenIncomplete first uncertain accept UnfinishedReplayed then unowned;
    }

    dependency claimOwnedByJournal from ConversationAdmissionLifecycle to target::sessionStore::conversationJournal;

    attribute def SessionSwitchRequested;
    attribute def SessionSwitchLoaded;
    attribute def SessionSwitchFailed;
    attribute def SessionSwitchCancelled;

    state def <'MAR-TUI-L04'> SessionSwitchLifecycle {
        doc /* Switch begins only without an active model turn. The existing
               session stays authoritative until the target is fully loaded;
               failure, concurrent append detection, or cancellation keeps it.
               A separate command editor retains the message draft and cursor;
               after a successful switch the draft belongs to the new session. */
        first start then current;
        state current;
        state loading;
        transition beginSwitch first current accept SessionSwitchRequested then loading;
        transition commitSwitch first loading accept SessionSwitchLoaded then current;
        transition failSwitch first loading accept SessionSwitchFailed then current;
        transition cancelSwitch first loading accept SessionSwitchCancelled then current;
    }
}

// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

package MarlinAppServer {
    part <'MAR-APP-A01'> appServer {
        doc /* marlin-agent-app-server exposes generated bindings through
               client and the host service through grpc. Agent CORE enables
               the service through app-server. The host supplies the
               listener, TLS, authorization, model route and shutdown.
               The application protocol projects Marlin Session operations. */
        part <'MAR-APP-A02'> turnService {
            doc /* marlin.app.v1 provides authenticated GetCapabilities and
                   StreamTurn. StreamTurn emits started, ordered text deltas
                   and a completion summary. Session-bound RPCs start a journaled
                   turn, resume from a committed event cursor, and request
                   cancellation with a typed local Runtime receipt. */
        }
        part <'MAR-APP-A03'> modelGateway {
            doc /* Existing ModelStreamingGateway, with the Weaver adapter as
                   a host-selectable implementation. */
        }
        part <'MAR-APP-A05'> sessionJournal {
            doc /* ConversationJournal owns admission and checkpoint commits.
                   SessionEventCursor provides an exclusive ordered position.
                   Reconnection reads the same committed SessionEventStore. */
        }
        part <'MAR-APP-A06'> runtimeTurnScope {
            doc /* Tokio Runtime owns the task, child cancellation and join
                   accounting. A disconnected observer leaves the task active.
                   Session outcomes preserve uncertainty about remote effects. */
        }
        dependency turnUsesJournal from turnService to sessionJournal;
        dependency turnUsesRuntime from turnService to runtimeTurnScope;
        dependency turnUsesGateway from turnService to modelGateway;
    }
    part <'MAR-APP-A04'> externalClient {
        doc /* Web, desktop and mobile clients are maintained in separate
               repositories and use the Rust-owned Protobuf schema.
               Rust applications can use marlin-agent-ui/grpc with default
               features disabled. Clients own
               rendering and view state. Browser access can use a
               deployment-owned gRPC-Web gateway on a trusted origin. */
    }
    part <'MAR-APP-A07'> weaverInterop {
        doc /* tao3k/weaver owns A2A and client-facing Agent Client Protocol
               adapters. The planned Marlin integration translates operations
               supported by marlin.app.v1 to its gRPC service. Weaver owns
               external protocol identity and lifecycle mapping. Marlin owns
               Session admission, event order, cancellation and local receipts. */
    }
    connection <'MAR-APP-C01'> grpcChannel connect externalClient to appServer.turnService;
    connection <'MAR-APP-C02'> weaverGrpcChannel connect weaverInterop to appServer.turnService;

    requirement <'MAR-APP-R07'> clientBuildIsolation {
        doc /* The Rust UI grpc feature with defaults off builds the
               generated marlin.app.v1 client and descriptor independently of
               Gerbil Scheme, the internal protocol, Session, Runtime and
               storage. App Server grpc enables the service and its owners.
               Both build paths use the same Protobuf schema. */
    }
    allocation clientBuildBoundary allocate clientBuildIsolation to externalClient;
    allocation serviceBuildBoundary allocate clientBuildIsolation to appServer.turnService;

    action def <'MAR-APP-F01'> ApplicationMessageFlow {
        doc /* Representative HTTP/2 fixture sequence for a known empty Session.
               The two observations after Stop may arrive in either order.
               The Org walkthrough describes the GPUI projection and failure cases. */
        action bindSession {
            doc /* GPUI reads the host-bound session_id from GetCapabilities. */
        }
        action submitDraft {
            doc /* GPUI sends submission_id, session_id, user_text and after_turn in
                   StartSessionTurn and keeps the draft pending. */
        }
        action commitStart {
            doc /* The existing ConversationJournal atomically commits the
                   submission key and start before dispatch. The same ID and
                   input recover the original turn. Changed input returns
                   ALREADY_EXISTS. */
        }
        action watchTurn {
            doc /* GPUI opens WatchSessionTurn. Rust projects the journal's
                   started event and committed visible text checkpoints. */
        }
        action applyCheckpoint {
            doc /* GPUI applies the visible event and saves its opaque event ID
                   with the projection. This is an exclusive replay position. */
        }
        action disconnect {
            doc /* The observer disconnects. Tokio retains the turn task and
                   the Session can commit more checkpoints. */
        }
        action replaySuffix {
            doc /* GPUI sends the same turn reference and saved after_event_id.
                   Rust reads the committed suffix from the same journal. */
        }
        action requestStop {
            doc /* GPUI calls CancelSessionTurn for that turn while the watch
                   can continue. Rust requests cancellation in the turn scope. */
        }
        action observeLocalRetirement {
            doc /* The unary response projects the Runtime shutdown receipt.
                   It conveys local evidence for the requested turn. */
        }
        action applyOutcome {
            doc /* GPUI applies the committed Session outcome from the watch.
                   It saves the terminal event ID with the view. The fixture
                   keeps provider status uncertain after local retirement. */
        }

        first start then bindSession;
        first bindSession then submitDraft;
        first submitDraft then commitStart;
        first commitStart then watchTurn;
        first watchTurn then applyCheckpoint;
        first applyCheckpoint then disconnect;
        first disconnect then replaySuffix;
        first replaySuffix then requestStop;
        first requestStop then observeLocalRetirement;
        first requestStop then applyOutcome;
    }

    requirement <'MAR-APP-R01'> authorizedExternalAccess {
        doc /* Authorize each RPC for the host-selected route. Clients carry
               text and transcript roles. Host policy owns effect authority. */
    }
    requirement <'MAR-APP-R02'> boundedTurnStream {
        doc /* Pull the provider stream under message, byte and in-flight
               limits. Deadline and client disconnect release local work.
               Report malformed or truncated streams through gRPC status. */
    }
    requirement <'MAR-APP-R03'> reconnectAndRetire {
        doc /* Commit start before dispatch. Resume after the last processed
               journal cursor. Publish local retirement after task completion
               and preserve the committed Session outcome independently. */
    }
    allocation sessionAllocation allocate reconnectAndRetire to appServer.sessionJournal;
    allocation retirementAllocation allocate reconnectAndRetire to appServer.runtimeTurnScope;
    allocation accessAllocation allocate authorizedExternalAccess to appServer.turnService;
    allocation streamAllocation allocate boundedTurnStream to appServer.turnService;

    // Implemented submission recovery. The Org contract describes the failure cases.
    requirement <'MAR-APP-R04'> recoverSubmission {
        doc /* The Session owner atomically binds an authorized submission ID
               and the original inputs to its committed turn before dispatch.
               An identical retry resolves to that turn even after later
               turns commit. A conflicting retry is rejected. Recovery of an
               uncertain dispatch preserves the original identity. After a
               restart, replay committed events and report an unfinished turn
               without a live owner. Recovery does not dispatch that turn. */
    }
    requirement <'MAR-APP-R05'> boundedSessionRecovery {
        doc /* GetSessionSummary reads one indexed event header for the
               authorized Session and reports typed recovery state. Replay
               committed visible events after the caller's exclusive cursor.
               Bound page work and bytes, and isolate slow replay from control
               calls for other Sessions. A terminal commit racing attachment
               remains visible in the journal suffix. */
    }
    // Proposed cancellation readback.
    requirement <'MAR-APP-R06'> recoverCancellationEvidence {
        doc /* Keep Session outcome, local retirement and provider stop
               evidence independently observable by turn. Persist any Runtime
               receipt offered across restart. Expose missing evidence and
               unresolved work when the corresponding owner cannot confirm it. */
    }
    allocation submissionIdentity allocate recoverSubmission to appServer.sessionJournal;
    allocation recoveryBudget allocate boundedSessionRecovery to appServer.turnService;
    allocation recoveryReplay allocate boundedSessionRecovery to appServer.sessionJournal;
    allocation cancellationObservation allocate recoverCancellationEvidence to appServer.runtimeTurnScope;
    allocation cancellationReadback allocate recoverCancellationEvidence to appServer.sessionJournal;
}

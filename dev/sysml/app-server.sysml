// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

package MarlinAppServer {
    part <'MAR-APP-A01'> appServer {
        doc /* Cargo package marlin-agent-app-server exposes the grpc feature.
               Agent CORE exposes it through app-server. The host supplies the
               listener, TLS, authorization, model route and shutdown. */
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
        doc /* GUI, browser frontend or peer App Server. Browsers connect via
               a deployment-owned gRPC-Web gateway on a trusted origin. */
    }
    connection <'MAR-APP-C01'> grpcChannel connect externalClient to appServer.turnService;

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
}

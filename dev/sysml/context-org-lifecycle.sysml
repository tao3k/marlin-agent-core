// Context-owned Org design: ../../docs/40-rfcs/40.181-context-org-ownership.org
package MarlinContextOrgLifecycle {
    doc /* Target only. No transition in this model asserts that the existing
           filesystem store provides transactions, source authentication,
           current authorization, or remote retirement. */

    attribute def TurnSubmitted;
    attribute def OrgStartCommitted;
    attribute def OrgStartUnknown;
    attribute def CurrentUseAdmitted;
    attribute def CurrentUseDenied;
    attribute def ProviderObserved;
    attribute def ProviderUncertain;
    attribute def OutcomeCommitted;
    attribute def OutcomeUnknown;
    attribute def StorageReconciledCommitted;
    attribute def StorageReconciledAbsent;

    part def <'MAR-CONTEXT-P01'> ContextOrgIntegration {
        doc /* Target ownership, not an implementation claim. The pinned Lab
               Context provides the projection, Session input proposal and
               coordinating host. The Rust boundary must check native readiness;
               the existing bridge does not expose these operations. */
        part <'MAR-CONTEXT-P02'> sourceOwner {
            doc /* Rust owns Org serialization and must retain every accepted
                   conversation turn and typed continuation. A successful write
                   alone is not evidence of a durable atomic turn commit.
                   Filesystem writes sync the replacement file and, on Unix,
                   its immediate parent after rename; this does not supply
                   concurrent CAS, ancestor durability or a session journal.
                   An explicit atomic compare-and-write capability compares
                   exact source snapshots; only the in-memory adapter currently
                   implements it. It does not establish revision identity,
                   atomic clean-state admission or durable persistence.
                   The old patch-based committer was removed; Lab Context and
                   its durable owner are not yet connected to this boundary. */
        }
        part <'MAR-CONTEXT-P03'> labProjection {
            doc /* A scoped, bounded projection and replay of source, not a
                   complete turn journal, source authentication, current-use
                   authorization, or proof of durable storage. Marlin Cargo and
                   Lab declare the same Orgize revision; that does not prove
                   a shared VM, a qualified native closure, or a turn journal. */
        }
        part <'MAR-CONTEXT-P04'> labSessionHost {
            doc /* Coordinates enrolled source snapshots and Session input
                   proposals with the independent Session lifecycle. Host
                   resolution is required before proposal publication. */
        }
        part <'MAR-CONTEXT-P05'> nativeBoundary {
            doc /* Rust checks ABI readiness and transfers typed requests and
                   receipts; no formatted text grants runtime authority. */
        }
        part <'MAR-CONTEXT-P06'> terminalClient {
            doc /* Presents runtime observations and submits user interactions;
                   it is not the source or lifecycle authority. */
        }
        part <'MAR-CONTEXT-P07'> orgBuildClosure {
            doc /* Orgize declares the Scheme bridge it imports. Lab's Scheme
                   package does not declare that bridge directly. Lab's current
                   Bazel producer still wires the bridge for Orgize; migration
                   of that build closure to Orgize remains open. */
        }
        dependency projectionReadsSource from labProjection to sourceOwner;
        dependency hostUsesProjection from labSessionHost to labProjection;
        dependency boundaryInvokesHost from nativeBoundary to labSessionHost;
        dependency clientUsesBoundary from terminalClient to nativeBoundary;
        dependency boundaryRequiresOrgClosure from nativeBoundary to orgBuildClosure;
    }

    state def <'MAR-CONTEXT-L01'> OrgTurnLifecycle {
        doc /* A typed source identity, scope, session and turn travel through
               the owning runtime. Persist the input before external dispatch;
               then obtain a current-use decision. Preserve provider outcome,
               including opaque continuation, before reporting replayability.
               Uncertain writes and remote work require separate reconciliation.
               No transition performs compression or infers remote retirement. */
        first start then draft;
        state draft;
        state startPending;
        state startUnknown;
        state ready;
        state dispatched;
        state providerUnknown;
        state outcomePending;
        state outcomeUnknown;
        state committed;
        state notCommitted;
        transition submit first draft accept TurnSubmitted then startPending;
        transition writeStart first startPending accept OrgStartCommitted then ready;
        transition unknownStart first startPending accept OrgStartUnknown then startUnknown;
        transition reconcileStart first startUnknown accept StorageReconciledCommitted then ready;
        transition absentStart first startUnknown accept StorageReconciledAbsent then notCommitted;
        transition admit first ready accept CurrentUseAdmitted then dispatched;
        transition deny first ready accept CurrentUseDenied then outcomePending;
        transition observe first dispatched accept ProviderObserved then outcomePending;
        transition uncertainProvider first dispatched accept ProviderUncertain then providerUnknown;
        transition writeOutcome first outcomePending accept OutcomeCommitted then committed;
        transition unknownOutcome first outcomePending accept OutcomeUnknown then outcomeUnknown;
        transition reconcileOutcome first outcomeUnknown accept StorageReconciledCommitted then committed;
        transition absentOutcome first outcomeUnknown accept StorageReconciledAbsent then outcomePending;
    }
}

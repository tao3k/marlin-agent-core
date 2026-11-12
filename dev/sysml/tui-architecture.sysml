// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// TUI design and evidence: ../../docs/40-rfcs/40.180-codewhale-inspired-tui.org
package MarlinTuiArchitecture {
    doc /* The actual package describes source-inspected implementations, not
           measurements. The target is a design, not a claim of implementation.
           Dependencies point from consumer to supplier; allocations identify
           the component responsible for a requirement. */

    package actual {
        part <'MAR-TUI-A19'> selectedNativeAdapters {
            doc /* The selected ui-components Module owns component prototypes,
                   bounded content constructors and the command catalog. The
                   selected ui-modeline Module owns the ordered semantic slots.
                   The ui Module requires both scopes and owns workbench
                   composition, generic event dispatch and typed projection.
                   The default Guide sidebar has been removed. The selected
                   c-abi Module owns UI and Deck adapter build
                   inputs through its Build role. Its published identity is
                   metadata, not a Rust ABI readiness receipt. Its linked scope
                   retains the selected UI, Context and Deck scopes. Scheme UI
                   dispatch returns declarative submit, dismiss, scroll and
                   workbench-ordered focus intents. The linked TUI routes these
                   through a Rust-owned terminal host. Rust uses the SDK's
                   LinkedObjectExport and rooted-value conversion for a
                   Scheme-owned session, workbench and intent projections. Rust
                   admits event/intent/successor agreement in the shared UI
                   library; terminal command handlers and focus are admitted
                   by the terminal host. Rust resolves modeline labels from
                   live facts and paints a colored width-aware strip above the
                   collapsible Mini Buffer. The terminal host constructs
                   rooted open requests from typed facts and SDK
                   objects after checking outer contribution limits.
                   Ordered CLOS method bundles compose into each fresh session
                   generic, with an upper bound on bundle count; a bundle's
                   internal size and its Module provenance remain unqualified. A
                   linked executable binds the selected UI publication and six
                   C ABI exports to the same rooted Rust request/response path
                   as the dynamic build. A PTY test opens the workbench without
                   a Gerbil package tree and restores terminal state. The
                   selected Yuanxu projection supplies its compiled closure;
                   no second list of Module targets is maintained. Identifier
                   names are copied before their Rust length check, so total
                   transient memory bounds are not established. */
        }
        part <'MAR-TUI-A14'> weaverConfiguration {
            doc /* Marlin pins unmerged Weaver PR #4 revision 2cbb6bf7, whose
                   prepared-profile API uses explicit configuration first and
                   Anthropic environment fallback. No context compression is
                   implied by this configuration. */
        }
        part <'MAR-TUI-A17'> sessionEventBoundary {
            doc /* Session events use a distinct typed storage contract. The
                   existing in-memory and Turso implementations provide it;
                   the conversation journal and TUI require only that contract.
                   No Org-backed event store or Lab Session lifecycle is implied. */
        }
        part <'MAR-TUI-A18'> legacySchemeBoundaryMigration {
            doc /* Gerbil IR no longer consumes the Org model or Workspace
                   patch and view contracts. Unused workspace policy records
                   and the legacy patch-intent workflow have been removed.
                   The Scheme integration
                   no longer has an unused direct Org model dependency, but
                   Workspace still depends on that model. Graph compilation
                   and release paths still consume the legacy IR. The existing
                   native projection has a readiness gate; VM execution of the
                   replacement graph and Org contract paths is not established.
                   The Core facade no longer re-exports the legacy raw graph
                   compilers; its execution-request compiler requires ABI
                   readiness for Scheme policy proposals. The lower protocol
                   still contains the old graph compiler pending migration. */
        }
        part <'MAR-TUI-A16'> contextOrgMigration {
            doc /* The former Rust org-memory projection is now under the
                   Context crate. It is still an in-memory workspace, not an
                   authoritative Org turn store or a Lab Org projection.
                   Org source and workspace APIs are Context modules; the
                   legacy source patch planner, commit workflow, and in-memory
                   Workspace patch mutation were removed. The remaining local
                   read model is transitional, not Lab Org projection.
                   The shared Org model remains a separate compilation unit.
                   The TUI's independent history/journal
                   remains. Context exposes Lab Scope, Org Contract source and
                   source-backed Scheme Org graph operations. These do not
                   authorize disclosure or establish durability, and the Rust
                   workspace loader is not yet migrated.
                   Source writes create temporary paths exclusively; failed
                   store calls yield an uncertain write outcome. Clean, dirty
                   and unknown source observations are distinct; the filesystem
                   adapter reports unknown. Parent symlink safety, durability
                   and concurrent CAS remain open. */
        }
        part <'MAR-TUI-A15'> providerContinuation {
            doc /* Marlin's typed message can retain a redacted-debug opaque
                   OpenAI Responses output from either gateway path. Logical
                   provider/model/alias mismatch is denied before dispatch;
                   new continuations also bind the configured destination and
                   credential fingerprint. Missing or mismatched bindings deny
                   dispatch; historical reconciliation remains unimplemented.
                   The sensitive fingerprint does not prove server identity. Replay,
                   context admission and completion memory accounting include
                   the carrier. Old records cannot prove prior output retention.
                   Local Weaver preserves Anthropic thinking and signatures,
                   exposes redacted-debug thinking fragments and typed thinking
                   controls, and succeeded in real DeepSeek streams and follow-ups.
                   Local Weaver checks Responses terminal text against a digest
                   of streamed text. Cargo currently uses the sibling Weaver
                   worktree; a portable published revision remains required.
                   Session journal can record ordered thinking checkpoints and
                   replay their ordered interleaving with answer text. The TUI can
                   show those recorded prefixes. The local Weaver adapter
                   delivers live thinking events and Anthropic continuation;
                   consistency of observed thinking and answer with that
                   continuation gates follow-up, without authenticating the
                   signature. Optional per-route Anthropic thinking and effort
                   use Weaver validation and request encoding; Org persistence
                   remains open.
                   These are not compression mechanisms. */
        }
        part <'MAR-TUI-A13'> nativeGraphAdmission {
            doc /* Static proposal compilation refuses native policy planes.
                   Native compilation requires a typed ABI readiness receipt
                   for the same identity, version and required symbol set,
                   including internally consistent counts. The AgentGraph
                   request builder checks the planning, projection, admission
                   and root-entry evidence for consistency. Receipt provenance
                   and a TUI graph execution connection remain separate work. */
        }
        part <'MAR-TUI-A01'> codewhaleFrameControl {
            doc /* Reference b8643a76: frame_rate_limiter.rs caps draws and
                   coalesces changes. This is an existing strength. */
        }
        part <'MAR-TUI-A02'> codewhaleTranscriptCaches {
            doc /* Reference b8643a76: viewport transcript cache and bounded
                   output rows cache. Cache size and invalidation are explicit. */
        }
        part <'MAR-TUI-A03'> codewhaleTranslationQueue {
            doc /* Reference b8643a76: ui/event_loop.rs creates an unbounded
                   TranslationEvent channel. Queue reachability and impact under
                   workload remain unmeasured; this is a risk, not a proven leak. */
        }
        part <'MAR-TUI-A04'> marlinTerminal {
            doc /* Current TUI suppresses idle draws and lays out visible rows
                   independently of the terminal renderer and u16 scroll.
                   One visible-viewport cache is reused until transcript,
                   dimensions, scroll position, or pending state changes.
                   Retained viewport allocations have a 1 MiB accounting
                   budget; oversized views are rendered without retention.
                   Consecutive zero-width source graphemes are represented
                   as borrowed runs in temporary wrapping state. Deep scroll
                   and changed long lines still require scans. At the live
                   tail, long lines can use one scan and retain at most 16384
                   row-slice slots; excess falls back to two scans. Rendered row
                   bytes and total temporary allocations remain unbounded.
                   After a nonzero-size draw, scroll offset is clamped to
                   the actual top for that layout, avoiding invisible overshoot.
                   Tail-only additions preserve the scrolled row position at
                   unchanged dimensions, including waiting-indicator removal.
                   Resize, clearing and non-tail changes invalidate the anchor;
                   deterministic repeated-resize frames and cursor are checked
                   against a fresh render, and a Unix PTY exercises resizing
                   during a live stream. Cross-terminal screen behavior remains
                   unqualified.
                   it is not a stable source-text position. Completed source
                   line counts are retained for the current tail; its open
                   line is rescanned. Input can precede the next draw's clamp.
                   Bracketed paste is bounded; unbracketed paste is unresolved.
                   Home/End and Ctrl-A/Ctrl-E move within the logical input
                   line, regardless of visual wrapping. Prompt navigation
                   uses completed current-session context; the draft and its
                   cursor are restored when leaving history or sending. */
        }
        part <'MAR-TUI-A05'> marlinStream {
            doc /* Weaver emits ordered chunks up to 64 KiB. The TUI queue
                   admits 64 items and 32 MiB of accounted heap capacity;
                   turns exceeding limits fail explicitly. Both TUI modes
                   preserve typed gateway errors through the queue and account
                   their owned payload capacities; the transcript currently
                   formats them rather than retaining a typed failure projection.
                   Both modes limit cumulative accepted text per turn to 16 MiB,
                   rejecting the first over-limit event before UI admission.
                   The configured Weaver adapter also rejects an over-limit
                   chunk before appending it to its accumulated completion;
                   provider decode allocations precede that check.
                   Completion metadata and all choices share the queue's
                   32 MiB heap-capacity ceiling before journal cloning;
                   rejection records a typed unresolved observation.
                   Complete model context and the next prompt are budgeted
                   before request cloning and dispatch, without truncation.
                   A local context view uses the same byte and turn accounting;
                   it does not estimate provider tokens or process memory.
                   A shared immutable history snapshot moves the full request
                   clone to a Tokio worker. Overlapping cancellation and a
                   later history mutation can trigger terminal-thread copy-on-write.
                   A clone consumes memory and occupies its executing thread;
                   cancellation does not establish remote retirement.
                   A response may take retained context above the admission
                   budget; later requests then fail while retaining the draft.
                   Input has an eight-item queue. These bounds exclude in-flight producer,
                   decoder, transcript, gateway and persistence memory. */
        }
        part <'MAR-TUI-A06'> conversationJournal {
            doc /* Session-owned journal persists start before dispatch and
                   a terminal observation separately. Replayed unfinished
                   work has no inferred owner and cannot be auto-dispatched.
                   The opt-in TUI path uses this journal before model dispatch
                   and replays completed turns. Ordered checkpoints retain a
                   prefix of observed stream output; the last uncommitted tail
                   can be lost. A typed command-line session ID selects a
                   journal in the configured project. Storage exposes a
                   project-and-agent-scoped, ID-ordered session page; the TUI
                   loads one bounded-count page asynchronously and validates
                   its order and cursor. Session switching loads and validates
                   the target asynchronously before replacing the current
                   journal and projection; failure or cancellation retains it.
                   Loading rejects projections above 10,000 turns or 64 MiB
                   accounted data. Storage pages, live history and total
                   process memory are not covered by this limit. Replay checks
                   append-only event counts before and after scanning; this
                   detects observed changes but does not create a snapshot.
                   Cancellation leaves an unfinished
                   turn. Stream-open timeout and Weaver transport failure can
                   be recorded as typed uncertainty and remain unfinished after
                   replay. Stream transport loss, timeout, invalid provider
                   response and missing terminal events are likewise unresolved;
                   invalid sequence or chunk size, byte-limit rejection and
                   inconsistent completion also lack trustworthy retirement.
                   The observed uncheckpointed prefix is saved first. Failed
                   checkpoint or terminal writes relinquish local authority,
                   including cancellation while a storage write is pending.
                   Unclassified open and stream errors remain unresolved;
                   only explicit pre-dispatch rejection releases the claim.
                   Recovery and full durability qualification are pending. */
        }
        part <'MAR-TUI-A07'> runtimeStreamOpenEdge {
            doc /* Applies the runtime-owned service-edge timeout while opening
                   a stream and exposes its typed policy receipt. The timeout
                   does not cover stream consumption, graph execution, policy
                   admission, approvals, or remote cancellation. */
        }
        part <'MAR-TUI-A08'> configuredModelSelection {
            doc /* An explicit bounded catalog maps typed aliases and exact
                   endpoint identities to configured gateways. Selection is
                   allowed only while idle; missing credentials disable a route.
                   The conversation journal records the endpoint identity.
                   Existing context is sent to the selected provider; the
                   catalog is not runtime policy or a persistent URL binding. */
        }
        part <'MAR-TUI-A09'> graphIterationCapture {
            doc /* The graph controller drains its bounded capture channel while
                   the iteration executes and adds remaining queued events after
                   completion. The primary stream still applies backpressure.
                   Iteration reports retain captured events in an unbounded Vec;
                   this is not durable evidence or a total-memory bound. */
        }
        part <'MAR-TUI-A10'> worktreeStatus {
            doc /* The terminal requests status through the Git owner on a
                   background task. NUL-delimited path bytes preserve identity;
                   terminal rendering escapes controls. A one MiB output cap,
                   10,000-entry cap and ten-second timeout reject incomplete
                   observations rather than presenting a partial result as full.
                   This is a point-in-time read, not a transactional snapshot,
                   authorization to mutate or a model conversation message. */
        }
        part <'MAR-TUI-A11'> worktreeDiff {
            doc /* A bounded background read exposes worktree-to-index and
                   index-to-HEAD separately. External diff, text conversion,
                   color and rename detection are disabled. Output bytes are
                   preserved by the Git owner and controls are escaped in the
                   terminal. Untracked files are outside both comparisons.
                   Optional UTF-8 file selection is root-relative, validated,
                   and passed as a literal Git pathspec. Non-UTF-8 file
                   selection is not yet supported. An empty result does not
                   distinguish absent, untracked and unchanged paths.
                   Results are not model context or a transactional snapshot. */
        }
        part <'MAR-TUI-A12'> workspaceProjection {
            doc /* Successful worktree observations replace a single temporary
                   view instead of accumulating full results in conversation
                   history. The draft and conversation scroll are retained;
                   Esc, Ctrl-C or submission closes the view. While a read is
                   pending, submission is deferred with input retained. Git's
                   raw output cap does not bound total rendering, temporary
                   conversion allocations or the ordinary transcript. The
                   workspace viewport traverses from the top to the requested
                   visible rows, without a preliminary full reverse scan.
                   Escaping and view construction execute on the worker. An
                   aborted worker must retire before a new workspace read is
                   admitted; synchronous conversion cannot be preempted.
                   Deep scroll, oversized graphemes, display conversion and
                   total end-to-end latency remain unqualified. */
        }
    }

    package target {
        part <'MAR-TUI-T22'> startupCoordinator {
            doc /* Target: terminal owner presents an honest, cancellable
                   initializing state immediately, admits Scheme-dependent
                   actions only after typed VM readiness, and reports startup
                   failure without losing terminal restoration. Background
                   resolution requires an SDK lifecycle that does not race
                   process environment or violate VM thread ownership. */
        }
        part <'MAR-TUI-T23'> compiledUiClosure {
            doc /* Yuanxu selects the same Module graph for a linked compiled
                   core; the SDK owns VM and native linkage. The linked Darwin
                   executable opens a Scheme UI without a package tree, and
                   the strict debug PTY startup and shutdown gate passes.
                   Release opens under the same denied Gerbil paths. An
                   explicit Release PTY gate passes, but observations range
                   from 3.90 seconds after relinking to 118-129 milliseconds
                   on warm runs. Fresh-file PTY probes on Darwin separated
                   a 2.17-2.21 second delay before first process output from
                   the 0.12-0.17 second in-app first frame. Repeated execution
                   of each same file reached first output in 8-10 milliseconds.
                   The large selected archive is a footprint candidate, while
                   the operating-system cause and cold distribution percentile
                   remain unqualified.
                   The selected compiled closure is now the default debug and
                   release TUI path; dynamic loading requires an explicit
                   no-default-features build. Optional external Scheme module
                   admission is still unqualified.
                   If residual UI operations remain slow,
                   profile the residual macro, object and generic-method
                   work and lower only admitted semantics with equivalence
                   evidence; the removed PR 17 IR is no current dependency. */
        }
        part <'MAR-TUI-T17'> schemeUiExtensionPlane {
            doc /* Selected Gerbil/Yuanxu modules own UI component composition
                   and generic-method event dispatch. The current bounded
                   extension projection admits text and field content with
                   copied strings; Scheme does not paint terminal cells.
                   Keyed local state, effects and wider widget composition
                   remain target design. A future GPUI host may consume
                   host-neutral semantics; no GPUI adapter is implemented. */
        }
        part <'MAR-TUI-T18'> typedUiBoundary {
            doc /* Proposed batched native projection validates component
                   identity, bounded values and typed intents. No per-cell
                   Scheme callbacks or text-serialized internal protocol. */
        }
        part <'MAR-TUI-T19'> ratatuiHost {
            doc /* Rust owns terminal lifecycle, input, layout, painting,
                   async I/O and routing intents through runtime authority. */
        }
        part <'MAR-TUI-T20'> schemeVmModuleAdmission {
            doc /* Proposed SDK-owned admission and invocation of typed Scheme
                   modules on the VM owner. The current pinned SDK does not yet
                   establish a general typed runtime-loaded module API. */
        }
        part <'MAR-TUI-T21'> optionalProgramArtifact {
            doc /* An AOT artifact is one optional packaging strategy. Its
                   builder and linker are not the UI protocol or FFI contract. */
        }
        part <'MAR-TUI-T16'> contextOrgAuthority {
            doc /* Org is the durable context content foundation. Context
                   composes Lab Scope and independent Session semantics with
                   current source and policy; Rust owns effect admission and
                   typed native projections. Provider continuation and turn
                   outcomes must survive replay. The TUI consumes projections,
                   not a competing context truth. A digest or parsed projection
                   alone is not a current-use grant or durable receipt. */
        }
        part <'MAR-TUI-T01'> terminalClient {
            doc /* The TUI is a projection and interaction shell. It does not
                   own model policy, execution authority or durable evidence. */
            part <'MAR-TUI-T02'> eventAdmission {
                doc /* Bound queued count and bytes, reserve control capacity,
                       propagate backpressure, and keep cancellation responsive. */
            }
            part <'MAR-TUI-T03'> projection {
                doc /* Apply ordered typed events by run identity and retain
                       terminal failure and denial evidence without reconstruction. */
            }
            part <'MAR-TUI-T04'> viewport {
                doc /* Project visible rows from versioned transcript segments;
                       bound caches, invalidate by content revision and width. */
            }
            part <'MAR-TUI-T11'> textLayout {
                doc /* Owns Unicode grapheme measurement and bounded visible-row
                       materialization independent of terminal scroll limits.
                       The renderer cannot mutate text or define history identity. */
            }
            part <'MAR-TUI-T05'> scheduler {
                doc /* Coalesce redraws after changes, avoid idle polling draws,
                       and honor deadlines for input and control events. */
            }
            part <'MAR-TUI-T06'> composer {
                doc /* Grapheme-safe input and paste with cursor-following
                       multiline viewport and displayed-row navigation. Total
                       draft UTF-8 bytes are limited; exceeding insertions are
                       rejected atomically and reported while retaining the draft.
                       ASCII rows use a specialized layout and navigation path;
                       classification and non-ASCII layout still scan the draft.
                       End-to-end latency remains unqualified. */
            }
            part <'MAR-TUI-T10'> terminalOwnership {
                doc /* One owner arbitrates input, raw mode, paste mode, child
                       handoff, suspension, resume and terminal restoration. */
            }
        }
        part <'MAR-TUI-T07'> runtime {
            doc /* Owns typed stream, model admission, tool and approval
                   authority, cancellation and terminal receipts. */
            part <'MAR-TUI-T13'> evidenceAdmission {
                doc /* Planned bounded event admission tracks both item count
                       and accounted bytes before enqueue. A durable mode must
                       commit ordered run-scoped identities before acknowledging
                       evidence and expose a typed failure or uncertain result
                       when storage cannot confirm it. Capacity pressure must
                       backpressure the producer without silently losing terminal
                       observations. The UI may coalesce display but may not
                       acknowledge durable evidence on the runtime's behalf.
                       Current graph iteration Vec capture does not implement it. */
            }
        }
        part <'MAR-TUI-T08'> sessionStore {
            doc /* Owns durable ordered events, bounded live projections,
                   replay and explicit retention. Never silently discard evidence. */
            part <'MAR-TUI-T12'> conversationJournal {
                doc /* Claims a unique turn identity before dispatch, appends
                       observed outcomes and exposes unowned incomplete work.
                       Storage uniqueness is not a process lease. Read-only
                       history pages limit their projected turn count and bytes,
                       explicitly mark oversized turns, and forbid model requests.
                       Each page scans the journal without a shared snapshot;
                       projection limits do not bound storage allocations.
                       In-memory event pages seek the ordered cursor directly;
                       this does not bound the cost of a complete replay. */
            }
        }
        part <'MAR-TUI-T09'> qualification {
            doc /* Executes matched workloads, records latency distributions,
                   CPU, allocations, RSS, I/O and correctness evidence. */
        }
        connection <'MAR-TUI-C01'> runtimeEvents connect runtime to terminalClient.eventAdmission;
        connection <'MAR-TUI-C02'> durableEvents connect runtime to sessionStore;
        dependency admissionFeedsProjection from terminalClient::eventAdmission to terminalClient::projection;
        dependency viewUsesProjection from terminalClient::viewport to terminalClient::projection;
        dependency viewUsesTextLayout from terminalClient::viewport to terminalClient::textLayout;
        dependency schedulerUsesView from terminalClient::scheduler to terminalClient::viewport;
        dependency replayReadsStore from terminalClient::projection to sessionStore;
        dependency journalUsesStore from sessionStore::conversationJournal to sessionStore;
        dependency evidenceUsesStore from runtime::evidenceAdmission to sessionStore;
        dependency qualificationMeasuresClient from qualification to terminalClient;
        dependency qualificationMeasuresRuntime from qualification to runtime;
        dependency uiHostAdmitsProjection from ratatuiHost to typedUiBoundary;
        dependency uiProjectionUsesScheme from typedUiBoundary to schemeUiExtensionPlane;
        dependency uiBoundaryUsesVmAdmission from typedUiBoundary to schemeVmModuleAdmission;
        dependency uiArtifactPackagesModule from optionalProgramArtifact to schemeUiExtensionPlane;
        dependency uiHostUsesRuntime from ratatuiHost to runtime;
        dependency qualificationMeasuresUiHost from qualification to ratatuiHost;
        dependency startupUsesVmAdmission from startupCoordinator to schemeVmModuleAdmission;
        dependency compiledClosurePackagesUi from compiledUiClosure to schemeUiExtensionPlane;
        dependency startupUsesCompiledClosure from startupCoordinator to compiledUiClosure;
        dependency qualificationMeasuresStartup from qualification to startupCoordinator;
    }

    package requirements {
        requirement <'MAR-TUI-R01'> boundedBackpressure {
            doc /* Bound count and bytes across queues and live projections.
                   No control starvation, silent terminal-event loss or
                   unbounded producer-side task accumulation. */
        }
        requirement <'MAR-TUI-R02'> responsiveIncrementalRendering {
            doc /* Idle does not redraw periodically; streamed chunks coalesce.
                   Viewport work is proportional to affected and visible data.
                   Validate under long histories, resize and burst loads. */
        }
        requirement <'MAR-TUI-R03'> authoritativeEvidence {
            doc /* Runtime and storage own durable facts; UI retains typed
                   completed, denied, failed and cancelled outcomes. */
        }
        requirement <'MAR-TUI-R04'> safeRestoration {
            doc /* Replay ordered committed events once, distinguish interrupted
                   turns and protect against stale generation and duplicate input.
                   A unique start claim is not an exactly-once delivery promise. */
        }
        requirement <'MAR-TUI-R05'> measuredPerformance {
            doc /* Match fixture, provider timing, terminal, machine and build.
                   Report distributions and failures; no unmeasured speedup claims. */
        }
        requirement <'MAR-TUI-R06'> usableEditing {
            doc /* Unicode graphemes, paste, cursor visibility, resize and
                   keyboard operation remain correct during active output.
                   Paste never dispatches its newlines as commands or turns. */
        }
        requirement <'MAR-TUI-R07'> terminalOwnership {
            doc /* Shell handoff and suspend or resume restore terminal modes,
                   preserve drafts and avoid concurrent readers or lost work. */
        }
        requirement <'MAR-TUI-R08'> evidenceDelivery {
            doc /* Evidence admission must preserve source identity and order,
                   distinguish committed, rejected and uncertain outcomes,
                   and forbid a complete replay claim when a required fact is
                   missing or its write outcome is unknown. Item and byte
                   bounds, storage failure and cancellation require explicit
                   tests, including a blocked consumer and restarted process. */
        }
        requirement <'MAR-TUI-R09'> responsiveStartup {
            doc /* Measure process start to first responsive terminal state and
                   separately to admitted Scheme UI readiness. Partition
                   spawn return, first process output, VM admission, UI
                   publication and first frame. Compare fresh-file and repeated
                   execution of one artifact with a small native control so
                   host first-execution cost is not charged to Scheme. Target subsecond
                   first interaction and readiness within a few seconds; retain
                   the existing ten-second workbench and five-second exit gates.
                   Loading, failure, cancellation and cleanup must be observable. */
        }
        requirement <'MAR-TUI-R10'> hybridExecutionEquivalence {
            doc /* A compiled core preserves selected Module identity, typed
                   contracts and dynamic extension dispatch. Any IR-lowered
                   candidate must demonstrate semantic equivalence to Scheme,
                   including rejection and boundary behavior, before adoption. */
        }
    }

    allocation queueBounds allocate requirements::boundedBackpressure to target::terminalClient.eventAdmission;
    allocation projectedBounds allocate requirements::boundedBackpressure to target::terminalClient.projection;
    allocation viewportResponsiveness allocate requirements::responsiveIncrementalRendering to target::terminalClient.viewport;
    allocation layoutResponsiveness allocate requirements::responsiveIncrementalRendering to target::terminalClient.textLayout;
    allocation drawScheduling allocate requirements::responsiveIncrementalRendering to target::terminalClient.scheduler;
    allocation runtimeAuthority allocate requirements::authoritativeEvidence to target::runtime;
    allocation durableEvidence allocate requirements::authoritativeEvidence to target::sessionStore;
    allocation replayOwnership allocate requirements::safeRestoration to target::sessionStore;
    allocation turnClaimOwnership allocate requirements::safeRestoration to target::sessionStore.conversationJournal;
    allocation measurement allocate requirements::measuredPerformance to target::qualification;
    allocation startupResponsiveness allocate requirements::responsiveStartup to target::startupCoordinator;
    allocation startupQualification allocate requirements::responsiveStartup to target::qualification;
    allocation compiledEquivalence allocate requirements::hybridExecutionEquivalence to target::compiledUiClosure;
    allocation editing allocate requirements::usableEditing to target::terminalClient.composer;
    allocation textLayout allocate requirements::usableEditing to target::terminalClient.textLayout;
    allocation ownership allocate requirements::terminalOwnership to target::terminalClient.terminalOwnership;
    allocation evidenceAdmission allocate requirements::evidenceDelivery to target::runtime.evidenceAdmission;
    allocation evidenceDurability allocate requirements::evidenceDelivery to target::sessionStore;
}

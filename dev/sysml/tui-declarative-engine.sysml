// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Target design, not a claim that the VM boundary or UI engine is implemented.
package MarlinDeclarativeUi {
    part <'MAR-UI-T01'> engine {
        doc /* Marlin's own declarative UI. Scheme authors components, state
               transitions and multi-methods using the existing POO/CLOS and
               Yuanxu Module system. Marlin Rust owns the shared typed UI
               protocol; the terminal adapter owns terminal authority.
               Reference products inform requirements, not identity,
               component structure, authority or an obligation to copy visuals. */
        part <'MAR-UI-T02'> modulePublication {
            doc /* Selected Yuanxu ui-components, ui-elements, ui-hydra, ui-modeline, ui-themes,
                   ui-layout, ui-transcript and ui Modules
                   publish the component vocabulary, command catalog, modeline
                   slots, UI scope and ordered extension contributions. UI
                   requires component, modeline, transcript, theme and layout scopes and composes
                   their constructors through selected dependencies.
                   The existing VM owner
                   resolves and retains that scope; a name or a caller-supplied
                   object alone does not establish its provenance. No separate
                   Marlin loader or duplicated module graph is introduced. */
        }
        part <'MAR-UI-T03'> functionalComposition {
            doc /* Given an admitted state, input and runtime facts, Scheme
                   produces a bounded declarative tree and typed intents.
                   Text spans, rows and columns use Scheme native values;
                   the Rust adapter admits bounded trees and maps their
                   primitives and semantic tones to terminal cells. Interactive
                   regions have unique identities and expose clipped geometry.
                   Painting and geometry traversals share the same scrolled
                   child layout and clipping, with per-region offsets; the deepest scrollable
                   region under a pointer is the scroll target. The selected
                   Scheme frame tree supplies region order and scrollability;
                   the terminal adapter uses its mapped rectangles both for
                   drawing and for pointer routing. Ordered choices select
                   bounded Scheme-authored views from named host facts; the
                   same facts select painted and interactive branches. Scheme
                   declares Mini Buffer status and search prompt precedence and
                   text; the adapter maps the editable field and cursor by cell
                   width. The terminal maps the whole selected frame tree in
                   one rendering traversal, so ordinary additional regions do
                   not require a Rust paint branch. The terminal host still
                   binds transcript, editor and list data through dedicated
                   adapters; transcript scrolling
                   retains its native deep-scroll cache. A list publishes hit
                   identities only for rows actually painted in the current
                   frame; the host validates the identity against current
                   candidates before dispatching Scheme navigation. Generic tree group
                   coordinates remain terminal-sized.
                   Components have stable domain identity and explicit state;
                   generic functions and method bundles permit specialization
                   without embedding Rust callbacks in Scheme components.
                   Forward and reverse focus traverse the same admitted order;
                   both wrap and return a focus intent without executing effects.
                   The selected Scheme modeline declares stable left and right
                   semantic slots. The host resolves live labels, cell width
                   and color without reentering the VM for every frame. The
                   selected Scheme profile also owns labels and tones for
                   runtime, workspace, recovery, continuation, history,
                   working and reading states; Rust selects those slots from
                   typed facts rather than maintaining parallel UI copy.
                   Dynamic model, turn and session slots carry one bounded
                   value placeholder; the terminal host fills it from live
                   facts and measures the resulting cell widths locally.
                   Each profile declares the separator before activity and
                   state labels; the terminal host paints the admitted text
                   without inventing a profile-specific glyph.
                   The
                   default Guide region is absent; the Scheme Help Buffer owns
                   shortcut content and M-x help selects its window binding.
                   Command browsing dispatches a bounded query to the declared
                   command component; its selection is a presentation proposal
                   checked against the host's admitted command menu. Nested
                   Buffer and theme menus display Scheme-declared prompts and
                   short candidates while retaining full admitted command
                   identities only as internal lookup keys. */
        }
        part <'MAR-UI-T15'> contextViewBoundary {
            doc /* A UI Buffer is a selected Window view, never a second
                   durable conversation or context container. Agentic AI Lab
                   Context and ORG define narrative and evidence semantics;
                   MRR is the intended evidence and storage boundary. The
                   current conversation journal only supplies provider
                   recovery and cannot impersonate a Context source. UI cells refer to
                   typed events or receipts from those owners. Switching
                   Buffer, Window, transcript fold or theme preserves those
                   identities and cannot copy or reconstruct their history.
                   Codewhale and oh-my-pi session browsers supply interaction
                   ideas, not Marlin session authority or storage semantics. */
        }
        part <'MAR-UI-T10'> portableComponentSemantics {
            doc /* Scheme owns component identity, typed props, pure state
                   transitions and generic-method event behavior. Host-neutral
                   semantics may later feed another renderer such as GPUI;
                   terminal cells, Ratatui geometry, native windows and JS
                   runtime objects stay in their respective adapters. Gambit
                   can target JavaScript, but this model does not claim that
                   Marlin's current Gerbil modules or C ABI compile to JS. */
        }
        part <'MAR-UI-T11'> sheprdAdapterBoundary {
            doc /* A future Sheprd client may depend on Marlin core's selected
                   Scheme publication and typed workbench protocol. The
                   common event/intent/successor agreement gate stays in the
                   shared Rust UI library. Sheprd owns GUI windows, input
                   mapping, geometry and painting outside Marlin core. Its
                   Gerbil package should depend on Marlin core only; Marlin
                   core must not depend on Sheprd. This is a boundary, not an
                   implemented GUI adapter or a JavaScript build claim. */
        }
        part <'MAR-UI-T04'> nativeBoundary {
            doc /* The SDK-owned VM boundary transfers rooted Scheme values to
                   Rust types in batches, with one VM owner and explicit object
                   lifetime. It is not text serialization or a per-cell API.
                   Startup resolves the selected publication and procedures via
                   the SDK's native ABI; later calls borrow roots and require
                   the selected dependency closure to remain available for
                   lazy module access. In a built development checkout, the
                   executable can restart before VM initialization with the
                   standard local package prefix when none was configured.
                   This is not module bundling; SDK-owned path lifetime remains
                   a separate integration requirement.
                   Scheme failures remain observable without publishing a
                   partial state or losing the previous session.
                   The current default debug and release TUI link the selected
                   compiled Scheme closure; explicit no-default-features is
                   the dynamic development path. Linkage is a packaging choice
                   independent of the typed C ABI contract. */
        }
        part <'MAR-UI-T05'> admittedSnapshot {
            doc /* Rust admits a complete candidate before publishing it.
                   Identity, type, size, generation, focus and effect intents
                   are checked. A rejected, failed, stale or partial candidate
                   leaves the last admitted snapshot and draft intact. */
        }
        part <'MAR-UI-T06'> incrementalView {
            doc /* Stable identities and explicit content, style and geometry
                   revisions enable subtree reuse. Resize, theme, focus, source
                   mutation and extension replacement invalidate the affected
                   views; reuse never changes the authoritative source.
                   Transcript viewport rows retain semantic role information,
                   while the selected Scheme role tone and weight are applied
                   during painting. A theme change can restyle cached rows
                   without rebuilding their geometry or deep-scroll index.
                   The Scheme main-view choice selects an admitted native
                   content slot from review, buffer, workspace and conversation
                   facts; slot bindings borrow the existing owner data and do
                   not materialize transcript history into the Scheme VM. */
        }
        part <'MAR-UI-T07'> terminalHost {
            doc /* Rust owns raw-mode lifecycle, input normalization,
                   Unicode cell geometry, Ratatui painting and output. The
                   selected Scheme frame specification owns layout choices. A
                   modeline sits above a bottom Mini Buffer. The Mini Buffer
                   displays secondary status while collapsed and expands into
                   the existing ChatBox on pointer hover or draft editing.
                   Scheme groups its interactive children into a named hover
                   region; shared layout derives that region's clipped extent
                   from its children for painting and pointer hit testing.
                   Draft data never disappears on pointer exit. Mouse motion
                   uses a latest-position observation instead of occupying
                   the bounded key queue. A visual theme is a Marlin
                   presentation choice; no reference branding is required. */
        }
        part <'MAR-UI-T12'> messageBus {
            doc /* Typed events carry source, severity, summary and detail.
                   Scheme owns one buffer catalog for identities, commands and
                   bounded informational content,
                   and selected Window in a Frame; the native host admits each
                   static buffer against a nonfocusable content component;
                   Rust retains a bounded local diagnostic route when the VM
                   itself fails. The runtime event receiver remains owned by
                   the TUI and drains in bounded batches into Messages.
                   Event publication never appends to the conversation or
                   changes its model context. Scheme declares named Buffer
                   identities and titles; a runtime owner may atomically
                   replace bounded display-only content for one of those
                   Buffers. Rejected replacements preserve its prior view.
                   A Runtime-owned managed revision and typed session identity
                   invalidate the Processes view; unchanged observations do
                   not clone or format process rows on each event-loop turn.
                   Generic Scheme event
                   subscriptions remain target design. */
        }
        part <'MAR-UI-T08'> intentRouter {
            doc /* A Scheme intent is a proposal, not an approval or execution
                   receipt. Rust validates and routes it to the owning typed
                   runtime API; only an actual runtime receipt reports outcome.
                   Scheme command descriptors declare names, argument grammar,
                   capability and fixed values, but cannot register executable
                   Rust handlers. The host admits only capabilities matching
                   an existing typed terminal effect contract. Future
                   plugin commands require a corresponding runtime-owned
                   registration, not a presentation-only declaration. */
        }
        part <'MAR-UI-T13'> foldedTranscript {
            doc /* Scheme selects compact user and assistant labels, bounded
                   thinking headings, initial fold state and M-x command.
                   Normal turns omit repetitive role headings; a folded thought
                   is one row. The native transcript keeps raw
                   thinking outside the visible row cache while collapsed;
                   a toggle selects a retained thought by newest-first ordinal
                   in constant time from one ordered fold-slot vector; raw
                   entry lookup is logarithmic in the fold count. It swaps
                   owned strings without copying the stream. Pruning old
                   entries rebases those slots before later selection.
                   Hidden chunks do not invalidate the viewport. Completion,
                   signed continuation, journal replay and Org context consume
                   the raw channel, never the summary. Folding is presentation,
                   not context compression. */
        }
        part <'MAR-UI-T14'> modelToolReview {
            doc /* A provider tool call and continuation cross as typed data.
                   The request names an advertised runtime tool and becomes a
                   distinct durable ToolRequested terminal observation before
                   the UI receives it. It never enters ordinary answer history.
                   Reopening the journal preserves the pending turn and blocks
                   another prompt. A direct model turn does not manufacture a
                   graph run, iteration or HumanGateReceipt. An actionable Ask
                   choice requires a runtime-owned pending gate bound to the
                   source turn, provider call ID, offered catalog entry and
                   execution policy. Scheme may project its Minibuffer menu;
                   only a matching typed runtime decision receipt may release
                   a registered handler. Duplicate, stale, replayed and
                   unknown-call decisions cannot authorize execution. */
        }
        part <'MAR-UI-T09'> qualification {
            doc /* Fixed fixtures compare correctness, responsiveness, CPU,
                   allocations, queue and retained memory under matched builds
                   and environments. Record failures and percentile tails; do
                   not promote a component timing into a UI latency claim.
                   Scheme tests own exact UI presentation and command catalog;
                   Rust tests own typed native admission, runtime behavior and
                   terminal interaction. Changing a Scheme glyph must not
                   require updating a duplicate Rust presentation oracle. */
        }
    }

    dependency publicationSuppliesComposition from engine::modulePublication to engine::functionalComposition;
    dependency semanticsFeedComposition from engine::portableComponentSemantics to engine::functionalComposition;
    dependency compositionCrossesBoundary from engine::functionalComposition to engine::nativeBoundary;
    dependency boundaryFeedsAdmission from engine::nativeBoundary to engine::admittedSnapshot;
    dependency snapshotFeedsView from engine::admittedSnapshot to engine::incrementalView;
    dependency snapshotFeedsSheprd from engine::admittedSnapshot to engine::sheprdAdapterBoundary;
    dependency viewFeedsTerminal from engine::incrementalView to engine::terminalHost;
    dependency snapshotRoutesIntent from engine::admittedSnapshot to engine::intentRouter;
    dependency foldUsesComposition from engine::foldedTranscript to engine::functionalComposition;
    dependency foldUsesView from engine::foldedTranscript to engine::incrementalView;
    dependency askUsesAuthority from engine::foldedTranscript to engine::intentRouter;
    dependency intentUsesRuntime from engine::intentRouter to MarlinTuiArchitecture::target::runtime;
    dependency moduleUsesVmOwner from engine::modulePublication to MarlinTuiArchitecture::target::schemeVmModuleAdmission;
    dependency contextUsesOrg from engine::functionalComposition to MarlinTuiArchitecture::target::contextOrgAuthority;
    dependency viewUsesContext from engine::contextViewBoundary to engine::functionalComposition;
    dependency contextFeedsAdmission from engine::contextViewBoundary to engine::admittedSnapshot;
    dependency qualificationMeasuresBoundary from engine::qualification to engine::nativeBoundary;
    dependency qualificationMeasuresView from engine::qualification to engine::incrementalView;
    dependency qualificationMeasuresTerminal from engine::qualification to engine::terminalHost;

    requirement <'MAR-UI-R01'> composableExtensions {
        doc /* Extensions use the existing Module lifecycle and declarative
               generic/method registration. Stable identities, deterministic
               ordering, key conflicts and focus participation are explicit;
               duplicate, cyclic, stale or invalid registrations fail visibly. */
    }
    requirement <'MAR-UI-R02'> transactionalPresentation {
        doc /* A fully validated generation replaces the previous tree as one
               publication. Late responses from retired generations cannot
               replace newer state or authorize effects. Failure preserves the
               last admitted frame, input draft and observable diagnostic. */
    }
    requirement <'MAR-UI-R03'> boundedInteractiveWork {
        doc /* Bound events and bytes before admission; configure limits for
               tree nodes, text, depth, extension count and native batch size.
               Work is incremental and cancellable; slow extensions, large
               histories, resize and streaming must not starve control input.
               A deep-scroll row index has sparse checkpoints and a fixed
               retained-byte cap. Old-entry edits and terminal width changes
               invalidate it; oversized histories use an explicit scan path.
               Short UTF-8 rows may bypass grapheme measurement only when a
               source-byte bound proves they fit, and must match the full
               renderer across Unicode and control-character fixtures.
               Measure first build separately from warm scroll latency and
               compare visible rows against a full-history reference.
               Retained, transient, VM and in-flight allocations are reported
               separately; a bound on a queue is not a bound on total memory. */
    }
    requirement <'MAR-UI-R04'> typedAuthority {
        doc /* Runtime facts and effect outcomes originate in their owners.
               No Scheme UI result manufactures an approval, tool execution,
               durable Org write, provider outcome or retirement receipt.
               A graph run suspended at a human gate remains pending and its
               exact typed gate receipt stays observable; it cannot be reported
               as completed before a matching decision is admitted. Approval
               claims the proposed or explicitly approved graph once;
               rejection ends the run; a question or patch request keeps the
               gate pending. Cancellation takes precedence over late success.
               An extension activation may propose a valid focus transition;
               only the focused composer may propose submission of its draft. */
    }
    requirement <'MAR-UI-R06'> losslessFolding {
        doc /* For every stream prefix and replayed checkpoint, toggling
               visibility preserves the exact raw thinking bytes and cannot
               change continuation verification or dispatched requests.
               A collapsed hidden append does not rebuild the visible viewport;
               fold and unfold invalidate old row geometry exactly once.
               A tool approval choice requires a matching live runtime gate
               and produces only its typed decision receipt. */
    }
    requirement <'MAR-UI-R20'> toolReviewProvenance {
        doc /* For every displayed tool Ask choice, a live runtime gate has
               an exact durable provider request and a registered handler.
               No presentation event grants authority; approval is consumed
               once against its call ID and source turn. A graph human gate
               may be used only when an actual graph run produced it. */
    }
    requirement <'MAR-UI-R22'> schemeToolGovernance {
        doc /* Yuanxu selects the Scheme tooling Module and projects bounded
               native declaration values. Rust validates names, capability,
               arguments and limits once, then binds each declaration to a
               reusable runtime handler. Scheme owns the governance choice;
               a tool request cannot execute before a matching durable user
               decision and one-shot claim. A rejected, stale or unclaimed
               request has no process effect. Execution results must be
               durable before provider continuation, and uncertain effects
               cannot be replayed as success. Tooling remains unadvertised
               until the complete Ask-to-result path is connected. */
    }
    requirement <'MAR-UI-R23'> durableToolResultBoundary {
        doc /* A claimed tool batch has one ordered set of bounded results.
               Every result retains the provider call ID and exact request
               provenance. Only the live claimant can append the result event.
               A restart after claim without results reports an unknown effect;
               a committed result reports a pending provider continuation,
               never a completed conversation answer. */
    }
    requirement <'MAR-UI-R24'> liveToolAskBinding {
        doc /* Only a handler-backed Scheme declaration may be advertised to
               the provider. A committed request binds its exact journal key
               to Scheme's finite Mini Buffer choices. The full arguments
               remain inspectable in a temporary view. Pointer selection of a
               painted choice only changes the preview; explicit confirmation
               commits one review and one effect claim on a managed Runtime
               task. A cancelled or restarted claimant cannot repeat the effect.
               Provider continuation requires its own durable transition. */
    }
    requirement <'MAR-UI-R05'> reproduciblePerformance {
        doc /* Qualification fixes workload, terminal geometry, fixture,
               revisions, build mode, machine and sampling. Measure idle and
               burst redraw, input-to-frame and chunk-to-frame tails, crossing
               count, CPU, allocations and RSS with correctness controls.
               Thresholds must be justified and recorded before comparing. */
    }
    requirement <'MAR-UI-R19'> marlinIdentity {
        doc /* Marlin's visual hierarchy, terminology and interaction serve its
               Scheme extension and agent workflows. Reference research does
               not require reproduction of another product's mascot or skin. */
    }
    requirement <'MAR-UI-R07'> vmModuleContinuity {
        doc /* The SDK owner resolves module exports using one selected package
               closure. The same VM must be able to execute those rooted
               procedures after resolution, including lazy dependencies; a
               missing dependency or caught Scheme exception is observable.
               Startup path configuration and steady-state event costs are
               measured independently. Caller-supplied roots do not by
               themselves prove selected-publication provenance. */
    }
    requirement <'MAR-UI-R08'> interactiveStartup {
        doc /* Measure cold startup through the first usable declared workbench,
               including SDK initialization, selected Module publication and
               lazy dependencies. A progress message is not a usable frame.
               The PTY fixture must observe the workbench in under ten seconds
               from process creation. Pair a newly copied executable's first
               launch with a second launch of the same file, and record first
               process output separately from the usable frame. Serialize
               competing cold-copy trials; the pre-application launch diagnostic
               includes a fresh small-executable control for host variability.
               Do not infer VM cost
               from time before application entry; measure shutdown
               separately; do not promote an extended diagnostic window into
               a passing acceptance result. The VM owner and native roots must
               retire normally; suppressing cleanup is not an optimization. */
    }
    requirement <'MAR-UI-R09'> selfContainedRelease {
        doc /* A release links the selected core Scheme Module closure and SDK
               VM into the application so it runs without a source checkout or
               a preinstalled Gerbil package tree. Optional extensions retain
               their declared Module identity and may be loaded separately.
               The linked and development paths preserve typed ABI, generic
               dispatch, failure visibility and normal VM retirement. Measure
               cold first-frame and cleanup cost under fixed workloads: static
               linking alone does not imply zero initialization cost. The
               release qualification runs from outside the source tree with
               Gerbil package paths absent and exercises the first M-x browse
               action through the linked native ABI. Yuanxu-owned publication
               and its admission evidence must remain available from the
               compiled closure without discovering package
               sources or writing a package root at runtime. Generated module
               identities and metadata must come from the same selected build
               specification; membership in a linker manifest alone does not
               prove that the selected UI can be realized. An init-only runtime
               import or a UI-family prefix selection can link a smaller
               archive yet fail at selected UI publication; every closure
               reduction must pass the source-free
               first-frame and first-input gate before adoption. */
    }
    requirement <'MAR-UI-R10'> adaptiveModelineAndMiniBuffer {
        doc /* The selected Scheme module fixes semantic segment identity,
               order, full/compact labels and idle/active color tones. A Rust terminal
               adapter resolves owner facts and paints
               one width-aware modeline directly above the Mini Buffer. The
               former top status row and default Guide sidebar consume no
               terminal area. Initial gateway readiness belongs to the
               modeline, so the transcript starts empty. Pointer hover may
               expand an empty Mini Buffer,
               and pointer exit may collapse it. Text, command editing and
               history search keep the ChatBox visible so a draft is never
               hidden or lost. Secondary host status appears in the collapsed
               Mini Buffer. High-rate motion must not fill the key queue or
               force a Scheme call for every frame. Terminals that omit motion
               or window-leave events retain keyboard access. */
    }
    requirement <'MAR-UI-R11'> reproducibleUiProperties {
        doc /* A bounded Quint transition model checks focus, layout and
               pointer/key separation. Selected Scheme POO properties cover
               modeline identity, order and host contribution admission.
               Rust properties cover typed projection and variable terminal
               geometry. A real PTY remains the evidence for terminal mode
               restoration and hover behavior; model exploration alone is
               not runtime acceptance. Seeds and bounds are fixed by the
               project property gate. */
    }
    requirement <'MAR-UI-R12'> selectedLayoutPolicy {
        doc /* The selected Scheme UI layout Module publishes distinct message
               and command frame specifications: transcript minimum, editor
               sizing, footer, extension visibility, border and prompt. The
               typed Rust boundary rejects malformed values before publication.
               Terminal measurement and clipping remain native. Pointer hit
               testing and painting consume the same admitted policy without
               calling the VM per frame. A focus successor cannot substitute
               another policy through an event. */
    }
    requirement <'MAR-UI-R13'> selectedSemanticTheme {
        doc /* A selected ui-themes Module publishes a bounded profile registry
               with one default, names, descriptions and palettes. UI composition
               derives M-x profile choices from that registry; every offered
               identity must select a Modeline and palette. Rust receives only
               the generic profile transition. Palettes use Scheme integer RGB
               triples. The UI projection admits exactly
               the declared semantic colors and valid 0..255 channels before
               publication. Ratatui applies canvas, Modeline and focus tones
               from the same snapshot; focus successors retain that theme.
               Theme choice must not alter effect authority or require a VM
               call per frame. Doom One is an attributed visual reference. */
    }
    requirement <'MAR-UI-R14'> terminalSuspendOwnership {
        doc /* Keyboard and external suspend requests cross the single Rust
               terminal owner. Before a process stop, the owner restores and
               flushes raw mode, alternate screen, paste and mouse modes.
               After SIGCONT, it rebuilds the terminal and repaints from the
               retained admitted UI snapshot and draft. A stop cannot be
               reported as a model cancellation or a completed runtime action.
               Real PTY evidence must observe stop, termios restoration,
               resume and a usable frame on both request paths. */
    }
    requirement <'MAR-UI-R15'> eventMessagesAndWindows {
        doc /* Operational event messages enter a bounded independent
               *Messages* buffer. The Mini Buffer displays a short summary;
               switch-buffer opens details or returns to the conversation.
               The same Scheme Buffer catalog supplies Frame identities and
               their M-x selection commands.
               A message cannot mutate conversation turns, context or draft.
               Repeated identical diagnostics coalesce, old records retire
               under a fixed bound, and Scheme failure still has a native
               fallback route. Buffer subscriptions are Scheme UI policy. */
    }
    requirement <'MAR-UI-R16'> schemeSelectedCommandsAndKeys {
        doc /* One selected Scheme catalog declares each M-x command name,
               annotation, argument rule, host capability and fixed value.
               Scheme also declares bounded key sequences that reference those
               commands or a finite typed native editor action. Rust admits
               capabilities and copies a bounded keymap once per UI
               publication. Scheme projects the bounded, ordered candidate
               names when the M-x query changes; native admission checks
               their identities against that publication and caches them.
               Session and history page choices are distinct Scheme-declared
               commands with fixed values admitted by the finite host
               capability contract; free-text effect suffixes are not parsed.
               A Scheme Hydra declaration owns its command descriptors, heads,
               hint and foreign-key policy. The native host admits the bounded
               projection and routes repeated heads without a VM call per key.
               Exit preserves the composer draft; a passed foreign key is
               delivered once to the enclosing keymap or composer.
               Repaint does not filter candidates or call the VM. Ordinary
               key input uses the native keymap. The first Up or Down after
               opening or filtering M-x must move from the visibly selected
               candidate even when opening, query edit and first arrow arrive
               in one terminal write,
               before the menu's initial frame. The native terminal boundary
               accepts plain CSI, SS3 application-cursor and explicitly
               unmodified CSI arrow sequences; none may be discarded before
               Scheme-owned browsing. Modified arrows retain their distinct
               identity and are not silently treated as plain arrows.
               Native lookup and transition
               admission have a maintained benchmark; a real PTY test checks
               the first navigation, query projection, redraw latency and terminal
               mode restoration. */
    }
    requirement <'MAR-UI-R17'> schemeOrderedModeline {
        doc /* The selected Scheme Profile owns the relative order of
               Modeline contributions on each side, bounded minimum and
               compact-width thresholds, and optional retention priority.
               Rust admits the
               same bounded role identity set and unchanged host status facts,
               then resolves live facts, terminal widths and Ratatui cells
               without a Scheme call per frame. A profile transition may
               change order, width and priority policy, palette, labels and
               tones but cannot create a
               new host fact or change runtime effect authority. Both native
               admission controls and a linked narrow-terminal profile switch
               must verify the different retained segments and this contract. */
    }
    requirement <'MAR-UI-R18'> coalescedPointerScroll {
        doc /* Terminal SGR wheel and touchpad pulses scroll the currently
               displayed transcript, workspace or Messages buffer without
               a keyboard focus command. Pointer motion remains outside the
               bounded key queue. Rapid wheel pulses coalesce into one wake
               and a capped per-frame scroll delta, preserving keyboard
               input capacity and responsive redraw. A real PTY test must
               observe visible movement in both directions. In M-x, the
               same bounded gesture advances Scheme-ordered candidates in
               one native ABI call rather than scrolling hidden content or
               calling the VM for every pulse. */
    }
    requirement <'MAR-UI-R21'> verticoCommandCompletion {
        doc /* M-x completion uses one Scheme-owned candidate order for the
               visible list, first selection and directional navigation.
               A pointer selection uses the painted row identity and the same
               Scheme navigation; it previews a buffer without confirming it.
               Stale or hidden rows cannot select a different current command.
               Accepted command identities enter bounded history; cancelled,
               unknown or incomplete input and command arguments do not.
               Exact fields outrank history, recent commands outrank unseen
               commands, repeated uses receive a decaying boost, and ties
               have deterministic length and lexical order. A selected
               candidate remains selected across filtering when still present.
               Candidate count and scroll margin stay bounded independently of
               catalog size. Optional Rust-owned storage may restore bounded
               command identities across launches without saving arguments,
               prompts or serializing the native Scheme API. Different completion
               categories retain separate histories and may select an explicit
               category ordering. Selected and matched text remain visually
               distinct while descriptions stay aligned and clipped safely.
               Scheme properties must verify sorting, history limits and
               filtering; linked PTY checks must verify real selection,
               navigation, cancellation, query edits and window geometry.
               Measure first open, query edits and navigation separately from
               VM startup, including tail latency and large bounded catalogs. */
    }

    allocation extensions allocate composableExtensions to engine.modulePublication;
    allocation specialization allocate composableExtensions to engine.functionalComposition;
    allocation admission allocate transactionalPresentation to engine.admittedSnapshot;
    allocation incremental allocate boundedInteractiveWork to engine.incrementalView;
    allocation boundedBoundary allocate boundedInteractiveWork to engine.nativeBoundary;
    allocation authority allocate typedAuthority to engine.intentRouter;
    allocation measurements allocate reproduciblePerformance to engine.qualification;
    allocation visualIdentity allocate marlinIdentity to engine.terminalHost;
    allocation vmContinuity allocate vmModuleContinuity to engine.nativeBoundary;
    allocation startupQualification allocate interactiveStartup to engine.qualification;
    allocation releaseBoundary allocate selfContainedRelease to engine.nativeBoundary;
    allocation modelineSemantics allocate adaptiveModelineAndMiniBuffer to engine.modulePublication;
    allocation miniBufferGeometry allocate adaptiveModelineAndMiniBuffer to engine.terminalHost;
    allocation uiPropertyQualification allocate reproducibleUiProperties to engine.qualification;
    allocation selectedGeometry allocate selectedLayoutPolicy to engine.modulePublication;
    allocation clippedGeometry allocate selectedLayoutPolicy to engine.terminalHost;
    allocation selectedPalette allocate selectedSemanticTheme to engine.modulePublication;
    allocation paintedPalette allocate selectedSemanticTheme to engine.terminalHost;
    allocation suspendBoundary allocate terminalSuspendOwnership to engine.terminalHost;
    allocation suspendEvidence allocate terminalSuspendOwnership to engine.qualification;
    allocation eventRouting allocate eventMessagesAndWindows to engine.messageBus;
    allocation messageWindow allocate eventMessagesAndWindows to engine.terminalHost;
    allocation commandPublication allocate schemeSelectedCommandsAndKeys to engine.modulePublication;
    allocation keyAdmission allocate schemeSelectedCommandsAndKeys to engine.nativeBoundary;
    allocation keyPerformance allocate schemeSelectedCommandsAndKeys to engine.qualification;
    allocation modelineOrder allocate schemeOrderedModeline to engine.modulePublication;
    allocation modelineOrderAdmission allocate schemeOrderedModeline to engine.nativeBoundary;
    allocation modelineOrderEvidence allocate schemeOrderedModeline to engine.qualification;
    allocation pointerDecode allocate coalescedPointerScroll to engine.terminalHost;
    allocation pointerBound allocate coalescedPointerScroll to engine.incrementalView;
    allocation pointerEvidence allocate coalescedPointerScroll to engine.qualification;
    allocation commandRanking allocate verticoCommandCompletion to engine.modulePublication;
    allocation rankedInput allocate verticoCommandCompletion to engine.nativeBoundary;
    allocation rankingEvidence allocate verticoCommandCompletion to engine.qualification;

    attribute def BeginUpdate;
    attribute def CandidateReady;
    attribute def Validated;
    attribute def Rejected;
    attribute def Superseded;
    attribute def OwnerRetired;
    state def <'MAR-UI-L01'> SnapshotLifecycle {
        doc /* The publication of one candidate is distinct from VM execution,
               effect authorization and terminal painting. Supersession and
               retirement discard the candidate, not the prior live snapshot. */
        first start then stable;
        state stable;
        state computing;
        state validating;
        state discarded;
        state closed;
        transition begin first stable accept BeginUpdate then computing;
        transition candidate first computing accept CandidateReady then validating;
        transition publish first validating accept Validated then stable;
        transition failedComputation first computing accept Rejected then discarded;
        transition invalid first validating accept Rejected then discarded;
        transition staleComputing first computing accept Superseded then discarded;
        transition staleValidating first validating accept Superseded then discarded;
        transition retain first discarded then stable;
        transition retireStable first stable accept OwnerRetired then closed;
        transition retireComputing first computing accept OwnerRetired then closed;
        transition retireValidating first validating accept OwnerRetired then closed;
        transition retireDiscarded first discarded accept OwnerRetired then closed;
    }
    dependency lifecycleGovernsAdmission from SnapshotLifecycle to engine::admittedSnapshot;
}

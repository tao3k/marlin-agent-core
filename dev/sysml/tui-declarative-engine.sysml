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
            doc /* Selected Yuanxu ui-components, ui-modeline and ui Modules
                   publish the component vocabulary, command catalog, modeline
                   slots, UI scope and ordered extension contributions. UI
                   requires both component and modeline scopes and composes
                   their constructors through selected dependencies.
                   The existing VM owner
                   resolves and retains that scope; a name or a caller-supplied
                   object alone does not establish its provenance. No separate
                   Marlin loader or duplicated module graph is introduced. */
        }
        part <'MAR-UI-T03'> functionalComposition {
            doc /* Given an admitted state, input and runtime facts, Scheme
                   produces a bounded declarative tree and typed intents.
                   Components have stable domain identity and explicit state;
                   generic functions and method bundles permit specialization
                   without embedding Rust callbacks in Scheme components.
                   Forward and reverse focus traverse the same admitted order;
                   both wrap and return a focus intent without executing effects.
                   The selected Scheme modeline declares stable left and right
                   semantic slots. The host resolves live labels, cell width
                   and color without reentering the VM for every frame. The
                   default Guide region is absent; /help owns shortcut help.
                   Command browsing dispatches a bounded query to the declared
                   command component; its selection is a presentation proposal
                   checked against the host's admitted command menu. */
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
                   views; reuse never changes the authoritative source. */
        }
        part <'MAR-UI-T07'> terminalHost {
            doc /* Rust owns raw-mode lifecycle, input normalization, layout,
                   Unicode cell geometry, Ratatui painting and output. A
                   modeline sits above a bottom Mini Buffer. The Mini Buffer
                   displays secondary status while collapsed and expands into
                   the existing ChatBox on pointer hover or draft editing.
                   Draft data never disappears on pointer exit. Mouse motion
                   uses a latest-position observation instead of occupying
                   the bounded key queue. A visual theme is a Marlin
                   presentation choice; no reference branding is required. */
        }
        part <'MAR-UI-T08'> intentRouter {
            doc /* A Scheme intent is a proposal, not an approval or execution
                   receipt. Rust validates and routes it to the owning typed
                   runtime API; only an actual runtime receipt reports outcome.
                   Bounded Scheme command descriptors describe discovery and
                   completion; they do not register executable Rust handlers.
                   The host admits only descriptions matching an existing
                   typed terminal command and its argument contract. Future
                   plugin commands require a corresponding runtime-owned
                   registration, not a presentation-only declaration. */
        }
        part <'MAR-UI-T09'> qualification {
            doc /* Fixed fixtures compare correctness, responsiveness, CPU,
                   allocations, queue and retained memory under matched builds
                   and environments. Record failures and percentile tails; do
                   not promote a component timing into a UI latency claim. */
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
    dependency intentUsesRuntime from engine::intentRouter to MarlinTuiArchitecture::target::runtime;
    dependency moduleUsesVmOwner from engine::modulePublication to MarlinTuiArchitecture::target::schemeVmModuleAdmission;
    dependency contextUsesOrg from engine::functionalComposition to MarlinTuiArchitecture::target::contextOrgAuthority;
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
               Retained, transient, VM and in-flight allocations are reported
               separately; a bound on a queue is not a bound on total memory. */
    }
    requirement <'MAR-UI-R04'> typedAuthority {
        doc /* Runtime facts and effect outcomes originate in their owners.
               No Scheme UI result manufactures an approval, tool execution,
               durable Org write, provider outcome or retirement receipt.
               An extension activation may propose a valid focus transition;
               only the focused composer may propose submission of its draft. */
    }
    requirement <'MAR-UI-R05'> reproduciblePerformance {
        doc /* Qualification fixes workload, terminal geometry, fixture,
               revisions, build mode, machine and sampling. Measure idle and
               burst redraw, input-to-frame and chunk-to-frame tails, crossing
               count, CPU, allocations and RSS with correctness controls.
               Thresholds must be justified and recorded before comparing. */
    }
    requirement <'MAR-UI-R06'> marlinIdentity {
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
               from process creation; measure shutdown
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
               Yuanxu-owned publication and its admission evidence must remain
               available from the compiled closure without discovering package
               sources or writing a package root at runtime. Generated module
               identities and metadata must come from the same selected build
               specification; membership in a linker manifest alone does not
               prove that the selected UI can be realized. */
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

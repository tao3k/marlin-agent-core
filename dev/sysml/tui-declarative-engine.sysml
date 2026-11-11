// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Target design, not a claim that the VM boundary or UI engine is implemented.
package MarlinDeclarativeUi {
    part <'MAR-UI-T01'> engine {
        doc /* Marlin's own declarative UI. Scheme authors components, state
               transitions and multi-methods using the existing POO/CLOS and
               Yuanxu Module system. Rust owns the terminal and typed runtime
               authority. Reference products inform requirements, not identity,
               component structure, authority or an obligation to copy visuals. */
        part <'MAR-UI-T02'> modulePublication {
            doc /* A selected Yuanxu Module publishes a UI scope and explicitly
                   ordered extension contributions. The existing VM owner
                   resolves and retains that scope; a name or a caller-supplied
                   object alone does not establish its provenance. No separate
                   Marlin loader or duplicated module graph is introduced. */
        }
        part <'MAR-UI-T03'> functionalComposition {
            doc /* Given an admitted state, input and runtime facts, Scheme
                   produces a bounded declarative tree and typed intents.
                   Components have stable domain identity and explicit state;
                   generic functions and method bundles permit specialization
                   without embedding Rust callbacks in Scheme components. */
        }
        part <'MAR-UI-T04'> nativeBoundary {
            doc /* The SDK-owned VM boundary transfers rooted Scheme values to
                   Rust types in batches, with one VM owner and explicit object
                   lifetime. It is not text serialization or a per-cell API.
                   Startup resolves the selected publication and procedures via
                   the SDK's native ABI; later calls borrow roots and retain
                   the resolved dependency closure for lazy module access.
                   Scheme failures remain observable without publishing a
                   partial state or losing the previous session.
                   AOT is optional packaging, not a prerequisite implied by
                   the C ABI; precompiled Scheme on the VM remains a mode. */
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
                   visual theme is a Marlin presentation choice; no whale
                   mascot, reference layout or branding is required. */
        }
        part <'MAR-UI-T08'> intentRouter {
            doc /* A Scheme intent is a proposal, not an approval or execution
                   receipt. Rust validates and routes it to the owning typed
                   runtime API; only an actual runtime receipt reports outcome. */
        }
        part <'MAR-UI-T09'> qualification {
            doc /* Fixed fixtures compare correctness, responsiveness, CPU,
                   allocations, queue and retained memory under matched builds
                   and environments. Record failures and percentile tails; do
                   not promote a component timing into a UI latency claim. */
        }
    }

    dependency publicationSuppliesComposition from engine::modulePublication to engine::functionalComposition;
    dependency compositionCrossesBoundary from engine::functionalComposition to engine::nativeBoundary;
    dependency boundaryFeedsAdmission from engine::nativeBoundary to engine::admittedSnapshot;
    dependency snapshotFeedsView from engine::admittedSnapshot to engine::incrementalView;
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
               durable Org write, provider outcome or retirement receipt. */
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

    allocation extensions allocate composableExtensions to engine.modulePublication;
    allocation specialization allocate composableExtensions to engine.functionalComposition;
    allocation admission allocate transactionalPresentation to engine.admittedSnapshot;
    allocation incremental allocate boundedInteractiveWork to engine.incrementalView;
    allocation boundedBoundary allocate boundedInteractiveWork to engine.nativeBoundary;
    allocation authority allocate typedAuthority to engine.intentRouter;
    allocation measurements allocate reproduciblePerformance to engine.qualification;
    allocation visualIdentity allocate marlinIdentity to engine.terminalHost;
    allocation vmContinuity allocate vmModuleContinuity to engine.nativeBoundary;

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

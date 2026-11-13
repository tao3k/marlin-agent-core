// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

package MarlinRustPackageOwnership {
    part <'MAR-PKG-A01'> packages {
        part protocol {
            doc /* marlin-agent-protocol: in-process Rust contracts for graph execution,
                   model requests and gateway ports, hooks, process control,
                   Context queries, environment records, events and evidence. */
        }
        part appServer {
            doc /* marlin-agent-app-server: marlin.app.v1 Protobuf schema and gRPC
                   service. Translates wire messages to Session and Runtime calls. */
        }
        part runtime { doc /* marlin-agent-runtime: Tokio scheduling, atomic admission, resource identities, cancellation and process/pipe reaping. */ }
        part telemetry { doc /* marlin-agent-telemetry: local host/process sampling, independent watchdog evidence and bounded OpenTelemetry export. */ }
        part tui { doc /* marlin-agent-tui: terminal I/O, input/render loop, application wiring and presentation. */ }
        part ui { doc /* marlin-agent-ui: scheme-ui enables typed Scheme projection. The optional grpc feature exposes generated client bindings. */ }
        part core { doc /* marlin-agent-core: public facade and readiness-aware composition of agent services. */ }
        part kernel { doc /* marlin-agent-kernel: graph-loop transitions, tool requests and effect execution via Runtime. */ }
        part hooks { doc /* marlin-agent-hooks: handler catalog, typed registration and dispatch. */ }
        part graph { doc /* marlin-agent-graph: graph topology and coordination types. */ }
        part sessions { doc /* marlin-agent-sessions: identity, visibility, isolation, turn journal and submission recovery. */ }
        part storage { doc /* marlin-agent-storage: typed persistence ports and enabled backend adapters. */ }
        part environment {
            doc /* marlin-agent-environment: resolve home and configuration layers,
                   execute working-copy commands, and take the applied Worktrunk
                   path and branch from the provider's structured output.
                   Git switch and branch finalization use a worktree root
                   verified against the selected repository's common directory.
                   Worktrunk PR checkout verifies the requested GitHub repository
                   against forge configuration, fetch URLs and gh default. */
        }
        part stream { doc /* marlin-agent-stream: adapt provider gateway responses to typed model streams. */ }
        part context { doc /* marlin-agent-context: Context admission and scope, Org loading and document, query, view and status projection. */ }
        part contextOrg { doc /* marlin-context-org-model: Org identity and workspace data model. */ }
        part gitOps {
            doc /* marlin-agent-gitops: bounded Git repository discovery, status
                   and diff, and typed GitWorktree and Worktrunk plans and
                   receipts. Git checkout mutations disable repository hooks
                   in the projected command. */
        }
        part workspaceQuery { doc /* marlin-workspace-query: workspace selectors and query records. */ }
        part workspaceStatus { doc /* marlin-workspace-status: workspace status records. */ }
        part workspaceView { doc /* marlin-workspace-view: workspace presentation contracts. */ }
        part gerbilIr { doc /* marlin-gerbil-ir: typed artifacts exchanged with the Gerbil extension plane. */ }
        part gerbilScheme {
            doc /* marlin-gerbil-scheme: Scheme bindings and typed data conversion.
                   The published working-copy policy selects a provider.
                   Rust checks availability and projects the selection into GitOps. */
        }
        part policyRoutingAbi { doc /* marlin-agent-policy-routing-abi: typed calls to the linked Scheme AgentGraph selector. */ }
        part deck { doc /* marlin-deck-runtime: ownership of the embedded VM and Deck Actor requests and receipts. */ }
        part harness { doc /* marlin-agent-harness: scenario execution and evidence validation. */ }
        part harnessTypes { doc /* marlin-agent-harness-types: harness evidence contracts. */ }
        part testSupport { doc /* marlin-agent-test-support: shared fixtures and test doubles. */ }
        part aotLink { doc /* marlin-gerbil-aot-link: build-time native linkage for selected Scheme programmes. */ }
        part devPolicy { doc /* marlin-rust-project-harness-policy: Cargo development-only ASP Rust policy. */ }
        part bazelResolution { doc /* marlin-bazel-scenario-resolution: build-only Cargo feature resolution for Bazel. */ }
    }

    part <'MAR-PKG-A02'> scheme {
        doc /* The selected Gerbil Module owns extension policy, semantic
               composition and UI layout. Existing native adapters under
               scheme/c-abi receive Rust-owned typed values and return typed
               proposals. Rust checks native ABI readiness and admits effects. */
    }

    dependency runtimeUsesPorts from packages::runtime to packages::protocol;
    dependency appServerUsesModelContracts from packages::appServer to packages::protocol;
    dependency appServerUsesSessionJournal from packages::appServer to packages::sessions;
    dependency appServerUsesRuntime from packages::appServer to packages::runtime;
    dependency uiUsesAppServerClient from packages::ui to packages::appServer;
    dependency environmentExecutesWorkingCopyPlans from packages::environment to packages::gitOps;
    dependency contextUsesQueryRecords from packages::context to packages::workspaceQuery;
    dependency contextUsesStatusRecords from packages::context to packages::workspaceStatus;
    dependency contextUsesViewRecords from packages::context to packages::workspaceView;
    dependency telemetryImplementsSampling from packages::telemetry to packages::protocol;
    dependency tuiComposesRuntime from packages::tui to packages::runtime;
    dependency tuiComposesTelemetry from packages::tui to packages::telemetry;
    dependency tuiUsesUiProjection from packages::tui to packages::ui;
    dependency kernelSubmitsEffects from packages::kernel to packages::runtime;
    dependency gitOpsUsesProcessPort from packages::gitOps to packages::protocol;
    dependency uiProjectsNativeFacts from packages::ui to scheme;
    dependency routingSubmitsNativeInputs from packages::policyRoutingAbi to scheme;
    dependency deckSubmitsNativeRequests from packages::deck to scheme;

    requirement <'MAR-PKG-R01'> observationAndAuthority {
        doc /* Local samples cross the protocol port without involving an OTLP
               collector. Runtime owns admission and cleanup. Telemetry can
               publish evidence with no Runtime dependency or cleanup handle. */
    }
    requirement <'MAR-PKG-R02'> typedNativeProjection {
        doc /* Rust sends the Scheme Module the typed domain inputs and the UI
               facts needed by its selected publication. Native projection uses
               Scheme objects, symbols and vectors. Host samples, budgets,
               resource handles and exporter settings stay within their Rust
               owners. Scheme owns presentation and returns typed policy proposals. */
    }
    allocation ownResourceDecisions allocate observationAndAuthority to packages.runtime;
    allocation ownObservation allocate observationAndAuthority to packages.telemetry;
    allocation projectHostFacts allocate typedNativeProjection to packages.ui;
    allocation routePolicyFacts allocate typedNativeProjection to packages.policyRoutingAbi;

    requirement <'MAR-PKG-R03'> processConsole {
        doc /* The selected Scheme publication declares the process buffer and
               M-x commands. The terminal displays Runtime observations and
               submits a displayed generation for the current session.
               Runtime resolves the full resource identity, not the PID, and
               wakes its managed owner to run the existing reaper. */
    }
    allocation authorizeProcessControl allocate processConsole to packages.runtime;
    allocation presentProcessControl allocate processConsole to packages.tui;
    allocation declareProcessCommands allocate processConsole to scheme;

    requirement <'MAR-PKG-R04'> workingCopyBinding {
        doc /* The working-copy Module selects a provider from the caller's
               preference and observed availability. Rust executes the typed
               plan. For a Worktrunk create, the active binding carries the
               provider's registered path and branch. Remove consumes that
               path and closes the binding only after successful removal. */
    }
    allocation selectWorkingCopyProvider allocate workingCopyBinding to packages.gerbilScheme;
    allocation projectWorkingCopyPlan allocate workingCopyBinding to packages.gitOps;
    allocation applyWorkingCopyReceipt allocate workingCopyBinding to packages.environment;
}

// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// Structural model for Marlin Plugin Contract v0.
// Rationale and scope: ../../docs/40-rfcs/40.166-plugin-contract.org::#plugin-model
package MarlinPluginEngine {
    doc /* Viewpoint: component ownership and transport boundaries.
           Actual records source-inspected interfaces. Target records proposed
           plugin integration. Dependencies point from consumer to supplier. */

    part def RustRuntime {
        doc /* Host authority for run lifecycle, message ordering, cancellation,
               and effects. Tokio schedules Rust work. */
    }
    part def SchemeCAbiBridge {
        doc /* Projects typed requests and results across the C ABI. Rust
               receives owned results. */
    }
    part def SchemeVm {
        doc /* Owns its heap and schedules policy evaluation with Gerbil
               Actors and green threads. */
    }
    part def PolicyProtocol {
        doc /* Typed policy packs, loop programs, graph proposals, and
               execution receipts. */
    }

    package actual {
        part <'MAR-PLG-A01'> runtime : RustRuntime {
            doc /* crates/marlin-agent-runtime/src/graph_loop.rs */
        }
        part <'MAR-PLG-A02'> schemeCAbiBridge : SchemeCAbiBridge {
            doc /* Rust bindings are in crates/marlin-gerbil-scheme.
                   Scheme link units are in scheme/c-abi. */
        }
        part <'MAR-PLG-A03'> policyProtocol : PolicyProtocol {
            doc /* crates/marlin-agent-protocol/src/loop_policy_ir.rs */
        }
        part <'MAR-PLG-A04'> graphAdmission {
            doc /* crates/marlin-agent-kernel/src/driver.rs owns graph compilation with ABI readiness checks. */
        }
        part <'MAR-PLG-A05'> schemePackage {
            doc /* scheme/init.ss selects Context with the causal Feature, Hooks, Policy and Deck Runtime.
                   scheme/context imports Lab's installed Scope library.
                   Lab owns Scope validation and Session semantics.
                   Context's causal Feature uses Lab's ASCENT exchange for solve and replay.
                   scheme/build.ss compiles Marlin's selected modules.
                   Policy declares a dependency on Hooks.
                   Deck Runtime depends on Policy and publishes its version in Scheme metadata.
                   Resident mailbox actors evaluate ordered batches on Gerbil green threads.
                   scheme/policy/projection shapes evaluated policy results. */
        }
        part <'MAR-PLG-A06'> contextAdapter {
            doc /* crates/marlin-agent-context owns typed Context inputs,
                   batch bounds and owned Lab digest projections.
                   The Gerbil feature binds an owner-local export.
                   Production VM image integration is pending. */
        }
        part <'MAR-PLG-A07'> hooks {
            doc /* crates/marlin-agent-hooks owns matcher compilation,
                   selection, catalog admission, execution and receipts.
                   scheme/hooks publishes the Scheme selector and its higher-order matchers. */
        }
        part <'MAR-PLG-A08'> deckRuntimeHost {
            doc /* crates/marlin-deck-runtime owns Deck metadata and typed
                   Context admission. A bounded Tokio queue feeds one linked VM
                   owner. Ready Context rows enter resident Scheme green actors;
                   the selected Lab operation returns ordered Scope digests.
                   Single-VM ownership and shutdown are detailed in
                   MarlinSingleVmRuntime. Local Context admission is verified. */
        }
        part <'MAR-PLG-A09'> workflowBundleAdmission {
            doc /* crates/marlin-agent-runtime/src/plugin owns signed TOML
                   verification, host-provisioned workflow registration, project
                   grant checks and generation-scoped effect admission. Grant
                   replacement and offline replay both check every granted
                   capability against the installed signed declaration. Closing
                   a generation retains outcomes for admitted effects.
                   Brief-to-Draft execution requires a live generation for
                   its registered policy module. Offline graph replay checks
                   the policy module, schemas and registered file capabilities
                   declared by the signed workflow.
                   The Brief-to-Draft host test binds the init-selected
                   settings policy through a linked Scheme object export.
                   The workspace broker resolves the registered file
                   capabilities, denies symlink traversal and overwrite,
                   and records digests for committed effects. Closed effect
                   evidence verifies against the trusted release and project
                   digest with the worktree removed. Graph replay checks the
                   selected template and audience requirement against the
                   admitted settings digest, then checks committed draft
                   bytes against the Rust renderer and recorded Scheme facts.
                   The linked fixture
                   projects Org facts and executor kinds through Scheme,
                   crosses Rust ABI readiness and runs render and create on
                   the graph kernel. Offline verification binds the ordered
                   node receipts and terminal result to the file effects.
                   A create refused before admission retains its typed host
                   denial with no fabricated file effect. */
        }

        dependency admissionUsesProtocol from graphAdmission to policyProtocol;
        dependency bridgeUsesProtocol from schemeCAbiBridge to policyProtocol;
        dependency bridgeUsesSchemePackage from schemeCAbiBridge to schemePackage;
        dependency contextUsesSchemeBridge from contextAdapter to schemeCAbiBridge;
        dependency deckRuntimeUsesSchemeBridge from deckRuntimeHost to schemeCAbiBridge;
        dependency workflowAdmissionUsesRuntime from workflowBundleAdmission to runtime;
    }

    package target {
        part <'MAR-PLG-T01'> pluginEngine {
            part <'MAR-PLG-T02'> grpcService {
                doc /* Engine-owned listener and Protobuf boundary. Agent
                       Calls decode to Rust-owned requests. */
            }
            part <'MAR-PLG-T03'> runtime : RustRuntime {
                doc /* Owns the execution path and bounded message channels. */
                part <'MAR-PLG-T12'> compiledExecution {
                    doc /* Rust runs admitted LoopProgram transitions over
                           immutable policy tables. Normal dispatch, streaming,
                           and cancellation stay on the Rust execution path. */
                }
                part <'MAR-PLG-T13'> messageChannels {
                    doc /* Rust owns admission, ordering, backpressure, and
                           retirement. In-flight Scheme work, ready results,
                           and client delivery have separate capacity bounds. */
                }
                part <'MAR-PLG-T14'> deckRuntimeHost {
                    doc /* Uses the gerbil-scheme-rust bounded service ingress
                           to submit ready batches on their Scheme VM owner.
                           Tokio drives admission, deadlines, cancellation,
                           and reply delivery. */
                }
            }
            part <'MAR-PLG-T04'> bundleRegistry {
                doc /* Verifies local release artifacts, manifest shape, and
                       catalog references. Stores verified bundle identity. */
            }
            part <'MAR-PLG-T05'> activationGate {
                doc /* Binds the project grant, policy epoch, and generation.
                       Rechecks live authority before admitting effects. */
            }
            part <'MAR-PLG-T06'> schemeCAbiBridge : SchemeCAbiBridge {
                doc /* Trusted Scheme inputs and projections cross here.
                       Graph proposals from Scheme require ABI readiness. Policy
                       composition occurs at activation or planning. Resident
                       Scheme computation uses bounded batches when needed. */
            }
            part <'MAR-PLG-T07'> evidence {
                doc /* Retains typed outcomes and their provenance. Supplies
                       bounded event projections and offline evidence. */
            }
        }
        part <'MAR-PLG-T08'> clientShell {
            doc /* Marlin-maintained client transport and scoped panel bridge.
                   Browser transport can use a gRPC-Web gateway. */
        }
        part <'MAR-PLG-T09'> tsPanel {
            doc /* Optional client asset. v0 observes scoped workflow events. */
        }
        part <'MAR-PLG-T10'> schemeVm : SchemeVm {
            doc /* Executes trusted, host-provisioned AI Agentic Lib and
                   Gerbil POO policy on the selected VM owner. */
        }
        part <'MAR-PLG-T11'> offlineVerifier {
            doc /* Reads authenticated retained evidence and checks identity,
                   linkage, and completeness. */
        }

        connection <'MAR-PLG-C01'> scopedPanelChannel connect tsPanel to clientShell;
        connection <'MAR-PLG-C02'> grpcChannel connect clientShell to pluginEngine.grpcService;
        connection <'MAR-PLG-C03'> schemeCAbiChannel connect pluginEngine.schemeCAbiBridge to schemeVm;
        dependency grpcUsesRuntime from pluginEngine::grpcService to pluginEngine::runtime;
        dependency runtimeUsesActivation from pluginEngine::runtime to pluginEngine::activationGate;
        dependency runtimeUsesEvidence from pluginEngine::runtime to pluginEngine::evidence;
        dependency deckRuntimeUsesCAbi from pluginEngine::runtime::deckRuntimeHost to pluginEngine::schemeCAbiBridge;
        dependency activationUsesBundle from pluginEngine::activationGate to pluginEngine::bundleRegistry;
        dependency activationUsesPolicy from pluginEngine::activationGate to pluginEngine::schemeCAbiBridge;
        dependency verifierUsesEvidence from offlineVerifier to pluginEngine::evidence;
    }

    // Planned reuse of the existing Rust protocol by the target execution path.
    dependency compiledExecutionUsesProtocol from target::pluginEngine::runtime::compiledExecution to actual::policyProtocol;

    package requirements {
        requirement <'MAR-PLG-R01'> rustAuthority {
            doc /* Rust owns lifecycle, message management, and effect admission. */
        }
        requirement <'MAR-PLG-R02'> schemeCAbi {
            doc /* In-process exchange is exclusively typed C ABI arguments,
                   owner-local Scheme objects, and owned Rust projections. */
        }
        requirement <'MAR-PLG-R03'> scopedFrontend {
            doc /* The engine serves gRPC. The optional frontend gets a scoped
                   client projection through Marlin's transport binding. */
        }
        requirement <'MAR-PLG-R04'> replayEvidence {
            doc /* An offline verifier can inspect retained typed evidence. */
        }
        requirement <'MAR-PLG-R05'> highPerformanceExecution {
            doc /* Execute ordinary transitions in Rust against compiled
                   policy. Amortize C ABI calls at planning boundaries.
                   Bound queue growth and decouple client delivery from
                   execution. Reuse gRPC channels at the external boundary. */
        }
    }

    // Planned allocations express ownership of the requirement.
    allocation runtimeAuthority allocate requirements::rustAuthority to target::pluginEngine.runtime;
    allocation gateAuthority allocate requirements::rustAuthority to target::pluginEngine.activationGate;
    allocation policyInterface allocate requirements::schemeCAbi to target::pluginEngine.schemeCAbiBridge;
    allocation grpcInterface allocate requirements::scopedFrontend to target::pluginEngine.grpcService;
    allocation frontendProjection allocate requirements::scopedFrontend to target::clientShell;
    allocation retainedEvidence allocate requirements::replayEvidence to target::pluginEngine.evidence;
    allocation replayInspection allocate requirements::replayEvidence to target::offlineVerifier;
    allocation hotExecution allocate requirements::highPerformanceExecution to target::pluginEngine.runtime.compiledExecution;
    allocation boundedMessages allocate requirements::highPerformanceExecution to target::pluginEngine.runtime.messageChannels;
    allocation schemeBatchProjection allocate requirements::highPerformanceExecution to target::pluginEngine.schemeCAbiBridge;
    allocation externalChannelReuse allocate requirements::highPerformanceExecution to target::pluginEngine.grpcService;
}

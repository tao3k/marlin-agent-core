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
            doc /* scheme/init.ss selects the session and policy module
                   publications and exports the Gerbil build specification.
                   scheme/policy/projection shapes policy results. */
        }

        dependency admissionUsesProtocol from graphAdmission to policyProtocol;
        dependency bridgeUsesProtocol from schemeCAbiBridge to policyProtocol;
        dependency bridgeUsesSchemePackage from schemeCAbiBridge to schemePackage;
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
            doc /* Trusted Scheme exchanges typed values through the C ABI. */
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

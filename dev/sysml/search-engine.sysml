// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

package MarlinSearchEngine {
    private import ScalarValues::*;

    doc /* Architecture and lifecycle model for Marlin's converged native
           search engine (`scheme/search-engine`, `crates/marlin-agent-search-engine`)
           backed directly by ASP (`agent-semantic-search`) over in-process Rust
           library boundaries with zero cross-engine gRPC, zero embedded Orgize
           FFI, and zero embedded Turso runtime in ASP. */

    enum def SearchPlanAxis {
        enum rg;
        enum tantivy;
        enum topology;
        enum graph;
        enum openEdb;
    }

    enum def SearchStorageBackendKind {
        enum inMemory;
        enum turso;
    }

    attribute def SearchPlanProjection {
        attribute workspace : String [0..1];
        attribute languages : String [0..*];
        attribute documents : String [0..*];
        attribute axes : SearchPlanAxis [1..*];
        attribute nativeAbiOnly : Boolean;
        doc /* Typed search playbook projected from Gerbil Scheme
               `RootedSchemeObject` values without text S-expression parsing. */
    }

    attribute def SearchGenerationPointer {
        attribute projectId : String;
        attribute workspaceId : String;
        attribute pointerKey : String;
        attribute generationDigest : String;
        attribute artifactHash : String;
        attribute revision : Integer;
        doc /* Content-addressed CAS generation pointer stored in Marlin's
               unified `AgentStorage`. Revision advances monotonically on
               matching compare-and-swap only. */
    }

    attribute def SearchTopologyEdge {
        attribute projectId : String;
        attribute generationDigest : String;
        attribute edgeId : String;
        attribute fromNode : String;
        attribute toNode : String;
        attribute relation : String;
        doc /* Idempotent, generation-scoped topology edge persisted via
               Marlin's `AgentStorage::append_topology_edge`. */
    }

    attribute def SearchBenchmarkBudget {
        attribute benchmarkSuite : String;
        attribute throughputElements : Integer;
        attribute zeroOrgizeDependency : Boolean;
        attribute zeroTursoDuplicate : Boolean;
        doc /* Qualification evidence for hot-path ranking, CAS generation
               publication, and native Scheme ABI projection. */
    }

    part <'MAR-SRCH-A01'> actual {
        part schemeSearchDsl {
            doc /* `scheme/search-dsl/{types,grammar,syntax,objects,repair,extract,funcs,config,interface}.ss`
                   and `scheme/c-abi/search-dsl.ss`: unified pure S-expression
                   search DSL powered by `:gerbil-ascent` (zero-eval closed-source
                   Datalog + Dusa POPL 2025 finite-choice), `:gerbil-parser`
                   (`deflanguage` typed structured sub-syntax parsing), zero-regex
                   native AST extraction (`extract.ss` for `.ss`/`.sysml`/`.qnt`),
                   zero-regex POO `SearchLanguageRegistry.` fragments over ASP's
                   universal `semantic-open-edb.v1` `(lang, kind, language_kind)`
                   protocol, the Safe Pre-Tool-Call Model AST Repair Hook v2
                   (`repair.ss` with wildcard padding, choice-value normalization,
                   and registry-backed did-you-mean diagnostics), live LLM Tool
                   Schema generation, and Helm TAB completions. */
        }
        part schemeSearchEngine {
            doc /* `scheme/search-engine/{types,objects,funcs,config,interface}.ss`
                   and `scheme/c-abi/search-engine.ss`. Composes multi-axis
                   search playbooks in pure Gerbil Scheme and exports native
                   vector/symbol objects across the C ABI. */
        }
        part marlinSearchBridge {
            doc /* `crates/marlin-agent-search-engine`: decodes
                   `RootedSchemeObject` plans into ASP `SearchPlaybookPlanInput`,
                   projects `WorkspaceEdbPrefilter`, `WorkspaceEdbSnapshot`, and
                   `SearchDslStructuredResult` across `edb_bridge/{mod,types,codec,runner}.rs`,
                   hydrates zero-regex `.org` + Scheme/SysML/Quint Open EDB facts
                   with BLAKE3 `content_hash` persistence via `open_edb_hydrator.rs`,
                   implements `OrgDocumentProjectionPort` over
                   `marlin-context-org-model`, and implements
                   `SearchEngineStoragePort` over `marlin-agent-storage`. */
        }
        part aspNativeEngine {
            doc /* `agent-semantic-search`, `agent-semantic-search-playbook`,
                   and `agent-semantic-topology` linked directly as Rust crates
                   from a zero-submodule ASP Git revision (zero pseudo-Tree-sitter
                   `.scm`, zero Rust `ascent`, zero `agent-semantic-mrr`, and
                   zero-regex protocol-native Open EDB ingestion/projection
                   conforming to `semantic-open-edb.v1.schema.json`). */
        }
        part marlinUnifiedStorage {
            doc /* `marlin-agent-storage`: owns `InMemoryAgentStorage` and
                   `TursoAgentStorage` for content-addressed blobs, CAS
                   generation pointers, and topology edges. */
        }
        part marlinOrgModel {
            doc /* `marlin-context-org-model`: owns canonical Org AST nodes
                   and property projections supplied to ASP topology admission. */
        }
        dependency projectsPlans from schemeSearchEngine to marlinSearchBridge;
        dependency invokesNativeAsp from marlinSearchBridge to aspNativeEngine;
        dependency adaptsStoragePort from marlinSearchBridge to marlinUnifiedStorage;
        dependency adaptsOrgPort from marlinSearchBridge to marlinOrgModel;
    }

    part <'MAR-SRCH-A02'> target {
        part schemePolicyPlane {
            doc /* Gerbil Scheme module system owns composable search playbook
                   construction, axis combinators, and interactive workspace
                   policies. */
        }
        part rustAbiAdmission {
            doc /* Rust bridge validates every `RootedSchemeObject` structure,
                   rejects scope traversal or malformed predicates fail-closed,
                   and enforces generation-bound execution. */
        }
        part aspSearchKernel {
            doc /* Minimal, zero-regex pure-Rust runtime search engine
                   interface owning trigram/lexical pre-filtering, mmap
                   snapshots, BLAKE3 generation tracking, incremental delta sync,
                   and generation-scoped Open EDB ingestion/projection
                   (`ingest_open_edb_facts` / `project_prefiltered_edb` under
                   `semantic-open-edb.v1`). Lightweight query and sub-syntax
                   parsing lives in Scheme (`:gerbil-parser`); heavy
                   compilation-unit language providers emit native AST facts
                   across HTTP/gRPC boundaries. */
        }
        part unifiedStoragePort {
            doc /* Single-owner Marlin storage backend providing atomic CAS
                   generation pointers, immutable BLAKE3 artifact CAS, and
                   idempotent generation-scoped topology edge persistence. */
        }
        part unifiedOrgPort {
            doc /* Single-owner Marlin Org AST projection port eliminating any
                   duplicate Org parser or FFI inside ASP. */
        }
        part verificationHarness {
            doc /* Formal Quint models, negative mutation controls, Rust
                   proptest stateful reference-model tests, and Criterion
                   multi-scale benchmarks guarding correctness and latency. */
        }

        dependency schemeToAbi from schemePolicyPlane to rustAbiAdmission;
        dependency abiToKernel from rustAbiAdmission to aspSearchKernel;
        dependency kernelToStorage from aspSearchKernel to unifiedStoragePort;
        dependency kernelToOrg from aspSearchKernel to unifiedOrgPort;
        dependency harnessQualifiesKernel from verificationHarness to aspSearchKernel;
        dependency harnessQualifiesBridge from verificationHarness to rustAbiAdmission;
    }

    state def SearchGenerationLifecycle {
        doc /* Atomic generation publication and retirement lifecycle across
               ASP search artifacts and Marlin `AgentStorage` CAS pointers. */
        entry; then unindexed;

        state unindexed {
            doc /* Workspace has no active search generation pointer yet. */
        }
        state artifactStaged {
            doc /* Immutable BLAKE3 content-addressed artifact blob has been
                   stored and verified before any pointer swap. */
        }
        state published {
            doc /* Workspace pointer atomically references the staged blob with
                   a strictly increasing revision and matching generation digest. */
        }
        state conflictRejected {
            doc /* Stale expected hash failed CAS admission; active pointer and
                   revision remain unchanged. */
        }
        state superseded {
            doc /* A newer CAS generation replaced this generation while old
                   immutable blobs and generation-scoped edges remain isolated. */
        }

        transition stageFirst
            first unindexed
            then artifactStaged;
        transition publishInitial
            first artifactStaged
            then published;
        transition rejectStaleCas
            first artifactStaged
            then conflictRejected;
        transition stageNext
            first published
            then artifactStaged;
        transition retirePrevious
            first published
            then superseded;
        transition recoverConflict
            first conflictRejected
            then published;
    }

    action def ExecuteConvergedSearch {
        in plan : SearchPlanProjection;
        out pointer : SearchGenerationPointer;

        action composeInScheme {
            doc /* Gerbil Scheme constructs a typed multi-axis search playbook
                   as native vectors and symbols. */
        }
        action decodeNativeAbi {
            doc /* `marlin-agent-search-engine` decodes `RootedSchemeObject`
                   directly into `SearchPlaybookPlanInput` with zero text
                   serialization. */
        }
        action admitPlaybook {
            doc /* ASP `admit_search_playbook_plan` enforces fail-closed scope,
                   axis, and predicate invariants before execution. */
        }
        action executeRankingAndTopology {
            doc /* ASP runs deterministic source-index ranking, graph-owner
                   ranking, and Org/topology admission via Marlin host ports. */
        }
        action publishCasGeneration {
            doc /* Store content-addressed BLAKE3 blob first, then advance
                   Marlin `AgentStorage` pointer atomically via typed CAS. */
        }

        first composeInScheme;
        then decodeNativeAbi;
        then admitPlaybook;
        then executeRankingAndTopology;
        then publishCasGeneration;
    }

    requirement <'MAR-SRCH-R01'> nativeRustIntegration {
        doc /* Marlin embeds ASP search crates as native Rust library
               dependencies with zero gRPC hop between Marlin and ASP.
               Process/gRPC transport exists exclusively between ASP and
               external non-Rust language providers. */
    }
    requirement <'MAR-SRCH-R02'> rootedSchemeAbiOnly {
        doc /* Search plans cross from Gerbil Scheme into Rust exclusively as
               typed `RootedSchemeObject` values. Pseudo S-expression text
               parsers are prohibited across the native search boundary. */
    }
    requirement <'MAR-SRCH-R03'> singleOwnerPorts {
        doc /* ASP consumes Marlin's `OrgDocumentProjectionPort` and
               `SearchEngineStoragePort` and must never re-introduce `orgize`
               FFI or a duplicate `turso_core` database into the dependency
               graph. */
    }
    requirement <'MAR-SRCH-R04'> atomicCasAndEdgeIsolation {
        doc /* Every published search generation pointer must reference an
               existing BLAKE3-verified artifact blob, advance revision
               monotonically under expected-hash CAS, reject stale writers
               without mutating active state, and keep topology edges
               idempotent and isolated by `(project_id, generation_digest)`. */
    }
    requirement <'MAR-SRCH-R05'> zeroSubmodulePackaging {
        doc /* ASP publishes a self-contained zero-`.gitmodules` Git repository
               managing external language providers via `providers.toml`, so
               Marlin resolves ASP through stock Cargo without SSH/CLI
               environment variable workarounds. */
    }
    requirement <'MAR-SRCH-R06'> formalAndStatefulVerification {
        doc /* Search generation publication, native ABI plan admission, and
               topology edge custody are verified by Quint formal invariants
               with negative mutation controls and Rust stateful reference-model
               property tests. */
    }
    requirement <'MAR-SRCH-R07'> highPerformanceQualification {
        doc /* Hot paths (source-index rank, graph-owner rank, Scheme ABI
               projection, Org port admission, and CAS storage round-trips) are
               measured by reproducible multi-scale Criterion benchmarks. */
    }

    allocation ownNativeIntegration allocate nativeRustIntegration to target.aspSearchKernel;
    allocation ownSchemeAbi allocate rootedSchemeAbiOnly to target.rustAbiAdmission;
    allocation ownSchemePolicy allocate rootedSchemeAbiOnly to target.schemePolicyPlane;
    allocation ownStoragePort allocate singleOwnerPorts to target.unifiedStoragePort;
    allocation ownOrgPort allocate singleOwnerPorts to target.unifiedOrgPort;
    allocation ownCasLifecycle allocate atomicCasAndEdgeIsolation to target.unifiedStoragePort;
    allocation ownZeroSubmodule allocate zeroSubmodulePackaging to target.aspSearchKernel;
    allocation ownFormalVerification allocate formalAndStatefulVerification to target.verificationHarness;
    allocation ownBenchQualification allocate highPerformanceQualification to target.verificationHarness;
}

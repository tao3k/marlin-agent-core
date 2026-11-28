;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Test Case Two: Complex Senior-Engineering Read-Only Evaluation Suite
;;; (with Post-Run Model Reflection for Human Audit).
;;;
;;; Demanding, pure-English natural-language engineering investigations with
;;; zero coaching system prompt, zero leaked paths/symbols/DSL hints, and
;;; strict `< 5` round budgets (`max-rounds: 4`, `target-max-rounds: 4`).
(import (only-in :clan/poo/object .def .mix .o seal-object!)
        (only-in "./objects.ss" TestingScenario. TestingSuite.))
(export CrossLayerSearchDslChangeImpactScenario
        PolyglotGovernanceAndBitemporalAuditScenario
        StreamingContinuationIncidentTriageScenario
        StructuralHealthAndArchitectureSolvingScenario
        ComplexCodingTestingSuite.
        ComplexCodingTestingSuiteValue)

(def CrossLayerSearchDslChangeImpactScenario
  (seal-object!
    (.mix (.o id: 'cross_layer_search_dsl_change_impact
              title: "Cross-Layer Refactoring Impact Analysis: Adding Trace Context to Search Execution"
              message: "We are planning a breaking change to attach a mandatory distributed trace context field to every relational search execution request and its package-level execution entrypoint. Conduct a thorough read-only cross-layer impact analysis from the Gerbil Scheme C-ABI boundary through the Rust search engine bridge, tool runtime layer, terminal UI overlay, and headless evaluation harness. Enumerate every affected production file, struct or function signature, and line number that must be updated, and explicitly separate production call sites from test-only code."
              target-max-rounds: 4
              max-rounds: 4
              max-tool-calls: 24
              enable-reflection?: #t
              checkpoints:
              '(("Scheme C-ABI search-dsl.ss" ("scheme/c-abi/search-dsl.ss"))
                ("edb_bridge types/runner definition"
                 ("crates/marlin-agent-search-engine/src/edb_bridge/types.rs"
                  "crates/marlin-agent-search-engine/src/edb_bridge/runner.rs"))
                ("marlin-agent-tooling consumer"
                 ("crates/marlin-agent-tooling/src/grpc_search/service.rs"
                  "marlin-agent-tooling"))
                ("marlin-agent-tui consumer"
                 ("crates/marlin-agent-tui/src/helm/overlay.rs"
                  "crates/marlin-agent-tui/src/"))
                ("marlin-agent-harness consumer"
                 ("crates/marlin-agent-harness/src/live_weaver/scheme_bridge.rs"))
                ("production vs test boundary" ("tests/" "unit/"))
                ("concrete line numbers" (":"))))
          TestingScenario.)))

(def PolyglotGovernanceAndBitemporalAuditScenario
  (seal-object!
    (.mix (.o id: 'polyglot_governance_and_bitemporal_audit
              title: "Security Audit: Fail-Closed Cedar Read-Only Enforcement and Bi-Temporal Causality"
              message: "Perform a read-only security architecture audit explaining how this codebase guarantees that read-only research and evaluation agents can never execute shell commands or mutating file operations. Trace the entire enforcement chain across Gerbil Scheme and Rust: (1) identify the Scheme modules and entrypoints defining declarative read-only governance policies and bi-temporal causal trajectory verification; (2) explain how the Rust kernel compiles and evaluates Cedar policies, detailing explicit allow, explicit forbid override, and default-deny semantics with exact functions and line numbers; and (3) show how the headless evaluation harness wires both Cedar gating and bi-temporal trajectory verification into live tool dispatch."
              target-max-rounds: 4
              max-rounds: 4
              max-tool-calls: 24
              enable-reflection?: #t
              checkpoints:
              '(("scheme/policy/governance.ss" ("scheme/policy/governance.ss"))
                ("evaluate-temporal-causal-governance" ("evaluate-temporal-causal-governance"))
                ("scheme/agent-testing governance or trajectory"
                 ("scheme/agent-testing/governance.ss" "scheme/agent-testing/trajectory.ss"))
                ("governance_cedar.rs" ("crates/marlin-agent-kernel/src/governance_cedar.rs"))
                ("compile_cedar_policy or evaluate_compiled_cedar_policies"
                 ("compile_cedar_policy" "evaluate_compiled_cedar_policies"))
                ("Cedar evaluation decision semantics"
                 ("ImplicitDeny" "matching_forbids" "forbid"))
                ("harness governance gate integration"
                 ("crates/marlin-agent-harness/src/live_weaver/governance.rs"
                  "CompiledHarnessGovernanceGate"))))
          TestingScenario.)))

(def StreamingContinuationIncidentTriageScenario
  (seal-object!
    (.mix (.o id: 'streaming_continuation_incident_triage
              title: "Production Incident Triage: Blocked Multi-Turn Tool Continuations and Consistency Checks"
              message: "Users report that multi-turn tool-calling sessions in the terminal chat interface occasionally stall with a blocked-continuation error or fail with a tool-use consistency mismatch between streamed observations and final completion choices. Perform a read-only root-cause trace starting from the terminal UI's multi-turn execution loop down through the streaming LLM gateway and wire protocol layer. Pinpoint where continuation blocking is triggered, the exact validation functions checking observed streaming blocks against final assistant tool-use or thinking blocks, the corresponding error variants, and the existing unit test files covering these invariants — with exact file paths and line numbers."
              target-max-rounds: 4
              max-rounds: 4
              max-tool-calls: 24
              enable-reflection?: #t
              checkpoints:
              '(("TUI turn/continuation path"
                 ("crates/marlin-agent-tui/src/app/turn.rs" "crates/marlin-agent-tui/src/"))
                ("Continuation blocked string or handling"
                 ("Continuation blocked" "continuation"))
                ("WeaverModelGateway gateway.rs" ("crates/marlin-agent-stream/src/gateway.rs"))
                ("tool_use consistency check"
                 ("tool_use" "matches_observed" "ModelGatewayError"))
                ("stream or tui unit tests"
                 ("crates/marlin-agent-stream/tests/unit/stream.rs"
                  "crates/marlin-agent-tui/tests/unit/app_cases/continuation.rs"))))
          TestingScenario.)))

(def StructuralHealthAndArchitectureSolvingScenario
  (seal-object!
    (.mix (.o id: 'structural_health_and_architecture_solving
              title: "Structural Codebase Health Check and Finite-Choice Architecture Placement Solving"
              message: "Run a two-part architectural verification over the Rust search engine and workspace: (1) inspect the search engine crate's core structs and enums, and find concrete cross-file naming collisions across the Rust workspace for both structs and functions (at least two distinct colliding structs and two distinct colliding functions, citing file paths and line numbers); (2) model and solve a finite-choice placement constraint problem over a bounded slice of modules inside the search engine source tree where every module in scope must be assigned to either local resident execution or an external provider, while hot-path modules are strictly forbidden from using an external provider. Report the exact number of satisfiable worlds, which module assignments are forced by deduction versus free branching, and concrete witness assignments."
              target-max-rounds: 4
              max-rounds: 4
              max-tool-calls: 24
              enable-reflection?: #t
              required-tiers: '(L1-Ascent L2-Dusa)
              checkpoints:
              '(("search-engine struct/enum inventory" ("crates/marlin-agent-search-engine/src/"))
                ("duplicate Rust structs across files" (".rs"))
                ("duplicate Rust functions across files" ("fn" "function"))
                ("Dusa finite-choice SAT worlds reported" ("SAT" "world" "worlds"))
                ("Dusa deduction/branch provenance reported"
                 ("deduction" "branch" "resident" "provider"))
                ("hot-path constraint verified"
                 ("hot-path" "hot_path" "codec.rs" "runner.rs" "edb_bridge"))))
          TestingScenario.)))

(.def ComplexCodingTestingSuite.
  id: 'marlin-complex-coding-read-only-eval
  (scenarios => append
    (list CrossLayerSearchDslChangeImpactScenario
          PolyglotGovernanceAndBitemporalAuditScenario
          StreamingContinuationIncidentTriageScenario
          StructuralHealthAndArchitectureSolvingScenario)))

(def ComplexCodingTestingSuiteValue
  (seal-object! (.mix ComplexCodingTestingSuite. TestingSuite.)))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Test Case One: Core Read-Only Codebase Search Evaluation Suite.
;;;
;;; Pure English natural-language engineering messages with zero coaching
;;; system prompt, zero leaked file paths or internal symbol names, and strict
;;; `< 5` round budgets (`max-rounds: 4`).
(import (only-in :clan/poo/object .def .mix .o seal-object!)
        (only-in "./objects.ss" TestingScenario. TestingSuite.))
(export ModelGatewayImplementationsScenario
        CrossLangSearchToolPipelineScenario
        TuiConfigSchemaAndCallersScenario
        SearchEngineArchitectureCollisionsScenario
        SearchTestingSuite.
        SearchTestingSuiteValue)

(def ModelGatewayImplementationsScenario
  (seal-object!
    (.mix (.o id: 'model_gateway_implementations
              title: "Discover All Streaming Model Gateway Implementations Across the Workspace"
              message: "I am onboarding onto the model streaming layer and need a complete inventory of how this repository abstracts and implements live versus deterministic LLM gateways. Find the core streaming gateway trait, list every concrete implementation type across the Rust workspace (both production gateways and deterministic, scripted, or local test stubs), and report each implementation's exact file path, line number, and distinguishing role."
              target-max-rounds: 4
              max-rounds: 4
              max-tool-calls: 20
              checkpoints:
              '(("WeaverModelGateway" ("WeaverModelGateway"))
                ("stream gateway.rs path" ("crates/marlin-agent-stream/src/gateway.rs"))
                ("RuntimeEdgeModelGateway" ("RuntimeEdgeModelGateway"))
                ("runtime model_gateway.rs path" ("crates/marlin-agent-runtime/src/model_gateway.rs"))
                ("additional workspace impls beyond 2 files"
                 ("NoLiveLlmModelGateway"
                  "ScriptedModelGateway"
                  "HotSwappableModelGateway"
                  "DeterministicLocalGateway"
                  "StaticGateway"))))
          TestingScenario.)))

(def CrossLangSearchToolPipelineScenario
  (seal-object!
    (.mix (.o id: 'cross_lang_search_tool_pipeline
              title: "Trace the Polyglot Scheme-to-Rust Codebase Search Tool Pipeline"
              message: "Trace the complete end-to-end architecture of the repository's built-in codebase search tool across both Gerbil Scheme and Rust. Explain where the tool is declaratively specified and exported on the Scheme side, how it crosses the native C-ABI boundary without JSON serialization, which Rust handler receives and dispatches incoming tool calls, and how queries fan out between fast in-memory text/outline/read execution versus relational logic and finite-choice evaluation. Include exact file paths, key symbols, and line numbers."
              target-max-rounds: 4
              max-rounds: 4
              max-tool-calls: 20
              checkpoints:
              '(("scheme/tooling/objects.ss" ("scheme/tooling/objects.ss"))
                ("Scheme tool declaration symbol"
                 ("WorkspaceSearchToolDeclaration" "WorkspaceSearchToolSpecValue"))
                ("Scheme C-ABI search-dsl or tooling bridge"
                 ("scheme/c-abi/search-dsl.ss" "scheme/c-abi/tooling.ss"))
                ("ResidentSearchToolHandler" ("ResidentSearchToolHandler"))
                ("crates/marlin-agent-tooling/src/search_tool.rs"
                 ("crates/marlin-agent-tooling/src/search_tool.rs"))
                ("L0 brute_search dispatcher"
                 ("execute_brute_search" "crates/marlin-agent-tooling/src/brute_search"))
                ("marlin-agent-search-engine edb_bridge"
                 ("marlin-agent-search-engine" "edb_bridge"))))
          TestingScenario.)))

(def TuiConfigSchemaAndCallersScenario
  (seal-object!
    (.mix (.o id: 'tui_config_schema_and_callers
              title: "Audit Terminal UI Configuration Schema and Workspace Startup Callers"
              message: "Before refactoring how the terminal UI loads its runtime TOML configuration, I need a precise structural audit of the TUI configuration module and all of its callers. Identify the file defining the TUI configuration schema, enumerate the major configuration structs and wire-protocol enums it declares with line numbers, and locate every Rust call site in the repository that invokes the configuration loader during CLI or application startup."
              target-max-rounds: 4
              max-rounds: 4
              max-tool-calls: 20
              checkpoints:
              '(("TUI config.rs path" ("crates/marlin-agent-tui/src/config.rs"))
                ("struct Config" ("Config"))
                ("struct GitContextConfig" ("GitContextConfig"))
                ("struct TuiUiPackageConfig" ("TuiUiPackageConfig"))
                ("struct SessionConfig" ("SessionConfig"))
                ("enum WireProtocol" ("WireProtocol"))
                ("struct ModelProfile" ("ModelProfile"))
                ("caller runner.rs" ("crates/marlin-agent-tui/src/app/runner.rs"))
                ("caller cli.rs" ("crates/marlin-agent-tui/src/cli.rs"))))
          TestingScenario.)))

(def SearchEngineArchitectureCollisionsScenario
  (seal-object!
    (.mix (.o id: 'search_engine_architecture_collisions
              title: "Analyze Search Engine Module Dependencies and Cross-File Rust Symbol Collisions"
              message: "Perform a two-part structural codebase analysis: first, map out the core source modules, primary types, and internal dependency flow inside the Rust search engine crate; second, detect real symbol-name collisions across the Rust workspace by identifying at least two concrete structs or functions that share the exact same name in different Rust source files, providing exact file paths and line numbers for each collision."
              target-max-rounds: 4
              max-rounds: 4
              max-tool-calls: 20
              checkpoints:
              '(("edb_bridge module" ("edb_bridge"))
                ("open_edb_hydrator or asp_http_client"
                 ("open_edb_hydrator" "asp_http_client"))
                ("projection or engine module"
                 ("projection.rs" "engine.rs" "SearchPlanProjectionError"))
                ("search-engine concrete path"
                 ("crates/marlin-agent-search-engine/src/"))
                ("concrete duplicate symbol with .rs paths" (".rs"))))
          TestingScenario.)))

(.def SearchTestingSuite.
  id: 'marlin-search-read-only-eval
  (scenarios => append
    (list ModelGatewayImplementationsScenario
          CrossLangSearchToolPipelineScenario
          TuiConfigSchemaAndCallersScenario
          SearchEngineArchitectureCollisionsScenario)))

(def SearchTestingSuiteValue
  (seal-object! (.mix SearchTestingSuite. TestingSuite.)))

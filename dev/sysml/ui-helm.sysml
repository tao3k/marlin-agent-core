// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

package MarlinUiHelm {
    private import ScalarValues::*;

    doc /* OpenSysML v2 design model for Marlin's Modern Helm (`scheme/ui-helm`),
           combining non-blocking Rust/ASP multi-axis execution with Consult
           two-stage queries, Orderless local filtering, Marginalia annotations,
           virtual viewport rendering, and Embark context/agent actions. */

    enum def HelmLayoutMode {
        enum splitPreview;
        enum bottomList;
    }

    enum def HelmSourceKind {
        enum search;
        enum dsl;
        enum buffers;
        enum commands;
        enum org;
    }

    attribute def HelmSessionState {
        attribute inputText : String;
        attribute queryEpoch : Integer;
        attribute appliedEpoch : Integer;
        attribute viewportHeight : Integer;
        attribute layout : HelmLayoutMode;
        attribute generationDigest : String [0..1];
        doc /* Pure immutable Scheme session state; stale async results with
               `resultEpoch < queryEpoch` are discarded fail-closed. */
    }

    part <'MAR-HELM-A01'> target {
        part helmSchemeUi {
            doc /* `scheme/ui-helm/{types,objects,funcs,config,interface}.ss`:
                   pure functional state transitions, two-stage parser,
                   Orderless local matcher, virtual viewport slicing, and TUI
                   frame projection. */
        }
        part helmCAbiBridge {
            doc /* `scheme/c-abi/ui-helm.ss` + `marlin-agent-search-engine`:
                   projects live Helm prompts straight into `RootedSchemeObject`
                   vectors admitted by ASP without text S-expression parsing. */
        }
        part nativeSearchBackend {
            doc /* `WorkspaceSearchIndex::open_or_build_cached`,
                   `WorkspaceSearchIndex::sync_workspace_from_disk`,
                   `WorkspaceSearchIndex::live_file_context`,
                   `hydrate_workspace_open_edb_org_and_cache`, and
                   `MarlinConvergedSearchEngine::execute_admitted_playbook_with_index`:
                   pure-Rust resident multi-axis workspace engine with zero-copy
                   mmap cold-start snapshots, BLAKE3 `content_hash`-verified
                   Open EDB caching, in-place incremental delta sync on refresh
                   (`C-l`), and BLAKE3 generation-bound hits in microseconds. */
        }
        part embarkActionSink {
            doc /* Dispatches confirmed/marked Helm hits (including live
                   `HELM-DSL` `:gerbil-ascent` / Dusa answer rows carrying
                   `@path:line` citations) to buffer navigation, Shared Org
                   Context export, or Agent turns carrying verified generation
                   provenance. */
        }
        part agentSearchTool {
            doc /* `scheme/tooling/objects.ss` (`workspace_search`, pure S-exp
                   description projected from `scheme/search-dsl`) +
                   `scheme/search-dsl/repair.ss` (safe pre-tool-call Model AST
                   repair hook v2 with wildcard padding, choice-value
                   normalization, audit receipt, and POO did-you-mean diagnostics) +
                   `marlin-agent-tooling::ResidentSearchToolHandler`:
                   executes pure `:gerbil-ascent` S-expression queries over the
                   shared resident `SharedSearchIndexSlot` + zero-regex
                   `semantic-open-edb.v1` Open EDB with BLAKE3 generation
                   evidence and zero legacy `@sym:` shim. */
        }
        part searchDslCompletionTable {
            doc /* `scheme/search-dsl/funcs.ss` (`search-dsl-completions`) +
                   `HelmOverlay::apply_dsl_result` / `linked_ui::service_helm_dsl`:
                   POO `SearchLanguageRegistry.`-driven context-aware TAB
                   completion table (`Form | Kind | Signature | Lang | Doc`)
                   plus live `HELM-DSL` S-expression execution with RAM preview
                   and `repaired[...]` mode-line receipts. */
        }

        dependency uiToAbi from helmSchemeUi to helmCAbiBridge;
        dependency abiToSearch from helmCAbiBridge to nativeSearchBackend;
        dependency uiToActions from helmSchemeUi to embarkActionSink;
        dependency toolToSearch from agentSearchTool to nativeSearchBackend;
    }

    state def HelmSessionLifecycle {
        doc /* Modern Helm interactive lifecycle with monotonic query epoch fencing. */
        entry; then closed;

        state closed;
        state prompting;
        state executingAsync;
        state presenting;
        state actionMenu;

        transition openHelm first closed then prompting;
        transition submitAsyncQuery first prompting then executingAsync;
        transition deliverFreshResults first executingAsync then presenting;
        transition supersedeQuery first executingAsync then prompting;
        transition refineLocalOrderless first presenting then presenting;
        transition openEmbarkMenu first presenting then actionMenu;
        transition executeAction first actionMenu then presenting;
        transition cancelHelm first presenting then closed;
    }

    action def ExecuteHelmSearch {
        in session : HelmSessionState;
        out nextSession : HelmSessionState;

        action parseTwoStagePrompt;
        action projectRootedPlaybook;
        action executeNativeAspSearch;
        action fenceEpochAndFilterOrderless;
        action renderVirtualViewport;

        first parseTwoStagePrompt;
        then projectRootedPlaybook;
        then executeNativeAspSearch;
        then fenceEpochAndFilterOrderless;
        then renderVirtualViewport;
    }

    requirement <'MAR-HELM-R01'> nonBlockingNativeExecution {
        doc /* Heavy workspace search executes in native Rust (`agent-semantic-search`)
               rather than blocking the Scheme UI loop. */
    }
    requirement <'MAR-HELM-R02'> twoStageConsultOrderless {
        doc /* Prompts split on `#` into an async Playbook query (`@lang:`, `@doc:`,
               `@prefix:`, `@ext:`, `@sym:`, `@graph:`) and instant local Orderless
               tokens (including `!negation`) that refine results without incrementing
               async epoch. */
    }
    requirement <'MAR-HELM-R03'> epochFencingAndMarkSafety {
        doc /* Async results with `resultEpoch < session.epoch` or arriving after
               close are dropped fail-closed; marked candidate IDs are always a
               subset of visible filtered candidates. */
    }
    requirement <'MAR-HELM-R04'> boundedVirtualViewport {
        doc /* TUI rendering slices candidates to `viewportHeight` so frame cost
               stays O(viewport) independent of total workspace matches. */
    }
    requirement <'MAR-HELM-R05'> embarkAgentAndContextActions {
        doc /* Confirmed or multi-marked candidates dispatch typed action receipts
               (`open`, `export-context`, `dispatch-agent`) carrying the active
               BLAKE3 `generationDigest`. */
    }

    allocation ownNonBlocking allocate nonBlockingNativeExecution to target.nativeSearchBackend;
    allocation ownTwoStage allocate twoStageConsultOrderless to target.helmSchemeUi;
    allocation ownEpochFence allocate epochFencingAndMarkSafety to target.helmSchemeUi;
    allocation ownViewport allocate boundedVirtualViewport to target.helmSchemeUi;
    allocation ownEmbarkActions allocate embarkAgentAndContextActions to target.embarkActionSink;
}

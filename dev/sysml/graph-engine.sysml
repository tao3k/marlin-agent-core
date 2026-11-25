// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

package MarlinGraphEngine {
    private import ScalarValues::*;

    doc /* OpenSysML v2 design model for Marlin's Session-free GraphEngine
           (`agentic-ai-lab` pure semantic core -> `scheme/graph-engine` ->
           `scheme/c-abi/graph-engine.ss` -> `marlin-gerbil-scheme` /
           `marlin-agent-graph`), unifying Acyclic FunFlow C4 DAGs, Convergent
           Trees, Tarjan SCC feedback subgraphs, Session-free TaskActors, C4
           mid-flight Steering convergence gates, anti-split-brain fencing-epoch
           receipt reconciliation, and pinned-Index Workbench tabs. */

    enum def TaskActorScopeMode {
        enum project;
        enum detached;
    }

    enum def TaskActorStatus {
        enum openFocus;
        enum running;
        enum waitingSteering;
        enum settled;
        enum failed;
        enum aborted;
    }

    enum def GraphDispatchOperation {
        enum executeFrontier;
        enum applySteering;
        enum convergeApex;
        enum commitSettlement;
    }

    attribute def TaskActorState {
        attribute taskId : String;
        attribute fencingEpoch : Integer;
        attribute appliedEpoch : Integer;
        attribute pendingSteeringBranches : Integer;
        attribute scopeMode : TaskActorScopeMode;
        attribute status : TaskActorStatus;
        attribute joinNodeId : String [0..1];
        doc /* Pure immutable Session-free TaskActor cursor; stale receipts
               with `receiptEpoch < fencingEpoch` are rejected fail-closed and
               trunk continuation cannot pass `joinNodeId` while anchored
               steering branches remain pending. */
    }

    part <'MAR-GRAPH-A01'> target {
        part labGraphEngineCore {
            doc /* `agentic-ai-lab/modules/graph-engine`: pure functional
                   Scheme library owning ObligationGraph, FunFlow C4 DAG
                   frontier scheduling, ConvergentTree, Tarjan SCC condensation,
                   TaskActor, SteeringMessage, MarlinDispatchEnvelope, and
                   opt-in `+sqlite-turso` WAL execution. */
        }
        part marlinGraphEngineScheme {
            doc /* `scheme/graph-engine/{types,objects,funcs,config,interface}.ss`:
                   mounts `(graph-engine +sqlite-turso)` via Yuanxu, projecting
                   TaskActor states into Task Workbench UI packages, continuation
                   protocols, and flat rooted C-ABI vectors. */
        }
        part graphEngineCAbiBridge {
            doc /* `scheme/c-abi/graph-engine.ss` + `marlin-gerbil-scheme::graph_engine`:
                   crosses the Scheme/Rust boundary strictly via borrowed
                   `RootedSchemeObject` vectors with zero text serialization. */
        }
        part rustGraphActorGate {
            doc /* `marlin-agent-graph::task_actor`: owns domain newtypes,
                   fail-closed `admit_marlin_dispatch`, monotonic `fencing-epoch`
                   `reconcile_marlin_execution_receipt`, and `WorkbenchTabProjection`. */
        }
        part workbenchTabSink {
            doc /* Projects pinned TaskActorIndex at slot 0, active TaskActor
                   tabs, and strictly disjoint read-only ID peek tabs into
                   Marlin's TUI Task Workbench. */
        }

        dependency schemeToLab from marlinGraphEngineScheme to labGraphEngineCore;
        dependency schemeToAbi from marlinGraphEngineScheme to graphEngineCAbiBridge;
        dependency abiToRustGate from graphEngineCAbiBridge to rustGraphActorGate;
        dependency rustGateToWorkbench from rustGraphActorGate to workbenchTabSink;
    }

    state def TaskActorLifecycle {
        doc /* Session-free TaskActor lifecycle with C4 steering gate and terminal freeze. */
        entry; then openFocus;

        state openFocus;
        state running;
        state waitingSteering;
        state settled;
        state failed;

        transition promoteFocus first openFocus then running;
        transition anchorSteering first running then waitingSteering;
        transition convergeSteering first waitingSteering then running;
        transition settleAllTerminal first running then settled;
        transition failExecution first running then failed;
        transition failDuringSteering first waitingSteering then failed;
    }

    action def ExecuteGraphDispatch {
        in taskActor : TaskActorState;
        out nextTaskActor : TaskActorState;

        action buildDispatchVector;
        action crossRootedCAbi;
        action admitRustDispatchGate;
        action executeNativeOrTursoStep;
        action reconcileFencingEpochReceipt;
        action projectWorkbenchTabs;

        first buildDispatchVector;
        then crossRootedCAbi;
        then admitRustDispatchGate;
        then executeNativeOrTursoStep;
        then reconcileFencingEpochReceipt;
        then projectWorkbenchTabs;
    }

    requirement <'MAR-GRAPH-R01'> sessionFreeTaskActorLifecycle {
        doc /* Tasks begin as lightweight `open-focus` drafts without creating
               a persistent session and promote on demand into bounded
               `TaskActor` units that settle atomically into `TaskActorIndex`. */
    }
    requirement <'MAR-GRAPH-R02'> c4SteeringConvergenceGate {
        doc /* Mid-flight `apply-steering` requires a concrete `join-node-id`
               anchor; trunk continuation and settlement cannot proceed while
               anchored steering branches remain pending. */
    }
    requirement <'MAR-GRAPH-R03'> monotonicFencingEpochAntiSplitBrain {
        doc /* Every execution receipt must match `task-id` and current
               `fencing-epoch`; stale, future, mismatched, or post-terminal
               receipts are rejected fail-closed without mutating state. */
    }
    requirement <'MAR-GRAPH-R04'> detachedScopeWriteIsolation {
        doc /* A `detached` TaskActor scope cannot commit project-level
               settlements into the workspace index. */
    }
    requirement <'MAR-GRAPH-R05'> rootedAbiAndPinnedIndexWorkbench {
        doc /* GraphEngine proposals and receipts cross Scheme/Rust strictly as
               rooted Scheme vectors decoded into typed Rust newtypes; Workbench
               projections always pin `TaskActorIndex` at slot 0 and keep
               read-only peek tabs strictly disjoint from active writable tabs. */
    }
}

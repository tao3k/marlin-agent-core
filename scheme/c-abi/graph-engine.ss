;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Native C-ABI entrypoint projecting Session-free GraphEngine TaskActor
;;; dispatch requests, anti-split-brain fencing-epoch receipt reconciliations,
;;; and pinned-Index Workbench tabs as rooted Scheme vectors for Rust
;;; (`marlin-gerbil-scheme` / `marlin-agent-graph`).
(import (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :marlin-agent-core/graph-engine/interface GraphEngine))
(export marlin-graph-engine-dispatch-vector
        marlin-graph-engine-reconcile-receipt-vector-abi
        marlin-graph-engine-workbench-tabs-vector
        marlin-graph-engine-governed-context-vector-abi)

(def (marlin-graph-engine-dispatch-vector input)
  ((.get GraphEngine core exports project-dispatch-vector) input))

(def (marlin-graph-engine-reconcile-receipt-vector-abi input)
  ((.get GraphEngine core exports reconcile-receipt-vector) input))

(def (marlin-graph-engine-workbench-tabs-vector input)
  ((.get GraphEngine core exports project-workbench-tabs-vector) input))

(def (marlin-graph-engine-governed-context-vector-abi input)
  ((.get GraphEngine core exports project-governed-context-vector) input))

(begin-foreign
  (c-define (graph-engine-dispatch-vector root) (int64) int64
    "marlin_graph_engine_dispatch_vector" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/graph-engine#marlin-graph-engine-dispatch-vector))
  (c-define (graph-engine-reconcile-receipt-vector root) (int64) int64
    "marlin_graph_engine_reconcile_receipt_vector" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/graph-engine#marlin-graph-engine-reconcile-receipt-vector-abi))
  (c-define (graph-engine-workbench-tabs-vector root) (int64) int64
    "marlin_graph_engine_workbench_tabs_vector" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/graph-engine#marlin-graph-engine-workbench-tabs-vector))
  (c-define (graph-engine-governed-context-vector root) (int64) int64
    "marlin_graph_engine_governed_context_vector" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/graph-engine#marlin-graph-engine-governed-context-vector-abi)))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Boundary: Marlin-side functional glue composing `agentic-ai-lab`'s pure
;;; `GraphEngine` library with Marlin's Task Workbench UI (`make-marlin-task-workbench-package`),
;;; Rust-facing LoopGraph continuation native ABI (`marlin-graph-loop-continuation-next-action`),
;;; and Marlin's opt-in `+sqlite-turso` WAL execution boundary.

(import (only-in :clan/poo/object .get .o .ref .slot? object?)
        (rename-in (only-in :tao3k/agentic-ai-lab/modules/graph-engine/interface GraphEngine)
                   (GraphEngine LabGraphEngine))
        (only-in :marlin-agent-core/interface LabDurableJournal))

(export marlin-graph-engine-project-workbench-ui
        marlin-graph-engine-dispatch->continuation
        marlin-graph-engine-execute-turso-dispatch!)

(def +marlin-projection-package+ "marlin-agent-core")
(def +marlin-projection-object-system+ "gerbil-poo")
(def +marlin-compiled-loop-graph-type-id+ "marlin.deck-runtime.compiled-loop-graph")
(def +marlin-compiled-loop-graph-schema-id+ "marlin.deck-runtime.compiled-loop-graph.v1")
(def +marlin-loop-continuation-type-id+ "marlin.agent.gerbil-loop-continuation")
(def +marlin-loop-continuation-schema-id+ "marlin.agent.gerbil_loop_graph_continuation.v1")

(def (opt-ref obj key default)
  (if (and (object? obj) (.slot? obj key))
    (.ref obj key)
    default))

(def (value->config-string v)
  (cond
   ((string? v) v)
   ((symbol? v) (symbol->string v))
   ((number? v) (number->string v))
   ((boolean? v) (if v "true" "false"))
   ((not v) "none")
   (else "value")))

(def (error-list->reason errors)
  (if (null? errors)
    "marlin-dispatch-rejected"
    (let loop ((rest (cdr errors))
               (acc (value->config-string (car errors))))
      (if (null? rest)
        acc
        (loop (cdr rest)
              (string-append acc "," (value->config-string (car rest))))))))

;;; Pure UI projection: turns Session-free `TaskActor.`s + `TaskActorIndex.`
;;; into `WorkbenchTabProjection`, optionally projecting through a caller-supplied
;;; `package-builder` (e.g. `make-marlin-task-workbench-package` from `ui`) so
;;; sibling Yuanxu modules stay cleanly decoupled.
(def (marlin-graph-engine-project-workbench-ui
      actors
      index
      peek-set: (peek-set '())
      focused-task-id: (focused-task-id #f)
      focused-tab-archive: (focused-tab-archive 'on-blur)
      disclosure-level: (disclosure-level 'L1)
      nerd-icons?: (nerd-icons? #f)
      package-builder: (package-builder #f))
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (project-tabs (.ref lab-exports 'project-task-actors->workbench-tabs))
         (projection (project-tabs
                      actors
                      index
                      peek-set: peek-set
                      focused-task-id: focused-task-id
                      focused-tab-archive: focused-tab-archive))
         (ui-package (and package-builder
                          (package-builder
                           projection
                           disclosure-level: disclosure-level
                           nerd-icons?: nerd-icons?))))
    (.o bind-these: #f
        session-free?: #t
        tab-projection: projection
        ui-package: ui-package)))

;;; Pure Rust-facing native ABI projection: translates a `make-graph-marlin-dispatch`
;;; result (admitted `MarlinDispatchEnvelope.` OR fail-closed rejection) into the
;;; exact typed POO continuation projection (`marlin.agent.gerbil-loop-continuation`
;;; / `marlin.agent.gerbil_loop_graph_continuation.v1`) consumed by Marlin's Rust
;;; controller. Never serializes text protocols or manufactures Rust handlers.
(def (marlin-graph-engine-dispatch->continuation
      dispatch-result
      executor-id: (executor-id "marlin.graph_engine.executor")
      settlement-outcome: (settlement-outcome 'converged))
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (dispatch->alist (.ref lab-exports 'marlin-dispatch->alist))
         (admitted? (opt-ref dispatch-result 'admitted? #f))
         (errors (opt-ref dispatch-result 'errors '())))
    (if (not admitted?)
      (let ((reason (error-list->reason errors)))
        (.o bind-these: #f
            type_id: +marlin-loop-continuation-type-id+
            schema_id: +marlin-loop-continuation-schema-id+
            action: (.o bind-these: #f kind: "escalate_to_human" reason: reason)
            diagnostics: (list (string-append "graph_engine_dispatch=blocked:" reason))))
      (let* ((env (.ref dispatch-result 'envelope))
             (op (.ref env 'operation))
             (tid (.ref env 'task-id))
             (rid (.ref env 'request-id))
             (abi-alist (dispatch->alist env))
             (config-rows
              (list (cons "request_id" rid)
                    (cons "task_id" tid)
                    (cons "operation" (value->config-string op))
                    (cons "fencing_epoch" (value->config-string (.ref env 'fencing-epoch)))
                    (cons "scope_mode" (value->config-string (.ref env 'scope-mode)))
                    (cons "substrate_target" (value->config-string (.ref env 'substrate-target)))
                    (cons "join_node_id" (value->config-string (.ref env 'join-node-id)))
                    (cons "kv_prefix_digest" (value->config-string (.ref env 'kv-prefix-digest)))
                    (cons "session_free" "true")))
             (action
              (cond
               ((eq? op 'commit-settlement)
                (if (eq? settlement-outcome 'converged)
                  (.o kind: "stop_completed")
                  (.o kind: "stop_failed")))
               (else
                (let ((compiled
                       (.o bind-these: #f
                           type_id: +marlin-compiled-loop-graph-type-id+
                           schema_id: +marlin-compiled-loop-graph-schema-id+
                           graph_id: (string-append "graph-engine:" tid)
                           object_system: +marlin-projection-object-system+
                           package: +marlin-projection-package+
                           module: ":marlin-agent-core/graph-engine/funcs"
                           nodes: (list (.o bind-these: #f
                                            id: (string-append tid ":" (symbol->string op))
                                            executor: executor-id
                                            config: config-rows))
                           edges: '())))
                  (.o bind-these: #f
                      kind: "continue_with_graph"
                      compiled_graph: compiled)))))
             (diags (list (string-append "graph_engine_dispatch=" (symbol->string op))
                          (string-append "fencing_epoch=" (number->string (.ref env 'fencing-epoch)))
                          (string-append "abi_fields=" (number->string (length abi-alist))))))
        (.o bind-these: #f
            type_id: +marlin-loop-continuation-type-id+
            schema_id: +marlin-loop-continuation-schema-id+
            action: action
            diagnostics: diags)))))

;;; Explicit `!` Marlin host execution boundary: executes a validated
;;; `MarlinDispatchEnvelope.` against Marlin's `LabDurableJournal` (SQLite/Turso
;;; WAL) and immediately reconciles the resulting `MarlinExecutionReceipt.`
;;; against `actor` (enforcing the `fencing-epoch` anti-split-brain guard).
(def (marlin-graph-engine-execute-turso-dispatch!
      journal
      actor
      dispatch-envelope
      join-node-id: (join-node-id #f)
      branch-outcomes: (branch-outcomes '())
      executed-wave-ids: (executed-wave-ids '()))
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (make-receipt-fn (.ref lab-exports 'make-marlin-receipt))
         (reconcile-fn (.ref lab-exports 'reconcile-marlin-receipt))
         (intent (.ref dispatch-envelope 'journal-handoff))
         (receive! (.ref LabDurableJournal 'receive!))
         (begin! (.ref LabDurableJournal 'begin!))
         (put! (.ref LabDurableJournal 'put!))
         (commit! (.ref LabDurableJournal 'commit!))
         (verify! (.ref LabDurableJournal 'verify!))
         (rows-fn (.ref LabDurableJournal 'rows)))
    (when intent
      (let ((event-id (.ref intent 'event-id))
            (event-payload (.ref intent 'event-payload))
            (state-key (.ref intent 'state-key))
            (snap (.ref intent 'inert-snapshot)))
        (receive! journal event-id event-payload)
        (begin! journal event-id)
        (put! journal state-key snap)
        (commit! journal)))
    (let* ((head-hash (verify! journal))
           (j-epoch (max 1 (length (rows-fn journal))))
           (receipt (make-receipt-fn
                     (.ref dispatch-envelope 'request-id)
                     (.ref dispatch-envelope 'task-id)
                     (.ref dispatch-envelope 'fencing-epoch)
                     status: 'ok
                     journal-epoch: j-epoch
                     chain-head-hash: head-hash
                     executed-wave-ids: executed-wave-ids
                     branch-outcomes: branch-outcomes
                     substrate: 'sqlite-turso))
           (reconciled (reconcile-fn actor receipt join-node-id: join-node-id)))
      (.o bind-these: #f
          receipt: receipt
          reconciled: reconciled
          accepted?: (.ref reconciled 'accepted?)
          reason: (.ref reconciled 'reason)
          actor: (.ref reconciled 'actor)))))

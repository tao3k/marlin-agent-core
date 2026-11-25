;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Boundary: Marlin-side functional glue composing `agentic-ai-lab`'s pure
;;; `GraphEngine` library with Marlin's Task Workbench UI (`make-marlin-task-workbench-package`),
;;; Rust-facing LoopGraph continuation native ABI (`marlin-graph-loop-continuation-next-action`),
;;; and Marlin's opt-in `+sqlite-turso` WAL execution boundary.

(import (only-in :clan/poo/object .get .o .ref .slot? object?)
        (rename-in (only-in :tao3k/agentic-ai-lab/modules/graph-engine/interface
                            GraphEngine
                            make-capability-envelope
                            make-obligation
                            make-deterministic-task-node
                            make-obligation-graph)
                   (GraphEngine LabGraphEngine))
        (only-in :marlin-agent-core/context-system/convergence
                 marlin-converge-assembly
                 marlin-converge-runtime-overlay)
        (only-in :marlin-agent-core/interface LabDurableJournal))

(export marlin-graph-engine-project-workbench-ui
        marlin-graph-engine-dispatch->continuation
        marlin-graph-engine-execute-turso-dispatch!
        marlin-graph-engine-assemble-governed-context
        marlin-graph-engine-project-dispatch-vector
        marlin-graph-engine-reconcile-receipt-vector
        marlin-graph-engine-project-workbench-tabs-vector
        marlin-graph-engine-project-governed-context-vector
        marlin-graph-engine-atomic-effect->vector
        marlin-graph-engine-vector->atomic-receipt
        marlin-graph-engine-make-demo-driver-state-vector
        marlin-graph-engine-step-driver-vector
        marlin-graph-engine-make-demo-fleet-vector
        marlin-graph-engine-step-fleet-vector
        marlin-graph-engine-fleet-checkpoint-vector
        marlin-graph-engine-fleet-restore-vector
        marlin-graph-engine-fleet-summary-vector)

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

;;; Pure rooted-vector C-ABI projection helpers (ZERO text serialization).
;;; ABI version 1 flat vector schemas decoded directly by Rust `marlin-gerbil-scheme`.
(def +marlin-graph-engine-abi-version+ 1)

(def +allowed-dispatch-operations+
  '(execute-frontier apply-steering converge-apex commit-settlement))

(def +allowed-scope-modes+ '(project detached))

(def +allowed-substrate-targets+ '(pure-memory sqlite-turso))

(def +terminal-actor-statuses+ '(settled failed aborted))

(def (non-empty-string? v)
  (and (string? v) (> (string-length v) 0)))

;;; Project a 7-slot dispatch request vector:
;;;   #(request-id task-id operation fencing-epoch scope-mode join-node-id substrate-target)
;;; into a 10-slot rooted ABI vector:
;;;   #(version admitted? request-id task-id operation fencing-epoch scope-mode join-node-id substrate-target reason)
(def (marlin-graph-engine-project-dispatch-vector input)
  (if (not (and (vector? input) (= (vector-length input) 7)))
    (vector +marlin-graph-engine-abi-version+ #f "" "" 'execute-frontier 0 'project #f 'pure-memory
            "invalid-dispatch-request-vector")
    (let ((req-id (vector-ref input 0))
          (task-id (vector-ref input 1))
          (op (vector-ref input 2))
          (epoch (vector-ref input 3))
          (scope-mode (vector-ref input 4))
          (join-node-id (vector-ref input 5))
          (substrate (vector-ref input 6)))
      (let ((reason
             (cond
              ((not (non-empty-string? req-id)) "empty-request-id")
              ((not (non-empty-string? task-id)) "empty-task-id")
              ((not (memq op +allowed-dispatch-operations+)) "unsupported-dispatch-operation")
              ((not (and (fixnum? epoch) (> epoch 0))) "invalid-fencing-epoch")
              ((not (memq scope-mode +allowed-scope-modes+)) "unsupported-scope-mode")
              ((not (or (not join-node-id) (non-empty-string? join-node-id))) "invalid-join-node-id")
              ((not (memq substrate +allowed-substrate-targets+)) "unsupported-substrate-target")
              ((and (eq? op 'apply-steering) (not join-node-id)) "steering-requires-join-node-id")
              ((and (eq? scope-mode 'detached) (eq? op 'commit-settlement))
               "detached-scope-cannot-commit-project-settlement")
              (else #f))))
        (if reason
          (vector +marlin-graph-engine-abi-version+ #f
                  (if (string? req-id) req-id "")
                  (if (string? task-id) task-id "")
                  (if (symbol? op) op 'execute-frontier)
                  (if (fixnum? epoch) epoch 0)
                  (if (symbol? scope-mode) scope-mode 'project)
                  (if (string? join-node-id) join-node-id #f)
                  (if (symbol? substrate) substrate 'pure-memory)
                  reason)
          (vector +marlin-graph-engine-abi-version+ #t
                  req-id task-id op epoch scope-mode join-node-id substrate #f))))))

;;; Reconcile an 8-slot execution receipt vector against actor state:
;;;   #(actor-task-id actor-epoch actor-status pending-steering-count
;;;     receipt-request-id receipt-task-id receipt-epoch receipt-status)
;;; Returns a 5-slot rooted ABI vector:
;;;   #(version accepted? next-epoch next-status reason)
(def (marlin-graph-engine-reconcile-receipt-vector input)
  (if (not (and (vector? input) (= (vector-length input) 8)))
    (vector +marlin-graph-engine-abi-version+ #f 0 'failed 'invalid-receipt-vector)
    (let ((actor-task-id (vector-ref input 0))
          (actor-epoch (vector-ref input 1))
          (actor-status (vector-ref input 2))
          (pending-steering (vector-ref input 3))
          (receipt-req-id (vector-ref input 4))
          (receipt-task-id (vector-ref input 5))
          (receipt-epoch (vector-ref input 6))
          (receipt-status (vector-ref input 7)))
      (cond
       ((not (and (non-empty-string? actor-task-id)
                  (non-empty-string? receipt-req-id)
                  (non-empty-string? receipt-task-id)))
        (vector +marlin-graph-engine-abi-version+ #f
                (if (fixnum? actor-epoch) actor-epoch 0)
                (if (symbol? actor-status) actor-status 'failed)
                'empty-identifier))
       ((not (string=? actor-task-id receipt-task-id))
        (vector +marlin-graph-engine-abi-version+ #f actor-epoch actor-status 'task-id-mismatch))
       ((memq actor-status +terminal-actor-statuses+)
        (vector +marlin-graph-engine-abi-version+ #f actor-epoch actor-status 'actor-already-terminal))
       ((not (and (fixnum? actor-epoch) (> actor-epoch 0)
                  (fixnum? receipt-epoch) (> receipt-epoch 0)))
        (vector +marlin-graph-engine-abi-version+ #f
                (if (fixnum? actor-epoch) actor-epoch 0)
                actor-status 'invalid-fencing-epoch))
       ((< receipt-epoch actor-epoch)
        (vector +marlin-graph-engine-abi-version+ #f actor-epoch actor-status 'stale-fencing-epoch))
       ((> receipt-epoch actor-epoch)
        (vector +marlin-graph-engine-abi-version+ #f actor-epoch actor-status 'future-fencing-epoch))
       ((not (and (fixnum? pending-steering) (>= pending-steering 0)))
        (vector +marlin-graph-engine-abi-version+ #f actor-epoch actor-status 'invalid-pending-steering))
       ((eq? receipt-status 'ok)
        (if (> pending-steering 0)
          (vector +marlin-graph-engine-abi-version+ #t (+ actor-epoch 1)
                  'waiting-steering 'awaiting-steering-convergence)
          (vector +marlin-graph-engine-abi-version+ #t (+ actor-epoch 1)
                  'running 'accepted)))
       ((eq? receipt-status 'settled)
        (if (> pending-steering 0)
          (vector +marlin-graph-engine-abi-version+ #f actor-epoch
                  actor-status 'cannot-settle-with-pending-steering)
          (vector +marlin-graph-engine-abi-version+ #t (+ actor-epoch 1)
                  'settled 'settled)))
       (else
        (vector +marlin-graph-engine-abi-version+ #t (+ actor-epoch 1)
                'failed 'execution-failed))))))

;;; Project a 3-slot Workbench tab request vector:
;;;   #(focused-task-id active-task-vectors peek-task-ids)
;;; where each active-task entry is #(task-id title status scope-mode fencing-epoch)
;;; Returns a 3-slot rooted ABI vector:
;;;   #(version focused-task-id #(tab-entry ...))
;;; with slot 0 always pinned to the Task Index and read-only peeks strictly
;;; disjoint from active task tabs.
(def (marlin-graph-engine-project-workbench-tabs-vector input)
  (let* ((focused (and (vector? input) (= (vector-length input) 3)
                       (let ((f (vector-ref input 0))) (and (non-empty-string? f) f))))
         (active-vec (if (and (vector? input) (= (vector-length input) 3)
                              (vector? (vector-ref input 1)))
                       (vector-ref input 1)
                       '#()))
         (peek-vec (if (and (vector? input) (= (vector-length input) 3)
                            (vector? (vector-ref input 2)))
                     (vector-ref input 2)
                     '#()))
         (active-seen (make-hash-table))
         (pinned-index (vector 'pinned-index "index" "Tasks" 'pinned #f 0))
         (active-tabs
          (let loop ((i 0) (acc '()))
            (if (>= i (vector-length active-vec))
              (reverse acc)
              (let ((row (vector-ref active-vec i)))
                (if (and (vector? row) (= (vector-length row) 5)
                         (non-empty-string? (vector-ref row 0))
                         (not (hash-key? active-seen (vector-ref row 0))))
                  (let* ((tid (vector-ref row 0))
                         (title (if (non-empty-string? (vector-ref row 1)) (vector-ref row 1) tid))
                         (status (if (symbol? (vector-ref row 2)) (vector-ref row 2) 'running))
                         (epoch (if (fixnum? (vector-ref row 4)) (vector-ref row 4) 1))
                         (is-focused? (and focused (string=? focused tid))))
                    (hash-put! active-seen tid #t)
                    (loop (+ i 1)
                          (cons (vector 'active-task tid title status is-focused? epoch) acc)))
                  (loop (+ i 1) acc))))))
         (peek-seen (make-hash-table))
         (peek-tabs
          (let loop ((i 0) (acc '()))
            (if (>= i (vector-length peek-vec))
              (reverse acc)
              (let ((tid (vector-ref peek-vec i)))
                (if (and (non-empty-string? tid)
                         (not (hash-key? active-seen tid))
                         (not (hash-key? peek-seen tid)))
                  (begin
                    (hash-put! peek-seen tid #t)
                    (loop (+ i 1)
                          (cons (vector 'read-only-peek tid tid 'peek #f 0) acc)))
                  (loop (+ i 1) acc)))))))
    (vector +marlin-graph-engine-abi-version+
            focused
            (list->vector (cons pinned-index (append active-tabs peek-tabs))))))

(def +allowed-disclosure-depths+ '(L0 L1 L2 L3))
(def +allowed-detour-modes+ '(mainline blocker-active side-quest-active))

(def (normalize-disclosure-depth v)
  (cond
   ((memq v '(L0 l0 0)) 'L0)
   ((memq v '(L1 l1 1)) 'L1)
   ((memq v '(L2 l2 2)) 'L2)
   ((memq v '(L3 l3 3)) 'L3)
   ((string? v)
    (let ((u (string-upcase v)))
      (cond ((string=? u "L0") 'L0)
            ((string=? u "L1") 'L1)
            ((string=? u "L2") 'L2)
            ((string=? u "L3") 'L3)
            (else v))))
   (else v)))

(def (disclosure-depth-rank depth)
  (case depth
    ((L0) 0)
    ((L1) 1)
    ((L2) 2)
    ((L3) 3)
    (else -1)))

;;; Compose Lab `GraphEngine`'s governed `ai-agentic-context` trunk assembly
;;; (Two-Zone KV-Prefix digest, MINJA quarantine, L3 active-defeater floor,
;;; Nono Lethal-Trifecta sandbox gate, inert Cedar handoff) with Marlin's
;;; Design 165 runtime detour overlay (`blocker-active` -> `auto-archive: #f`),
;;; returning a pure POO governed-context bundle + ready 13-slot C-ABI vector.
(def (marlin-graph-engine-assemble-governed-context
      task-id
      trunk-id
      apex-obligation-id
      obligation-graph
      capability-envelope
      org-text
      frozen-entries: (frozen-entries '())
      memory-candidates: (memory-candidates '())
      cedar-handoff: (cedar-handoff #f)
      sandbox-admission: (sandbox-admission #f)
      detour-mode: (detour-mode 'mainline))
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (make-governed-trunk (.ref lab-exports 'make-governed-context-trunk))
         (bundle (make-governed-trunk
                  trunk-id
                  apex-obligation-id
                  obligation-graph
                  capability-envelope
                  org-text
                  frozen-entries: frozen-entries
                  memory-candidates: memory-candidates
                  cedar-handoff: cedar-handoff
                  sandbox-admission: sandbox-admission))
         (assembly (marlin-converge-assembly '(env durable ui-workbench context-system)))
         (overlay (marlin-converge-runtime-overlay
                   assembly
                   (list (cons 'durable (.o bind-these: #f detour-state: detour-mode)))))
         (ctx-resolved (.ref (.ref overlay 'effective-configs) 'context-system))
         (auto-archive? (and (object? ctx-resolved)
                             (.slot? ctx-resolved 'auto-archive)
                             (.ref ctx-resolved 'auto-archive)))
         (cut-val (.ref obligation-graph 'cut))
         (kv-dig (.ref bundle 'kv-prefix-digest))
         (depth (normalize-disclosure-depth (.ref bundle 'effective-disclosure-depth)))
         (quarantined (let ((q (.ref bundle 'quarantined-excluded)))
                        (cond ((fixnum? q) q)
                              ((list? q) (length q))
                              (else 0))))
         (has-defeater? (let loop ((obs (.ref obligation-graph 'obligations)))
                          (and (pair? obs)
                               (or (pair? (.ref (car obs) 'active-defeaters))
                                   (loop (cdr obs))))))
         (sandbox-ok? (or (not sandbox-admission) (.ref sandbox-admission 'admitted?)))
         (cedar-ok? (or (not cedar-handoff) (.ref cedar-handoff 'policy-valid?)))
         (abi-vector (marlin-graph-engine-project-governed-context-vector
                      (vector task-id cut-val kv-dig depth has-defeater?
                              quarantined sandbox-ok? cedar-ok?
                              detour-mode auto-archive?))))
    (.o bind-these: #f
        kind: 'marlin.graph-engine.governed-context.v1
        admitted?: (and (.ref bundle 'admitted?) (vector-ref abi-vector 1))
        reason: (or (vector-ref abi-vector 12) (.ref bundle 'reason))
        trunk-bundle: bundle
        design165-overlay: overlay
        auto-archive-enabled?: auto-archive?
        abi-vector: abi-vector
        runtime-executed?: #f)))

;;; Project a 10-slot governed-context request vector:
;;;   #(task-id context-cut kv-prefix-digest disclosure-depth has-active-defeater?
;;;     quarantined-count sandbox-admitted? cedar-handoff-valid? detour-mode auto-archive?)
;;; into a 13-slot rooted ABI vector:
;;;   #(version admitted? task-id context-cut kv-prefix-digest disclosure-depth
;;;     has-active-defeater? quarantined-count sandbox-admitted? cedar-handoff-valid?
;;;     detour-mode auto-archive? reason)
(def (marlin-graph-engine-project-governed-context-vector input)
  (if (not (and (vector? input) (= (vector-length input) 10)))
    (vector +marlin-graph-engine-abi-version+ #f "" 0 "" 'L0 #f 0 #f #f 'mainline #f
            "invalid-governed-context-vector")
    (let ((task-id (vector-ref input 0))
          (context-cut (vector-ref input 1))
          (kv-digest (vector-ref input 2))
          (depth (vector-ref input 3))
          (has-defeater? (vector-ref input 4))
          (quarantined (vector-ref input 5))
          (sandbox-ok? (vector-ref input 6))
          (cedar-ok? (vector-ref input 7))
          (detour-mode (vector-ref input 8))
          (auto-archive? (vector-ref input 9)))
      (let ((reason
             (cond
              ((not (non-empty-string? task-id)) "empty-task-id")
              ((not (and (fixnum? context-cut) (> context-cut 0))) "invalid-context-cut")
              ((not (non-empty-string? kv-digest)) "empty-kv-prefix-digest")
              ((not (memq depth +allowed-disclosure-depths+)) "unsupported-disclosure-depth")
              ((not (boolean? has-defeater?)) "invalid-defeater-flag")
              ((not (and (fixnum? quarantined) (>= quarantined 0))) "invalid-quarantined-count")
              ((not (boolean? sandbox-ok?)) "invalid-sandbox-flag")
              ((not (boolean? cedar-ok?)) "invalid-cedar-flag")
              ((not (memq detour-mode +allowed-detour-modes+)) "unsupported-detour-mode")
              ((not (boolean? auto-archive?)) "invalid-auto-archive-flag")
              ((and has-defeater? (< (disclosure-depth-rank depth) 3))
               "active-defeater-requires-l3-disclosure")
              ((not sandbox-ok?) "sandbox-lethal-trifecta-denied")
              ((not cedar-ok?) "invalid-cedar-policy-handoff")
              ((and (eq? detour-mode 'blocker-active) auto-archive?)
               "blocker-detour-must-disable-auto-archive")
              (else #f))))
        (if reason
          (vector +marlin-graph-engine-abi-version+ #f
                  (if (string? task-id) task-id "")
                  (if (fixnum? context-cut) context-cut 0)
                  (if (string? kv-digest) kv-digest "")
                  (if (symbol? depth) depth 'L0)
                  (and (boolean? has-defeater?) has-defeater?)
                  (if (fixnum? quarantined) quarantined 0)
                  (and (boolean? sandbox-ok?) sandbox-ok?)
                  (and (boolean? cedar-ok?) cedar-ok?)
                  (if (symbol? detour-mode) detour-mode 'mainline)
                  (and (boolean? auto-archive?) auto-archive?)
                  reason)
          (vector +marlin-graph-engine-abi-version+ #t
                  task-id context-cut kv-digest depth has-defeater?
                  quarantined sandbox-ok? cedar-ok? detour-mode auto-archive? #f))))))

;;; ---------------------------------------------------------------------------
;;; Thin Atomic Effect C-ABI Vector Bridge (`scheme/graph-engine/funcs.ss`)
;;; ---------------------------------------------------------------------------
;;; Encodes Scheme's 4 canonical `AtomicEffect.` POO objects into rooted Scheme
;;; vectors for Marlin's thin Rust `effect_pump.rs`, and decodes raw Rust
;;; `AtomicEffectReceipt` vectors back into POO receipts for `drive-graph-actor-step`.

(def (marlin-graph-engine-atomic-effect->vector eff)
  (let ((k (.ref eff 'kind))
        (id (.ref eff 'effect-id)))
    (cond
     ((eq? k 'cas-get)
      (vector 'cas-get id (.ref eff 'key)))
     ((eq? k 'cas-put)
      (vector 'cas-put id (.ref eff 'key) (value->config-string (.ref eff 'payload))))
     ((eq? k 'llm-complete)
      (let ((msg-vecs
             (list->vector
              (map (lambda (m) (vector (car m) (cadr m)))
                   (.ref eff 'messages)))))
        (vector 'llm-complete id (.ref eff 'endpoint) msg-vecs)))
     ((eq? k 'sandbox-run)
      (vector 'sandbox-run id (.ref eff 'program)
              (list->vector (opt-ref eff 'args '()))
              (opt-ref eff 'cwd #f)))
     (else
      (error "unsupported atomic effect kind for C-ABI vector" k)))))

(def (marlin-graph-engine-vector->atomic-receipt vec)
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (k (vector-ref vec 0))
         (id (vector-ref vec 1)))
    (cond
     ((eq? k 'cas-hit)
      ((.ref lab-exports 'make-cas-hit-receipt) id (vector-ref vec 2)))
     ((eq? k 'cas-miss)
      ((.ref lab-exports 'make-cas-miss-receipt) id))
     ((eq? k 'cas-stored)
      ((.ref lab-exports 'make-cas-stored-receipt) id))
     ((eq? k 'llm-completed)
      ((.ref lab-exports 'make-llm-completed-receipt) id (vector-ref vec 2)))
     ((eq? k 'sandbox-exited)
      ((.ref lab-exports 'make-sandbox-exited-receipt)
       id (vector-ref vec 2) (vector-ref vec 3) stderr: (vector-ref vec 4)))
     (else
      ((.ref lab-exports 'make-effect-error-receipt) id (vector-ref vec 2))))))

;;; Helper constructing a live rooted Scheme `UnifiedDriverState.` (1 pure-cas
;;; deterministic task + 2 competing hypotheses x 2 branches) from `#(task-id title kv-digest)`.
(def (marlin-graph-engine-make-demo-driver-state-vector input)
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (tid (vector-ref input 0))
         (title (vector-ref input 1))
         (kv-digest (vector-ref input 2))
         (ob-mod (.ref lab-exports 'make-context-trunk))
         (make-actor (.ref lab-exports 'make-task-actor))
         (make-hyp (.ref lab-exports 'make-competing-hypothesis))
         (make-br (.ref lab-exports 'make-hypothesis-branch-proposal))
         (make-st (.ref lab-exports 'make-unified-driver-state))
         (env (make-capability-envelope
               "vm-owner"
               allowed-caps: '(context-read)
               denied-caps: '(host-egress)
               egress-allowed?: #f))
         (ob (make-obligation
              "ob-vm-1"
              title: "Verify VM Task"
              required-evidence-ids: '("ev-vm-det")))
         (det (make-deterministic-task-node
               "det-vm-cas-1"
               "ob-vm-1"
               "ev-vm-det"
               'funflow-vm-cas
               "vm-in"
               effect-class: 'pure-cas
               read-footprint: '("vm.ss")
               write-footprint: '()))
         (g0 (make-obligation-graph
              "g-vm-driver"
              cut: 1
              obligations: (list ob)
              evidence: '()
              det-tasks: (list det)))
         (trunk (ob-mod "trunk-vm-driver" "ob-vm-1" g0 env
                        shared-context-cut: 1
                        kv-prefix-digest: kv-digest))
         (actor (make-actor tid trunk '("scope-main") title: title))
         (ctx (.o bind-these: #f
                  context-cut: 1
                  kv-prefix-digest: kv-digest
                  disclosure-depth: 'L3
                  has-active-defeater?: #f
                  sandbox-admitted?: #t
                  cedar-handoff-valid?: #t))
         (h1 (make-hyp "H1" "Hypothesis Alpha" contradictions: '("c1") open-questions: '("q1")))
         (h2 (make-hyp "H2" "Hypothesis Beta" contradictions: '() open-questions: '("q2")))
         (b1 (make-br "B1" "Check trace A"))
         (b2 (make-br "B2" "Check trace B")))
    (make-st actor g0 ctx
             competing-hypotheses: (list h1 h2)
             branch-proposals: (list b1 b2))))

;;; Thin C-ABI driver step:
;;; Input:  `#(driver-state #(receipt-vec ...))`
;;; Output: `#(next-driver-state phase-sym llm-calls-int cas-hits-int cas-puts-int #(effect-vec ...))`
(def (marlin-graph-engine-step-driver-vector input)
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (drive (.ref lab-exports 'drive-graph-actor-step))
         (st (vector-ref input 0))
         (rcpt-vecs (vector->list (vector-ref input 1)))
         (rcpts (map marlin-graph-engine-vector->atomic-receipt rcpt-vecs))
         (step (drive st rcpts))
         (eff-vecs (list->vector (map marlin-graph-engine-atomic-effect->vector (.ref step 'effects)))))
    (vector (.ref step 'state)
            (.ref step 'phase)
            (.ref step 'llm-calls-emitted)
            (opt-ref step 'cas-hits-observed 0)
            (opt-ref step 'cas-puts-emitted 0)
            eff-vecs)))

;;; Construct a live 2-actor Fleet inside Gambit heap from `#(parent-id sib-id kv-digest)`:
;;;   - `parent-id`: active actor with a `defeater-discovered` bitemporal event (blocker-active + L3 + human-inbox)
;;;   - `sib-id`: active sibling actor with a `pure-cas` deterministic task ready to run & auto-archive!
(def (marlin-graph-engine-make-demo-fleet-vector input)
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (parent-id (vector-ref input 0))
         (sib-id (vector-ref input 1))
         (kv-digest (vector-ref input 2))
         (make-trunk (.ref lab-exports 'make-context-trunk))
         (make-actor (.ref lab-exports 'make-task-actor))
         (enter-detour (.ref lab-exports 'enter-task-actor-detour))
         (make-st (.ref lab-exports 'make-unified-driver-state))
         (make-fleet (.ref lab-exports 'make-graph-fleet-state))
         (spawn-sib (.ref lab-exports 'fleet-spawn-sibling-from-detour))
         (apply-ev (.ref lab-exports 'fleet-apply-bitemporal-event))
         (env (make-capability-envelope "vm-fleet" allowed-caps: '(context-read) denied-caps: '(host-egress) egress-allowed?: #f))
         (ctx (.o bind-these: #f
                  context-cut: 1
                  kv-prefix-digest: kv-digest
                  disclosure-depth: 'L1
                  has-active-defeater?: #f
                  sandbox-admitted?: #t
                  cedar-handoff-valid?: #t))
         (ob-p (make-obligation "ob-p" title: "Parent Gate" required-evidence-ids: '("ev-p")))
         (g-p (make-obligation-graph "g-p" cut: 1 obligations: (list ob-p) evidence: '() det-tasks: '()))
         (tr-p (make-trunk "tr-p" "ob-p" g-p env shared-context-cut: 1 kv-prefix-digest: kv-digest))
         (p0 (make-actor parent-id tr-p '("scope-main") title: "Turso Fleet Parent"))
         (p-sq (.ref (enter-detour p0 'side-quest-active "sq-vm-split") 'actor))
         (fl0 (make-fleet (list (make-st p-sq g-p ctx)) peek-set: (list sib-id)))
         (ob-s (make-obligation "ob-s" title: "Sibling Task" required-evidence-ids: '("ev-s")))
         (det-s (make-deterministic-task-node
                 "det-turso-sib" "ob-s" "ev-s" 'funflow-turso "in-s"
                 effect-class: 'pure-cas read-footprint: '("sib.ss") write-footprint: '()))
         (g-s (make-obligation-graph "g-s" cut: 1 obligations: (list ob-s) evidence: '() det-tasks: (list det-s)))
         (fl1 (spawn-sib fl0 parent-id sib-id "Turso Fleet Sibling" g-s sibling-apex-id: "ob-s"))
         (def-ev (.o bind-these: #f
                     event-kind: 'defeater-discovered
                     obligation-id: "ob-p"
                     defeater-id: "def-turso-01")))
    (apply-ev fl1 parent-id def-ev)))

;;; Thin C-ABI multi-actor Fleet step:
;;; Input:  `#(fleet #(receipt-vec ...) auto-archive?)`
;;; Output: `#(next-fleet #(active-ids...) #(settled-ids...) llm-calls cas-hits cas-puts #(effect-vec ...))`
(def (marlin-graph-engine-step-fleet-vector input)
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (step-fleet (.ref lab-exports 'step-graph-fleet))
         (fl (vector-ref input 0))
         (rcpts (map marlin-graph-engine-vector->atomic-receipt (vector->list (vector-ref input 1))))
         (auto-arch? (vector-ref input 2))
         (res (step-fleet fl rcpts auto-archive?: auto-arch?))
         (next-fl (.ref res 'fleet))
         (active-ids (list->vector (map (lambda (st) (.ref (.ref st 'actor) 'task-id))
                                        (.ref next-fl 'active-drivers))))
         (settled-ids (list->vector (map (lambda (a) (.ref a 'task-id))
                                         (.ref next-fl 'archived-actors))))
         (eff-vecs (list->vector (map marlin-graph-engine-atomic-effect->vector (.ref res 'effects)))))
    (vector next-fl
            active-ids
            settled-ids
            (.ref res 'llm-calls-emitted)
            (.ref res 'cas-hits-observed)
            (.ref res 'cas-puts-emitted)
            eff-vecs)))

;;; Thin C-ABI Fleet checkpoint: `#(fleet cas-key)` -> `#(cas-put id key wire-utf8-string)`
(def (marlin-graph-engine-fleet-checkpoint-vector input)
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (plan-cp (.ref lab-exports 'plan-fleet-cas-checkpoint))
         (eff (plan-cp (vector-ref input 0) (vector-ref input 1))))
    (marlin-graph-engine-atomic-effect->vector eff)))

;;; Thin C-ABI Fleet restore: `#(cas-hit id wire-utf8-string)` -> live `GraphFleetState.`
(def (marlin-graph-engine-fleet-restore-vector input)
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (restore (.ref lab-exports 'restore-fleet-from-cas-receipt))
         (rcpt (marlin-graph-engine-vector->atomic-receipt input)))
    (restore rcpt)))

;;; Thin C-ABI Fleet summary vector for cross-restart assertions:
;;; `fleet` -> `#(#(active-ids...) #(settled-ids...) #(detour-modes...) #(defeater-counts...) #(inbox-counts...) #(visible-tab-ids...))`
(def (marlin-graph-engine-fleet-summary-vector fleet)
  (let* ((lab-exports (.get LabGraphEngine core exports))
         (proj-fn (.ref lab-exports 'project-fleet->workbench-projection))
         (proj (proj-fn fleet))
         (active-drivers (.ref fleet 'active-drivers))
         (active-ids (list->vector (map (lambda (st) (.ref (.ref st 'actor) 'task-id)) active-drivers)))
         (settled-ids (list->vector (map (lambda (a) (.ref a 'task-id)) (.ref fleet 'archived-actors))))
         (detours (list->vector (map (lambda (st) (opt-ref (.ref st 'actor) 'detour-mode 'mainline)) active-drivers)))
         (defeaters (list->vector
                     (map (lambda (st)
                            (let loop ((obs (.ref (.ref st 'obligation-graph) 'obligations)) (acc 0))
                              (if (null? obs) acc (loop (cdr obs) (+ acc (length (.ref (car obs) 'active-defeaters)))))))
                          active-drivers)))
         (inboxes (list->vector (map (lambda (st) (length (opt-ref (.ref st 'obligation-graph) 'human-inbox '())))
                                     active-drivers)))
         (vis-tabs (list->vector (map value->config-string (.ref proj 'visible-tab-ids)))))
    (vector active-ids settled-ids detours defeaters inboxes vis-tabs)))

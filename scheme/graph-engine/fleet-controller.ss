;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Round S6: 100% Pure Scheme Live Fleet <-> Declarative TUI Bi-Directional Controller.
;;;
;;; Lives under `scheme/graph-engine/` (NEVER imported by `scheme/ui/session.ss` or
;;; `scheme/ui-tui/entry.ss` so `MAR-UI-R08` first-frame TUI startup stays lightweight).
;;;
;;; Connects declarative Task Workbench `scheme-action` commands (`SPC o 0..3`,
;;; `SPC o d b/s/r/a/n/x`, `SPC o h a`, `SPC o f s`, `SPC o f c`) directly to a live
;;; `GraphFleetState.`, updating real `TaskActor.` / `ObligationGraph.` state,
;;; emitting inert `[AtomicEffect...]` batches for Marlin's thin 4-primitive Rust
;;; `effect_pump`, and immediately re-projecting the checked `UiScope` + Ratatui
;;; vector with ZERO Rust UI changes!

(import (only-in :clan/poo/object .get .o .ref .slot? object?)
        (only-in :std/list/list find)
        (rename-in (only-in :tao3k/agentic-ai-lab/modules/graph-engine/interface
                            GraphEngine
                            make-obligation
                            make-obligation-graph)
                   (GraphEngine LabGraphEngine))
        (only-in :marlin-agent-core/ui/session
                 make-marlin-task-workbench-package
                 marlin-ui-session-open
                 marlin-ui-session-project
                 marlin-ui-session-dispatch
                 marlin-task-workbench-next-disclosure)
        (only-in "./funcs.ss"
                 marlin-graph-engine-atomic-effect->vector
                 marlin-graph-engine-vector->atomic-receipt))

(export make-marlin-fleet-workbench-controller
        marlin-fleet-workbench-controller-apply-action
        marlin-fleet-workbench-controller-project
        marlin-fleet-workbench-controller-open-vector
        marlin-fleet-workbench-controller-dispatch-vector)

(def (opt-ref obj key default)
  (if (and (object? obj) (.slot? obj key))
    (.ref obj key)
    default))

(def (lab-fn name)
  (.ref (.get LabGraphEngine core exports) name))

(def (open-fleet-session fleet disclosure model-label session-label)
  (let* ((proj ((lab-fn 'project-fleet->workbench-projection) fleet))
         (pkg-ui (make-marlin-task-workbench-package proj disclosure-level: disclosure))
         (sess (marlin-ui-session-open
                (vector pkg-ui (vector model-label session-label '#() '#()) '#()))))
    (values proj sess)))

(def (build-controller fleet disclosure model-label session-label last-effects)
  (let-values (((proj sess) (open-fleet-session fleet disclosure model-label session-label)))
    (.o bind-these: #f
        kind: 'marlin.graph-engine.fleet-workbench-controller.v1
        fleet: fleet
        projection: proj
        disclosure-level: disclosure
        model-label: model-label
        session-label: session-label
        last-effects: last-effects
        session: sess
        authorizes-effect?: #f)))

(def (make-marlin-fleet-workbench-controller
      fleet
      disclosure-level: (disclosure-level 'L1)
      model-label: (model-label "gemini-2.5-pro")
      session-label: (session-label "fleet-workbench")
      last-effects: (last-effects '()))
  (let ((disc (marlin-task-workbench-next-disclosure disclosure-level disclosure-level)))
    (build-controller fleet disc model-label session-label last-effects)))

(def (resolve-target-task-id fleet explicit-task-id)
  (let ((drivers (.ref fleet 'active-drivers)))
    (cond
     (explicit-task-id explicit-task-id)
     ((let ((foc (.ref fleet 'focused-task-id)))
        (and foc
             (find (lambda (st) (equal? (.ref (.ref st 'actor) 'task-id) foc)) drivers)
             foc)))
     ((pair? drivers) (.ref (.ref (car drivers) 'actor) 'task-id))
     (else #f))))

(def (first-active-defeater-for fleet task-id)
  (let ((st (find (lambda (d) (equal? (.ref (.ref d 'actor) 'task-id) task-id))
                  (.ref fleet 'active-drivers))))
    (and st
         (let loop ((obs (.ref (.ref st 'obligation-graph) 'obligations)))
           (cond
            ((null? obs) #f)
            ((pair? (.ref (car obs) 'active-defeaters))
             (car (.ref (car obs) 'active-defeaters)))
            (else (loop (cdr obs))))))))

(def (replace-driver-actor fleet task-id actor-fn)
  (let* ((make-st (lab-fn 'make-unified-driver-state))
         (make-fleet (lab-fn 'make-graph-fleet-state))
         (next-drivers
          (map (lambda (st)
                 (if (not (equal? (.ref (.ref st 'actor) 'task-id) task-id))
                   st
                   (let ((next-actor (actor-fn (.ref st 'actor))))
                     (make-st
                      next-actor
                      (.ref st 'obligation-graph)
                      (.ref st 'governed-context)
                      competing-hypotheses: (.ref st 'competing-hypotheses)
                      branch-proposals: (.ref st 'branch-proposals)
                      hypothesis-matrix: (opt-ref st 'hypothesis-matrix #f)
                      completed-step-outputs: (opt-ref st 'completed-step-outputs '())
                      step-count: (opt-ref st 'step-count 0)
                      repair-budget: (opt-ref st 'repair-budget 3)
                      repair-history: (opt-ref st 'repair-history '())))))
               (.ref fleet 'active-drivers))))
    (make-fleet
     next-drivers
     archived-actors: (.ref fleet 'archived-actors)
     peek-set: (.ref fleet 'peek-set)
     focused-task-id: (.ref fleet 'focused-task-id)
     round-count: (.ref fleet 'round-count))))

(def (marlin-fleet-workbench-controller-apply-action
      controller
      action-sym
      task-id: (task-id #f)
      arg: (arg #f)
      receipts: (receipts '())
      auto-archive?: (auto-archive? #t))
  (let* ((fleet (.get controller fleet))
         (disc (.get controller disclosure-level))
         (model-label (.get controller model-label))
         (session-label (.get controller session-label))
         (last-effs (.get controller last-effects))
         (sess (.get controller session))
         (target (resolve-target-task-id fleet task-id)))
    (cond
     ((memq action-sym '(task-drill-l0 task-drill-l1 task-drill-l2 task-drill-l3))
      (let ((next-disc (case action-sym
                         ((task-drill-l0) 'L0)
                         ((task-drill-l1) 'L1)
                         ((task-drill-l2) 'L2)
                         (else 'L3))))
        (build-controller fleet next-disc model-label session-label last-effs)))

     ((eq? action-sym 'task-human-approve)
      (let* ((def-id (or (and (string? arg) (> (string-length arg) 0) arg)
                         (and target (first-active-defeater-for fleet target))))
             (ev (.o bind-these: #f
                     event-kind: 'human-approved
                     resolve-defeater-id: def-id))
             (next-fleet (if target
                           ((lab-fn 'fleet-apply-bitemporal-event) fleet target ev)
                           fleet)))
        (build-controller next-fleet disc model-label session-label '())))

     ((memq action-sym '(task-detour-blocker task-detour-sidequest))
      (let* ((mode (if (eq? action-sym 'task-detour-blocker) 'blocker-active 'side-quest-active))
             (prefix (if (eq? mode 'blocker-active) "blk-" "sq-"))
             (enter-detour (lab-fn 'enter-task-actor-detour))
             (next-fleet
              (if target
                (replace-driver-actor
                 fleet target
                 (lambda (actor)
                   (.ref (enter-detour
                          actor mode
                          (or (and (string? arg) (> (string-length arg) 0) arg)
                              (string-append prefix target)))
                         'actor)))
                fleet)))
        (build-controller next-fleet disc model-label session-label '())))

     ((memq action-sym '(task-detour-exit-return task-detour-exit-absorb task-detour-exit-abort))
      (let* ((exit-kind (case action-sym
                          ((task-detour-exit-return) 'return-to-origin)
                          ((task-detour-exit-absorb) 'absorb-and-continue)
                          (else 'abort-detour)))
             (exit-detour (lab-fn 'exit-task-actor-detour))
             (next-fleet
              (if target
                (replace-driver-actor
                 fleet target
                 (lambda (actor)
                   (.ref (exit-detour actor exit-kind) 'actor)))
                fleet)))
        (build-controller next-fleet disc model-label session-label '())))

     ((eq? action-sym 'task-detour-exit-sibling)
      (if (not target)
        controller
        (let* ((sib-id (if (and (string? arg) (> (string-length arg) 0))
                         arg
                         (string-append target "-sibling")))
               (parent-st (find (lambda (d) (equal? (.ref (.ref d 'actor) 'task-id) target))
                                (.ref fleet 'active-drivers)))
               (parent-cut (if parent-st (.ref (.ref parent-st 'obligation-graph) 'cut) 1))
               (sib-apex (string-append "ob-" sib-id))
               (sib-ob (make-obligation sib-apex title: (string-append "Sibling " sib-id)
                                        required-evidence-ids: '()))
               (sib-g (make-obligation-graph
                       (string-append "g-" sib-id)
                       cut: parent-cut
                       obligations: (list sib-ob)
                       evidence: '()
                       det-tasks: '()))
               (next-fleet
                ((lab-fn 'fleet-spawn-sibling-from-detour)
                 fleet target sib-id (string-append "Sibling of " target) sib-g
                 sibling-apex-id: sib-apex)))
          (build-controller next-fleet disc model-label session-label '()))))

     ((eq? action-sym 'task-fleet-step)
      (let* ((step ((lab-fn 'step-graph-fleet) fleet receipts auto-archive?: auto-archive?))
             (next-fleet (.ref step 'fleet))
             (emitted (.ref step 'effects)))
        (build-controller next-fleet disc model-label session-label emitted)))

     ((eq? action-sym 'task-fleet-checkpoint)
      (let* ((cas-key (if (and (string? arg) (> (string-length arg) 0))
                        arg
                        "fleet:checkpoint:live"))
             (cp-eff ((lab-fn 'plan-fleet-cas-checkpoint) fleet cas-key)))
        (.o bind-these: #f
            kind: 'marlin.graph-engine.fleet-workbench-controller.v1
            fleet: fleet
            projection: (.get controller projection)
            disclosure-level: disc
            model-label: model-label
            session-label: session-label
            last-effects: (list cp-eff)
            session: sess
            authorizes-effect?: #f)))

     (else
      (let* ((step (marlin-ui-session-dispatch
                    (vector sess 'scheme-action (vector action-sym arg))))
             (next-sess (vector-ref step 0)))
        (.o bind-these: #f
            kind: 'marlin.graph-engine.fleet-workbench-controller.v1
            fleet: fleet
            projection: (.get controller projection)
            disclosure-level: disc
            model-label: model-label
            session-label: session-label
            last-effects: last-effs
            session: next-sess
            authorizes-effect?: #f))))))

(def (marlin-fleet-workbench-controller-project controller)
  (marlin-ui-session-project (.get controller session)))

;;; ---------------------------------------------------------------------------
;;; Thin C-ABI Rooted-Vector Entrypoints for Embedded Gerbil VM (`Round S7`)
;;; ---------------------------------------------------------------------------

(def (tab-id->str v)
  (cond
   ((string? v) v)
   ((symbol? v) (symbol->string v))
   (else "unknown")))

;;; Input:  `#(fleet disclosure-sym)`
;;; Output: rooted `FleetWorkbenchController.`
(def (marlin-fleet-workbench-controller-open-vector input)
  (make-marlin-fleet-workbench-controller
   (vector-ref input 0)
   disclosure-level: (vector-ref input 1)))

;;; Input:  `#(controller action-sym task-id-or-#f arg-or-#f #(receipt-vec ...) auto-archive?)`
;;; Output: `#(next-controller disclosure-sym #(effect-vec ...) #(visible-tab-ids...) #(detour-modes...) #(defeater-counts...) ratatui-projection-vec)`
(def (marlin-fleet-workbench-controller-dispatch-vector input)
  (let* ((ctrl (vector-ref input 0))
         (action-sym (vector-ref input 1))
         (tid (vector-ref input 2))
         (arg (vector-ref input 3))
         (rcpts (map marlin-graph-engine-vector->atomic-receipt
                     (vector->list (vector-ref input 4))))
         (auto-arch? (vector-ref input 5))
         (next-ctrl (marlin-fleet-workbench-controller-apply-action
                     ctrl action-sym
                     task-id: tid
                     arg: arg
                     receipts: rcpts
                     auto-archive?: auto-arch?))
         (next-fleet (.get next-ctrl fleet))
         (active-drivers (.ref next-fleet 'active-drivers))
         (eff-vecs (list->vector (map marlin-graph-engine-atomic-effect->vector
                                      (.get next-ctrl last-effects))))
         (vis-tabs (list->vector (map tab-id->str
                                      (.ref (.get next-ctrl projection) 'visible-tab-ids))))
         (detours (list->vector (map (lambda (st) (opt-ref (.ref st 'actor) 'detour-mode 'mainline))
                                     active-drivers)))
         (defeaters (list->vector
                     (map (lambda (st)
                            (let loop ((obs (.ref (.ref st 'obligation-graph) 'obligations)) (acc 0))
                              (if (null? obs)
                                acc
                                (loop (cdr obs) (+ acc (length (.ref (car obs) 'active-defeaters)))))))
                          active-drivers)))
         (ui-vec (marlin-fleet-workbench-controller-project next-ctrl)))
    (vector next-ctrl
            (.get next-ctrl disclosure-level)
            eff-vecs
            vis-tabs
            detours
            defeaters
            ui-vec)))

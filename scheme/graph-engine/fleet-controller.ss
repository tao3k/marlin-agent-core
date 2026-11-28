;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; -*- Gerbil -*-
;;; Pure Scheme Live Fleet <-> Declarative TUI Bi-Directional Controller.
;;; Follows Style 02/03 (`(.o (:: @ Prototype.) ...)`, constant slot capture
;;; `(slot)` / `(slot := expr)`, and standard functional combinators).
;;; Lives under `scheme/graph-engine/` (NEVER imported by `scheme/ui/` or
;;; `scheme/ui-tui/entry.ss` so `MAR-UI-R08` first-frame startup stays lean).

(import (only-in :clan/poo/object .def .get .o .ref .slot? object?)
        (only-in :std/list/list find)
        (rename-in (only-in :tao3k/agentic-ai-lab/modules/graph-engine/interface
                            GraphEngine
                            make-capability-envelope
                            make-deterministic-task-node
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

(export FleetWorkbenchController.
        make-marlin-fleet-workbench-controller
        marlin-fleet-workbench-controller-apply-action
        marlin-fleet-workbench-controller-project
        marlin-fleet-workbench-controller-audit-capsule
        marlin-fleet-workbench-controller-open-vector
        marlin-fleet-workbench-controller-dispatch-vector
        marlin-fleet-workbench-controller-audit-capsule-vector)

(.def FleetWorkbenchController.
  kind: 'marlin.graph-engine.fleet-workbench-controller.v1
  authorizes-effect?: #f)

(def (opt-ref obj key default)
  (if (and (object? obj) (.slot? obj key)) (.ref obj key) default))

(def (non-empty-str? v) (and (string? v) (> (string-length v) 0)))

(def (lab-fn name) (.ref (.get LabGraphEngine core exports) name))

(def (split-colon raw default-left)
  (let ((idx (string-index raw #\:)))
    (if (and idx (> idx 0) (< (+ idx 1) (string-length raw)))
      (values (substring raw 0 idx) (substring raw (+ idx 1) (string-length raw)))
      (values default-left raw))))

(def (build-controller fleet disclosure-level model-label session-label last-effects)
  (let* ((projection ((lab-fn 'project-fleet->workbench-projection) fleet))
         (pkg-ui (make-marlin-task-workbench-package projection disclosure-level: disclosure-level))
         (session (marlin-ui-session-open
                   (vector pkg-ui (vector model-label session-label '#() '#()) '#()))))
    (.o (:: @ FleetWorkbenchController.)
        (fleet) (projection) (disclosure-level)
        (model-label) (session-label) (last-effects) (session))))

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
        (and foc (find (lambda (st) (equal? (.ref (.ref st 'actor) 'task-id) foc)) drivers) foc)))
     ((pair? drivers) (.ref (.ref (car drivers) 'actor) 'task-id))
     (else #f))))

(def (first-active-defeater-for fleet task-id)
  (let ((st (find (lambda (d) (equal? (.ref (.ref d 'actor) 'task-id) task-id))
                  (.ref fleet 'active-drivers))))
    (and st
         (let ((ob (find (lambda (o) (pair? (.ref o 'active-defeaters)))
                         (.ref (.ref st 'obligation-graph) 'obligations))))
           (and ob (car (.ref ob 'active-defeaters)))))))

(def (owner-task-id-for-defeater fleet def-id)
  (and (non-empty-str? def-id)
       (let ((st (find (lambda (d)
                         (find (lambda (o) (member def-id (.ref o 'active-defeaters)))
                               (.ref (.ref d 'obligation-graph) 'obligations)))
                       (.ref fleet 'active-drivers))))
         (and st (.ref (.ref st 'actor) 'task-id)))))

(def (replace-driver-actor fleet task-id actor-fn)
  (.o (:: @ fleet)
      (active-drivers :=
       (map (lambda (st)
              (if (equal? (.ref (.ref st 'actor) 'task-id) task-id)
                (.o (:: @ st) (actor := (actor-fn (.ref st 'actor))))
                st))
            (.ref fleet 'active-drivers)))))

(def (marlin-fleet-workbench-controller-apply-action
      controller action-sym
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
    (def (reproject next-fleet (next-disc disc) (effs '()))
      (build-controller next-fleet next-disc model-label session-label effs))
    (case action-sym
      ((task-drill-l0 task-drill-l1 task-drill-l2 task-drill-l3)
       (reproject fleet (case action-sym
                          ((task-drill-l0) 'L0) ((task-drill-l1) 'L1)
                          ((task-drill-l2) 'L2) (else 'L3))
                  last-effs))

      ((task-human-approve)
       (let* ((approve-target (or task-id
                                  (and (non-empty-str? arg) (owner-task-id-for-defeater fleet arg))
                                  target))
              (def-id (or (and (non-empty-str? arg) arg)
                          (and approve-target (first-active-defeater-for fleet approve-target)))))
         (reproject (if approve-target
                      ((lab-fn 'fleet-apply-bitemporal-event)
                       fleet approve-target (.o (event-kind := 'human-approved)
                                                (resolve-defeater-id := def-id)))
                      fleet))))

      ((task-propose-branch)
       (if (not target)
         controller
         (let* ((st (find (lambda (d) (equal? (.ref (.ref d 'actor) 'task-id) target))
                          (.ref fleet 'active-drivers)))
                (default-id (string-append "B" (number->string (+ (if st (length (.ref st 'branch-proposals)) 0) 1))))
                (raw (if (non-empty-str? arg) arg "Investigator branch inquiry")))
           (let-values (((bid question) (split-colon raw default-id)))
             (reproject ((lab-fn 'fleet-propose-hypothesis-branch)
                         fleet target ((lab-fn 'make-hypothesis-branch-proposal)
                                       bid question proposer: 'investigator)))))))

      ((task-propagate-defeater)
       (let-values (((def-id prem-id)
                     (split-colon (if (non-empty-str? arg) arg "def-shared:shared-premise")
                                  "def-shared")))
         (reproject ((lab-fn 'fleet-propagate-shared-defeater) fleet def-id prem-id))))

      ((task-peek)
       (let* ((archived (.ref fleet 'archived-actors))
              (peek-id (or (and (non-empty-str? arg) arg)
                           (and (pair? archived) (.ref (car archived) 'task-id)))))
         (if peek-id (reproject ((lab-fn 'fleet-peek-archived-actor) fleet peek-id)) controller)))

      ((task-reopen)
       (let* ((archived (.ref fleet 'archived-actors))
              (peek-set (.ref fleet 'peek-set))
              (reopen-id (or (and (non-empty-str? arg) arg)
                             (and (pair? peek-set) (car peek-set))
                             (and (pair? archived) (.ref (car archived) 'task-id)))))
         (if reopen-id (reproject ((lab-fn 'fleet-reopen-archived-actor) fleet reopen-id)) controller)))

      ((task-detour-blocker task-detour-sidequest)
       (let* ((mode (if (eq? action-sym 'task-detour-blocker) 'blocker-active 'side-quest-active))
              (node-id (if (non-empty-str? arg)
                         arg
                         (string-append (if (eq? mode 'blocker-active) "blk-" "sq-") (or target "task")))))
         (reproject (if target
                      (replace-driver-actor
                       fleet target
                       (lambda (actor) (.ref ((lab-fn 'enter-task-actor-detour) actor mode node-id) 'actor)))
                      fleet))))

      ((task-detour-exit-return task-detour-exit-absorb task-detour-exit-abort)
       (let ((exit-kind (case action-sym
                          ((task-detour-exit-return) 'return-to-origin)
                          ((task-detour-exit-absorb) 'absorb-and-continue)
                          (else 'abort-detour))))
         (reproject (if target
                      (replace-driver-actor
                       fleet target
                       (lambda (actor) (.ref ((lab-fn 'exit-task-actor-detour) actor exit-kind) 'actor)))
                      fleet))))

      ((task-detour-exit-sibling)
       (if (not target)
         controller
         (let* ((sib-id (if (non-empty-str? arg) arg (string-append target "-sibling")))
                (parent-st (find (lambda (d) (equal? (.ref (.ref d 'actor) 'task-id) target))
                                 (.ref fleet 'active-drivers)))
                (sib-apex (string-append "ob-" sib-id))
                (sib-g (make-obligation-graph
                        (string-append "g-" sib-id)
                        cut: (if parent-st (.ref (.ref parent-st 'obligation-graph) 'cut) 1)
                        obligations: (list (make-obligation sib-apex title: (string-append "Sibling " sib-id)
                                                            required-evidence-ids: '()))
                        evidence: '() det-tasks: '())))
           (reproject ((lab-fn 'fleet-spawn-sibling-from-detour)
                       fleet target sib-id (string-append "Sibling of " target) sib-g
                       sibling-apex-id: sib-apex)))))

      ((task-fleet-step)
       (let ((step ((lab-fn 'step-graph-fleet) fleet receipts auto-archive?: auto-archive?)))
         (reproject (.ref step 'fleet) disc (.ref step 'effects))))

      ((task-sandbox-verify)
       (let-values (((sib-id rest)
                     (split-colon (if (non-empty-str? arg) arg "sandbox-sib:/bin/echo:sandbox-ok")
                                  "sandbox-sib")))
         (let-values (((prog cmd-arg) (split-colon rest "/bin/echo")))
           (let* ((drivers (.ref fleet 'active-drivers))
                  (parent-st (or (and target
                                      (find (lambda (d) (equal? (.ref (.ref d 'actor) 'task-id) target))
                                            drivers))
                                 (and (pair? drivers) (car drivers))))
                  (cut (if parent-st (.ref (.ref parent-st 'obligation-graph) 'cut) 1))
                  (kv-digest (if parent-st
                               (opt-ref (.ref parent-st 'governed-context) 'kv-prefix-digest "sha256:sandbox-kv")
                               "sha256:sandbox-kv"))
                  (env (make-capability-envelope
                        (string-append "env-" sib-id)
                        allowed-caps: '(context-read)
                        denied-caps: '(host-egress)
                        egress-allowed?: #f))
                  (sib-apex (string-append "ob-" sib-id))
                  (sib-ev (string-append "ev-" sib-id))
                  (det (.o (:: @ (make-deterministic-task-node
                                  (string-append "det-" sib-id) sib-apex sib-ev
                                  'funflow-sandbox "sandbox-in"
                                  effect-class: 'fenced-effect
                                  read-footprint: '("sandbox.ss")
                                  write-footprint: '()))
                           (program := prog)
                           (args := (if (non-empty-str? cmd-arg) (list cmd-arg) '()))))
                  (sib-g (make-obligation-graph
                          (string-append "g-" sib-id)
                          cut: cut
                          obligations: (list (make-obligation sib-apex
                                                              title: (string-append "Sandbox Verify " sib-id)
                                                              required-evidence-ids: (list sib-ev)))
                          evidence: '()
                          det-tasks: (list det)))
                  (sib-tr ((lab-fn 'make-context-trunk)
                           (string-append "tr-" sib-id) sib-apex sib-g env
                           shared-context-cut: cut kv-prefix-digest: kv-digest))
                  (sib-actor ((lab-fn 'make-task-actor)
                              sib-id sib-tr '("scope-main")
                              title: (string-append "Sandbox Verify " sib-id)))
                  (sib-ctx (.o (context-cut := cut)
                               (kv-prefix-digest := kv-digest)
                               (disclosure-depth := 'L3)
                               (has-active-defeater? := #f)
                               (sandbox-admitted? := #t)
                               (cedar-handoff-valid? := #t)))
                  (sib-st ((lab-fn 'make-unified-driver-state) sib-actor sib-g sib-ctx))
                  (fleet1 (.o (:: @ fleet)
                              (active-drivers := (append drivers (list sib-st)))))
                  (step ((lab-fn 'step-graph-fleet) fleet1 '() auto-archive?: auto-archive?)))
             (reproject (.ref step 'fleet) disc (.ref step 'effects))))))

      ((task-fleet-checkpoint)
       (let ((cp-eff ((lab-fn 'plan-fleet-cas-checkpoint)
                      fleet (if (non-empty-str? arg) arg "fleet:checkpoint:live"))))
         (.o (:: @ controller) (last-effects := (list cp-eff)))))

      (else
       (let ((step (marlin-ui-session-dispatch (vector sess 'scheme-action (vector action-sym arg)))))
         (.o (:: @ controller) (session := (vector-ref step 0))))))))

(def (marlin-fleet-workbench-controller-project controller)
  (marlin-ui-session-project (.get controller session)))

(def (marlin-fleet-workbench-controller-audit-capsule controller)
  ((lab-fn 'fleet-export-audit-capsule) (.get controller fleet)))

;;; ---------------------------------------------------------------------------
;;; Thin C-ABI Rooted-Vector Entrypoints for Embedded Gerbil VM
;;; ---------------------------------------------------------------------------

(def (tab-id->str v)
  (cond ((string? v) v) ((symbol? v) (symbol->string v)) (else "unknown")))

(def (marlin-fleet-workbench-controller-open-vector input)
  (make-marlin-fleet-workbench-controller
   (vector-ref input 0) disclosure-level: (vector-ref input 1)))

(def (marlin-fleet-workbench-controller-dispatch-vector input)
  (let* ((next-ctrl (marlin-fleet-workbench-controller-apply-action
                     (vector-ref input 0) (vector-ref input 1)
                     task-id: (vector-ref input 2) arg: (vector-ref input 3)
                     receipts: (map marlin-graph-engine-vector->atomic-receipt
                                    (vector->list (vector-ref input 4)))
                     auto-archive?: (vector-ref input 5)))
         (drivers (.ref (.get next-ctrl fleet) 'active-drivers)))
    (vector next-ctrl
            (.get next-ctrl disclosure-level)
            (list->vector (map marlin-graph-engine-atomic-effect->vector
                               (.get next-ctrl last-effects)))
            (list->vector (map tab-id->str (.ref (.get next-ctrl projection) 'visible-tab-ids)))
            (list->vector (map (lambda (st) (opt-ref (.ref st 'actor) 'detour-mode 'mainline)) drivers))
            (list->vector (map (lambda (st)
                                 (foldl (lambda (o acc) (+ acc (length (.ref o 'active-defeaters))))
                                        0 (.ref (.ref st 'obligation-graph) 'obligations)))
                               drivers))
            (marlin-fleet-workbench-controller-project next-ctrl))))

;;; Input:  `controller`
;;; Output: `#(active-count archived-count peek-count total-defeaters total-inbox favored-collapsed? authenticates-evidence? authorizes-effect?)`
(def (marlin-fleet-workbench-controller-audit-capsule-vector controller)
  (let ((cap (marlin-fleet-workbench-controller-audit-capsule controller)))
    (vector (.ref cap 'active-actor-count)
            (.ref cap 'archived-actor-count)
            (.ref cap 'peek-count)
            (.ref cap 'total-active-defeaters)
            (.ref cap 'total-human-inbox-items)
            (.ref cap 'favored-path-collapsed?)
            (.ref cap 'authenticates-evidence?)
            (.ref cap 'authorizes-effect?))))

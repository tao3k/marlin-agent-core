;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted Scheme-object ABI for the selected UI workbench.
(import :gerbil-scheme-rust/scheme/runtime
        (only-in :clan/poo/object .def .get .o seal-object!)
        (only-in :yuanxu/poo-clos/types clos-instance?)
        "../ui/projection.ss")
(export marlin-ui-session-open marlin-ui-session-project marlin-ui-session-dispatch)

;;; Each event returns a successor; older frames keep their own rooted state.
(.def UiNativeSession.)
(def (session ui-value generic-value workbench-value focus-value)
  (seal-object! (.o (:: @ UiNativeSession.) ui: ui-value generic: generic-value
                    workbench: workbench-value focus: focus-value)))
(def (checked-session value)
  (unless (clos-instance? UiNativeSession. value) (error "invalid UI native session"))
  value)

(def (marlin-ui-session-open input)
  (unless (and (vector? input) (= (vector-length input) 3))
    (error "invalid UI native scope, facts and method bundles"))
  ;; The input is Yuanxu's complete selected publication. This module does
  ;; not import or initialize a parallel copy of INIT.
  (let* ((selection (vector-ref input 0))
         (ui (.get selection Marlin ui))
         (workbench ((.get ui workbench) (marlin-ui-native-facts (vector-ref input 1))))
         (generic ((.get ui make-dispatch) (vector-ref input 2))))
    (session ui generic workbench 'composer)))

(def (marlin-ui-session-project input)
  (let* ((current (checked-session input))
         (ui (.get current ui))
         (projection ((.get ui project-workbench) (.get current workbench))))
    (unless (and (vector? projection) (= (vector-length projection) 3))
      (error "invalid UI workbench projection"))
    (vector (vector-ref projection 0) (vector-ref projection 1)
            (vector-ref projection 2) (.get current focus))))

(def (event-for ui workbench kind)
  (case kind
    ((next-focus) ((.get ui focus-event) workbench))
    ((submit) (let (prototype (.get ui events submit)) (.o (:: @ prototype))))
    ((dismiss) (let (prototype (.get ui events dismiss)) (.o (:: @ prototype))))
    ((scroll-up scroll-down)
     (let (prototype (.get ui events scroll))
       (.o (:: @ prototype) direction: (if (eq? kind 'scroll-up) 'up 'down))))
    (else (error "unknown UI native event"))))

(def (marlin-ui-session-dispatch input)
  (unless (and (vector? input) (= (vector-length input) 2))
    (error "invalid UI native event request"))
  (let* ((current (checked-session (vector-ref input 0)))
         (ui (.get current ui))
         (generic (.get current generic))
         (workbench (.get current workbench))
         (focus (.get current focus))
         (event (event-for ui workbench (vector-ref input 1)))
         (intent (marlin-ui-apply-event ui generic workbench focus event))
         (next-focus (if (eq? (vector-ref intent 0) 'focus) (vector-ref intent 1) focus)))
    (vector (session ui generic workbench next-focus) intent)))

(begin-foreign
  (c-define (ui-open root) (int64) int64
    "marlin_ui_open" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/c-abi/ui#marlin-ui-session-open)))
(begin-foreign
  (c-define (ui-project-workbench root) (int64) int64
    "marlin_ui_project_workbench" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/c-abi/ui#marlin-ui-session-project)))
(begin-foreign
  (c-define (ui-dispatch-event root) (int64) int64
    "marlin_ui_dispatch_event" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/c-abi/ui#marlin-ui-session-dispatch)))

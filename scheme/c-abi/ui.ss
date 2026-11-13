;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted Scheme-object ABI for the selected UI workbench.
(import :gerbil-scheme-rust/scheme/runtime
        (only-in :clan/poo/object .def .get .o seal-object!)
        (only-in :std/list/list find filter)
        (only-in :yuanxu/poo-clos/types clos-instance?)
        "../ui/projection.ss")
(export marlin-ui-session-open marlin-ui-session-project marlin-ui-session-dispatch
        marlin-ui-session-browse-command
        marlin-ui-session-command-candidates
        marlin-ui-session-review-menu
        marlin-ui-session-refresh-labels)

;;; Each event returns a successor; older frames keep their own rooted state.
(.def UiNativeSession.)
(def (session ui-value generic-value workbench-value focus-value frame-value history-value)
  (seal-object! (.o (:: @ UiNativeSession.) ui: ui-value generic: generic-value
                    workbench: workbench-value focus: focus-value frame: frame-value
                    command-history: history-value)))
(def (checked-session value)
  (unless (clos-instance? UiNativeSession. value) (error "invalid UI native session"))
  value)

(def (marlin-ui-session-browse-command input)
  (unless (and (vector? input) (= (vector-length input) 6)
               (integer? (vector-ref input 4))
               (<= 1 (vector-ref input 4) 16)
               (or (not (vector-ref input 5))
                   (and (vector? (vector-ref input 5))
                        (<= (vector-length (vector-ref input 5)) 256))))
    (error "invalid UI command browse request"))
  (let* ((current (checked-session (vector-ref input 0)))
         (ui (.get current ui))
         (component (find (lambda (region) (eq? (.get region identity) 'commands))
                          (.get (.get current workbench) regions)))
         (prototype (.get ui events browse-command))
         (event (.o (:: @ prototype) query: (vector-ref input 1)
                    previous: (vector-ref input 2) direction: (vector-ref input 3)
                    steps: (vector-ref input 4) history: (.get current command-history)
                    allowed: (and (vector-ref input 5) (vector->list (vector-ref input 5))))))
    (unless component (error "UI command region is missing"))
    (let (intent ((.get ui dispatch) (.get current generic) component event))
      (unless (eq? (.get intent kind) 'command-selection) (error "invalid command browse intent"))
      (.get intent name))))

(def (marlin-ui-session-command-candidates input)
  (unless (and (vector? input) (= (vector-length input) 3)
               (or (not (vector-ref input 2))
                   (and (string? (vector-ref input 2)) (<= (string-length (vector-ref input 2)) 32))))
    (error "invalid UI command candidate request"))
  (let* ((current (checked-session (vector-ref input 0)))
         (ui (.get current ui))
         (component (find (lambda (region) (eq? (.get region identity) 'commands))
                          (.get (.get current workbench) regions))))
    (unless component (error "UI command region is missing"))
    (let* ((names ((.get ui command-candidates) (.get component entries) (vector-ref input 1)
                   (.get current command-history)))
           (locked (vector-ref input 2)))
      (vector (list->vector names) (and locked (member locked names) locked)))))

(def (marlin-ui-session-review-menu input)
  (unless (and (vector? input) (= (vector-length input) 2)
               (symbol? (vector-ref input 1)))
    (error "invalid UI review menu request"))
  (let* ((current (checked-session (vector-ref input 0)))
         (ui (.get current ui)))
    ((.get ui review-menu) (vector-ref input 1))))

(def (marlin-ui-session-open input)
  (unless (and (vector? input) (= (vector-length input) 3))
    (error "invalid UI native scope, facts and method bundles"))
  ;; The input is Yuanxu's complete selected publication. This module does
  ;; not import or initialize a parallel copy of INIT.
  (let* ((selection (vector-ref input 0))
         (ui (.get selection Marlin ui))
         (workbench ((.get ui workbench) (marlin-ui-native-facts (vector-ref input 1))))
         (generic ((.get ui make-dispatch) (vector-ref input 2))))
    (session ui generic workbench 'composer ((.get ui make-frame)) '())))

(def (marlin-ui-session-project input)
  (let* ((current (checked-session input))
         (ui (.get current ui))
         (projection ((.get ui project-workbench) (.get current workbench))))
    (unless (and (vector? projection) (= (vector-length projection) 5))
      (error "invalid UI workbench projection"))
    (vector (vector-ref projection 0) (vector-ref projection 1)
            (vector-ref projection 2) (.get current focus)
            (vector-ref projection 3) (vector-ref projection 4)
            ((.get ui project-frame) (.get current frame))
            ((.get ui keymap-entries))
            ((.get ui transient-entries)))))

;;; Refresh only owner-supplied display facts, retaining components and generic.
;;; Construct a fresh workbench directly so repeated updates do not build an
;;; unbounded prototype chain of old UI generations.
(def (marlin-ui-session-refresh-labels input)
  (unless (and (vector? input) (= (vector-length input) 3))
    (error "invalid UI label refresh"))
  (let* ((current (checked-session (vector-ref input 0)))
         (model-value (vector-ref input 1))
         (session-label (vector-ref input 2))
         (ui (.get current ui))
         (old (.get current workbench)))
    (unless (and (string? model-value) (<= (string-length model-value) 256)
                 (string? session-label) (<= (string-length session-label) 256))
      (error "invalid UI label facts"))
    (let (next (.o (:: @ (.get ui components workbench))
                   identity: (.get old identity) kind: (.get old kind)
                   model: (string-copy model-value) session: (string-copy session-label)
                   geometry-policy: (.get old geometry-policy)
                   transcript-policy: (.get old transcript-policy)
                   theme-palette: (.get old theme-palette)
                   regions: (.get old regions) focus-order: (.get old focus-order)))
      (session ui (.get current generic) next (.get current focus) (.get current frame)
               (.get current command-history)))))

(def (event-for ui workbench kind)
  (case kind
    ((next-focus) ((.get ui focus-event) workbench))
    ((previous-focus) ((.get ui previous-focus-event) workbench))
    ((submit) (let (prototype (.get ui events submit)) (.o (:: @ prototype))))
    ((dismiss) (let (prototype (.get ui events dismiss)) (.o (:: @ prototype))))
    ((scroll-up scroll-down)
     (let (prototype (.get ui events scroll))
       (.o (:: @ prototype) direction: (if (eq? kind 'scroll-up) 'up 'down))))
    (else (error "unknown UI native event"))))

(def (profile-facts workbench frame)
  (let* ((regions (.get workbench regions))
         (buffer-ids (map (lambda (buffer) (.get buffer identity))
                          (.get frame buffers)))
         (status (find (lambda (region) (eq? (.get region kind) 'status)) regions)))
    (unless status (error "UI status region is missing"))
    ;; Reuse only the sealed workbench snapshot, never the mutable open request.
    (.o model: (.get workbench model) session: (.get workbench session)
        contributions: (filter (lambda (item)
                                 (not (string-prefix? "modeline/"
                                       (symbol->string (.get item identity)))))
                               (.get status children))
        extensions: (filter (lambda (region)
                              (and (eq? (.get region kind) 'extension)
                                   ;; Selected Buffer components are rebuilt by
                                   ;; the owner and must not become contributions.
                                   (not (memq (.get region identity) buffer-ids))))
                            regions))))

(def (marlin-ui-session-dispatch input)
  (unless (and (vector? input) (<= 2 (vector-length input) 3))
    (error "invalid UI native event request"))
  (let* ((current (checked-session (vector-ref input 0)))
         (ui (.get current ui))
         (generic (.get current generic))
         (workbench (.get current workbench))
         (focus (.get current focus))
         (kind (vector-ref input 1)))
    (if (eq? kind 'record-command)
      (let* ((_arity (unless (and (= (vector-length input) 3) (symbol? (vector-ref input 2)))
                       (error "invalid UI command history request")))
             (name (symbol->string (vector-ref input 2)))
             (component (find (lambda (region) (eq? (.get region identity) 'commands))
                              (.get workbench regions)))
             (_known (unless (and component
                                  (find (lambda (entry) (string=? (car entry) name))
                                        (.get component entries)))
                       (error "unknown UI command history entry")))
             (next-history ((.get ui record-command-history) (.get current command-history) name)))
        (vector (session ui generic workbench focus (.get current frame) next-history)
                (vector 'focus focus)))
    (if (eq? kind 'select-buffer)
      (let* ((_arity (unless (= (vector-length input) 3)
                       (error "invalid UI buffer request")))
             (next-frame ((.get ui select-buffer) (.get current frame)
                          (vector-ref input 2))))
        (vector (session ui generic workbench focus next-frame (.get current command-history))
                (vector 'select-buffer (vector-ref input 2))))
    (if (eq? kind 'select-profile)
      (let* ((_arity (unless (and (= (vector-length input) 3)
                                  (symbol? (vector-ref input 2)))
                       (error "invalid UI profile request")))
             (profile (vector-ref input 2))
             (next ((.get ui profile-workbench)
                    (profile-facts workbench (.get current frame)) profile)))
        (vector (session ui generic next focus (.get current frame) (.get current command-history)) (vector 'focus focus)))
      (let* ((_arity (unless (= (vector-length input) 2)
                       (error "invalid UI event arity")))
             (event (event-for ui workbench kind))
             (intent (marlin-ui-apply-event ui generic workbench focus event))
             (next-focus (if (eq? (vector-ref intent 0) 'focus) (vector-ref intent 1) focus)))
        (vector (session ui generic workbench next-focus (.get current frame) (.get current command-history)) intent)))))))

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
(begin-foreign
  (c-define (ui-browse-command root) (int64) int64
    "marlin_ui_browse_command" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/c-abi/ui#marlin-ui-session-browse-command)))
(begin-foreign
  (c-define (ui-command-candidates root) (int64) int64
    "marlin_ui_command_candidates" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/c-abi/ui#marlin-ui-session-command-candidates)))
(begin-foreign
  (c-define (ui-review-menu root) (int64) int64
    "marlin_ui_review_menu" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/c-abi/ui#marlin-ui-session-review-menu)))
(begin-foreign
  (c-define (ui-refresh-labels root) (int64) int64
    "marlin_ui_refresh_labels" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/c-abi/ui#marlin-ui-session-refresh-labels)))

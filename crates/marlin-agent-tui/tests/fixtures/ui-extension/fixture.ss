;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; A contributed component and method bundle selected from the real UI scope.
(import (only-in :clan/poo/object .get .o)
        (only-in :tao3k/yuanxu/prelude yuanxu-autoload)
        (only-in :marlin-agent-core/ui/config UiScope)
        (only-in :marlin-agent-core/ui-centaur-tabs/funcs marlin-ui-buffer-tabs)
        (only-in :marlin-agent-core/ui-layout/config marlin-ui-layout-with-tab-bar)
        (only-in :marlin-agent-core/ui-meow/interface define-key-bindings meow-global meow-keypad
                 meow-things)
        (only-in :clan/poo/mop .defgeneric .defmethod .defmethod-bundle)
        (only-in :marlin-agent-core/ui-elements/funcs
                 ui-binding-present ui-rows ui-columns ui-region ui-sized ui-sized-with
                 ui-text ui-list
                 ui-size-cells ui-size-choice ui-template)
        (only-in :marlin-agent-core/ui/session UiPackage. make-ui-package
                 marlin-ui-session-open marlin-ui-session-project marlin-ui-session-dispatch))
(export test-extension make-test-methods make-test-forged-methods test-publication
        test-window make-test-window extension-test-window)

;;; This package contributes only Scheme values. The host has no case for
;;; notes, open-notes or M-z.
(define-key-bindings NotesKeymap
  keypad: (meow-keypad ((SPC z) (M-z)))
  (meow-global ((M-z) open-notes "Package notes"))
  (meow-things (z #\| #\| "pipes")))

(def (make-package-methods)
  (def event-protocol (.get UiScope event-protocol))
  (def select-row (.get UiScope events select-list-row))
  (def select-package-row
    (yuanxu-autoload ':marlin-tui-ui-test/row-action 'select-package-row))
  (.defgeneric package-event (component event) (protocol event-protocol))
  (.defmethod-bundle PackageMethods event-protocol
    (.defmethod (package-event (list-id (eql 'package-list))
                               (event (prototype select-row)))
      (select-package-row (.get event row))))
  PackageMethods)

(def test-publication
  (make-ui-package
    (.o (:: @ UiPackage.)
        buffers: `((notes "Package notes"
                     ("*Notes*"
                       ,(ui-region 'notes-body #f
                          (ui-rows
                            (list (ui-text '((info . "Scheme package window")))
                                  (ui-columns
                                    (list (ui-sized 'cells 5 (ui-text '((accent . "中🐟"))))
                                          (ui-text '((normal . "Layout from Scheme")))))
                                  (ui-sized 'cells 2
                                    (ui-list 'package-list
                                      '#(#(first #(#(entry "First Scheme row")))
                                         #(second #(#(entry "Second Scheme row"))))
                                      (ui-size-cells 0)
                                      (ui-template 'none '((binding normal #f entry)))))
                                  (ui-sized-with
                                    (ui-size-choice
                                      (list (list (ui-binding-present 'package-state) (ui-size-cells 1)))
                                      (ui-size-cells 0))
                                    (ui-region 'package-status #f
                                      (ui-template 'none '((binding accent #f package-state)))))))))
                     #f))
        commands: '(("open-notes" "Open package notes" none select-buffer "notes"))
        keymap: (.get NotesKeymap bindings)
        layout: (marlin-ui-layout-with-tab-bar
                  (lambda (buffers style)
                    (marlin-ui-buffer-tabs buffers '((notes "Lab Notes" packages)) style)))
        method-factories: (vector make-package-methods))))

(let (receipt (getenv "MARLIN_TEST_UI_PUBLICATION_RECEIPT" #f))
  (when receipt
    (call-with-output-file (list path: receipt append: #t)
      (lambda (port) (display "UI-PUBLICATION-LOADED\n" port)))))

;;; A bounded frame dialogue: assertions stay in this independently compiled
;;; package; the native test window returns the actual Ratatui screen rows.
(def (make-test-window _seed)
  (let ((step 0)
        (state (marlin-ui-session-open
                 (vector test-publication #("model" "session" #() #()) #()))))
    (lambda (screen)
      (case step
        ((0)
         (let* ((base-session (marlin-ui-session-open
                                (vector UiScope #("model" "session" #() #()) #())))
                (base (marlin-ui-session-project base-session))
                (resumed (vector-ref (marlin-ui-session-dispatch
                                       (vector base-session 'select-buffer 'messages)) 0))
                (activated (marlin-ui-session-project
                             (marlin-ui-session-open (vector test-publication resumed))))
                (frame (vector-ref activated 6))
                (window (vector-ref (vector-ref frame 1) 0))
                (view (marlin-ui-session-project state)))
           (unless (eq? (vector-ref window 1) 'messages)
             (error "package activation changed the selected Buffer"))
           (unless (and (memq 'notes (vector->list (vector-ref (vector-ref view 6) 4)))
                        (not (memq 'notes (vector->list (vector-ref (vector-ref base 6) 4))))
                        (member '#(global #(M-z) open-notes)
                                (vector->list (vector-ref view 7))))
             (error "package catalog or publication isolation failed"))
           (unless (equal? (vector-ref (marlin-ui-session-dispatch
                                        (vector base-session 'select-list-row '#(package-list first))) 1)
                           '#(focus composer))
             (error "package method escaped its session")))
         (set! state (vector-ref (marlin-ui-session-dispatch
                                  (vector state 'select-buffer 'notes)) 0))
         (set! state (vector-ref (marlin-ui-session-dispatch
                                  (vector state 'select-profile 'tabs-box)) 0))
         (set! step 1)
         (vector 60 20 (marlin-ui-session-project state) #(#(2 2) #(2 4))
                 '#(#(package-state "Provided by Scheme"))))
        ((1)
         (unless (and (ormap (lambda (row) (string-contains row "Scheme package window"))
                             (vector->list (vector-ref screen 0)))
                      (string-contains (vector-ref (vector-ref screen 0) 0) "Lab Notes│")
                      (ormap (lambda (row) (string-contains row "Provided by Scheme"))
                             (vector->list (vector-ref screen 0)))
                      (ormap (lambda (hit) (and (eq? (vector-ref hit 0) 'package-status)
                                                (= (vector-ref hit 4) 1)))
                             (vector->list (vector-ref screen 1)))
                      (ormap (lambda (hit)
                               (and (eq? (vector-ref hit 0) 'buffer-notes)
                                    (= (vector-ref hit 2) 0)))
                             (vector->list (vector-ref screen 1)))
                      (ormap (lambda (hit) (eq? (vector-ref hit 0) 'notes-body))
                             (vector->list (vector-ref screen 1)))
                      (eq? (vector-ref (vector-ref (vector-ref screen 3) 0) 0) 'notes-body)
                      (let* ((hit (vector-ref (vector-ref screen 3) 1))
                             (selection (vector (vector-ref hit 2) (vector-ref hit 3))))
                        (and (equal? selection '#(package-list first))
                             (equal? (vector-ref (marlin-ui-session-dispatch
                                                  (vector state 'select-list-row selection)) 1)
                                     '#(focus commands)))))
           (error "package buffer was not painted with its hit region"))
         (set! step 2)
         (vector 32 10 (marlin-ui-session-project state) #()))
        ((2)
         (unless (and (= (vector-length (vector-ref screen 0)) 10)
                      (string-contains (vector-ref (vector-ref screen 0) 0) "Lab Notes")
                      (ormap (lambda (row) (string-contains row "Scheme package window"))
                             (vector->list (vector-ref screen 0)))
                      (ormap (lambda (row)
                               (and (string-contains row "中") (string-contains row "🐟")))
                             (vector->list (vector-ref screen 0)))
                      (not (ormap (lambda (row) (string-contains row "Provided by Scheme"))
                                  (vector->list (vector-ref screen 0))))
                      (not (ormap (lambda (hit) (eq? (vector-ref hit 0) 'package-status))
                                  (vector->list (vector-ref screen 1))))
                      (<= 0 (vector-ref (vector-ref screen 2) 0) 31)
                      (<= 0 (vector-ref (vector-ref screen 2) 1) 9))
           (error "package window resize failed"))
         (unless (andmap (lambda (region)
                           (and (= (vector-length region) 7)
                                (<= 0 (vector-ref region 1)
                                    (+ (vector-ref region 1) (vector-ref region 3)) 32)
                                (<= 0 (vector-ref region 2)
                                    (+ (vector-ref region 2) (vector-ref region 4)) 10)))
                         (vector->list (vector-ref screen 1)))
           (error "package window hit geometry escaped frame"))
         #f)
        (else (error "unexpected test window step"))))))

(def ui UiScope)
(def test-extension
  (.o (:: @ (.get ui components extension)) identity: 'test/extension
      kind: 'extension label: "Test extension" focusable: #t
      content: (list
                 (.o (:: @ (.get ui components text))
                     kind: 'text text: "Scheme component body")
                 (.o (:: @ (.get ui components field))
                     kind: 'field label: "Source" value: "selected module"))))
(def submit-event (.get ui events submit))
(.defgeneric test-event (component event) (protocol (.get ui event-protocol)))
(def (make-test-methods)
  (.defmethod-bundle Methods (.get ui event-protocol)
    (.defmethod (test-event (component (prototype test-extension))
                            (event (prototype submit-event)))
      (.o kind: 'focus identity: 'composer)))
  Methods)
(def (make-test-forged-methods)
  (.defmethod-bundle Methods (.get ui event-protocol)
    (.defmethod (test-event (component (prototype test-extension))
                            (event (prototype submit-event)))
      (.o kind: 'submit-draft)))
  Methods)

;;; Presentation assertions belong to this package, not a parallel native list
;;; of expected labels. The driver still supplies the production mapper.
(def extension-test-window
  (let ((painted? #f)
        (state
          (foldl (lambda (event state)
                  (vector-ref (marlin-ui-session-dispatch (vector state event)) 0))
                (marlin-ui-session-open
                  (vector UiScope
                          (vector "model" "session" #(#(test/status "VM ready"))
                                  (vector test-extension))
                          (vector make-test-methods)))
                '(next-focus next-focus))))
    (lambda (screen)
      (if painted?
        (let (text (apply string-append (vector->list (vector-ref screen 0))))
          (unless (andmap (lambda (expected) (string-contains text expected))
                         '("VM ready" "Test extension" "Scheme component body"
                           "Source: selected module"))
            (error "extension window presentation failed" (vector-ref screen 0)))
          #f)
        (begin
          (set! painted? #t)
          (vector 60 20 (marlin-ui-session-project state) #()))))))

(def test-window (make-test-window (void)))

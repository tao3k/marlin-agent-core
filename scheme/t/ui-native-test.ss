;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Exercise dynamic module imports and the selected native UI ABI.
(import :std/test :gerbil/expander
        (only-in :clan/poo/object .get)
        (only-in :clan/poo/type Enum List)
        (only-in :clan/poo/support/testing check-property)
        (only-in :clan/poo/testing/property make-policy)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :marlin-agent-core/ui/session marlin-ui-session-open
                 marlin-ui-session-dispatch marlin-ui-session-refresh-labels))
(export ui-native-test)

(def (observe-presentation state)
  (let ((workbench (.get state workbench))
        (main-window (car (.get state frame windows))))
    (vector (.get workbench theme-profile) (.get workbench tab-style)
            (.get main-window buffer identity)
            (.get workbench model) (.get workbench session) (.get state focus))))

(def (presentation-trace-preserves-state? actions)
  (def ui (.get marlin-agent-core.init.exports Marlin ui))
  (def initial (marlin-ui-session-open (vector ui #("model" "session" #() #()) #())))
  (def (dispatch state event choice)
    (vector-ref (marlin-ui-session-dispatch (vector state event choice)) 0))
  (let loop ((state initial)
             (expected (vector 'doom-one 'tabs-bar 'conversation "model" "session" 'composer))
             (remaining actions))
    (and (equal? (observe-presentation state) expected)
         (or (null? remaining)
             (let* ((action (car remaining))
                    (successor (vector-copy expected))
                    (next
                      (case action
                        ((doom-one pi)
                         (vector-set! successor 0 action)
                         (dispatch state 'select-profile action))
                        ((tabs-bar tabs-box tabs-slant)
                         (vector-set! successor 1 action)
                         (dispatch state 'select-profile action))
                        ((conversation messages help)
                         (vector-set! successor 2 action)
                         (dispatch state 'select-buffer action))
                        ((refresh)
                         (vector-set! successor 3 "updated-model")
                         (vector-set! successor 4 "updated-session")
                         (marlin-ui-session-refresh-labels
                           (vector state "updated-model" "updated-session"))))))
               ;; A successor must preserve the previous rooted session.
               (and (equal? (observe-presentation state) expected)
                    (loop next successor (cdr remaining))))))))

(def ui-native-test
  (test-suite "Marlin UI workbench"
    (test-case "generated presentation actions preserve independent state"
      (check-property
        (List (Enum doom-one pi tabs-bar tabs-box tabs-slant conversation messages help refresh))
        presentation-trace-preserves-state?
        name: 'independent-ui-presentation seed: 42 cases: 48 size: 8 shrink-budget: 32
        policy: (make-policy
                  labels: (lambda (trace)
                            (filter (lambda (action) (memq action trace))
                                    '(doom-one pi tabs-bar tabs-box tabs-slant
                                      conversation messages help refresh)))
                  required: '((doom-one . 1) (pi . 1) (tabs-bar . 1) (tabs-box . 1)
                              (tabs-slant . 1) (conversation . 1) (messages . 1)
                              (help . 1) (refresh . 1)))))
    (test-case "standard module import retains the selected UI scope"
      (let ((expected (.get marlin-agent-core.init.exports Marlin ui)))
        (parameterize ((current-expander-context (make-top-context)))
          (eval-syntax '(import :gerbil/core))
          (eval-syntax '(import (only-in :marlin-agent-core/init marlin-agent-core.init.exports)))
          (let (selected (.get (eval 'marlin-agent-core.init.exports) Marlin ui))
            (check (eq? selected expected) => #t))
          (eval-syntax '(import (only-in :marlin-agent-core/ui/session
                                        marlin-ui-session-open marlin-ui-session-project
                                        marlin-ui-session-dispatch
                                        marlin-ui-session-browse-command
                                        marlin-ui-session-command-candidates
                                        marlin-ui-session-review-menu
                                        marlin-ui-session-refresh-labels)))
          (let* ((publication expected)
                 (open (eval 'marlin-ui-session-open))
                 (project (eval 'marlin-ui-session-project))
                 (dispatch (eval 'marlin-ui-session-dispatch))
                 (browse (eval 'marlin-ui-session-browse-command))
                 (candidates (eval 'marlin-ui-session-command-candidates))
                 (review-menu (eval 'marlin-ui-session-review-menu))
                 (refresh-labels (eval 'marlin-ui-session-refresh-labels))
                 (state (open (vector publication #("model" "session" #() #()) #())))
                 (names (vector->list (vector-ref (candidates (vector state "" #f)) 0)))
                 (ranked (vector-ref (dispatch (vector state 'record-command 'runtime)) 0))
                 (view (project state))
                 (selection (dispatch (vector state 'select-profile 'pi)))
                 (pi-view (project (vector-ref selection 0)))
                 (buffer-selection (dispatch (vector state 'select-buffer 'messages)))
                 (message-view (project (vector-ref buffer-selection 0)))
                 (help-selection (dispatch (vector state 'select-buffer 'help)))
                 (help-state (vector-ref help-selection 0))
                 (help-view (project help-state))
                 (help-profile (dispatch (vector help-state 'select-profile 'pi)))
                 (help-pi-view (project (vector-ref help-profile 0))))
            (check-equal? (vector-ref view 0) "model")
            (check-equal? (browse (vector state "" #f 'next 1 #f)) (car names))
            (check-equal? (vector-ref (vector-ref (candidates (vector ranked "" #f)) 0) 0) "runtime")
            (check-equal?
              (review-menu (vector state 'permission-escalation))
              '#("Permission: "
                 #(#(approved "Approve once" "Continue with the approved graph")
                   #(rejected "Deny" "Stop this graph loop")
                   #(needs-question "Ask a question" "Keep this gate pending")
                   #(patch-requested "Request changes" "Keep this gate pending"))))
            (check-equal?
              (review-menu (vector state 'model-tool))
              '#("Tool request: "
                 #(#(approve-once "Approve once" "Run the requested tool once")
                   #(reject "Deny" "Do not run the requested tool"))
                 "Model tool request · inspect every argument before approval"))
            (check-equal? (vector-ref view 3) 'composer)
            (let* ((transcript-request (dispatch (vector state 'select-focus 'transcript)))
                   (transcript-state (vector-ref transcript-request 0))
                   (editor-request (dispatch (vector transcript-state 'select-focus 'composer))))
              (check-equal? (vector-ref transcript-request 1) '#(focus transcript))
              (check-equal? (vector-ref editor-request 1) '#(focus composer))
              (check-equal? (vector-ref (project (vector-ref editor-request 0)) 3) 'composer))
            (check-exception (dispatch (vector state 'select-focus 'unknown)) error?)
            (check-equal? (vector-ref selection 1) '#(focus composer))
            (check-equal? (vector-ref message-view 1) (vector-ref view 1))
            (check-equal? (vector-ref help-pi-view 1) (vector-ref view 1))
            (check-equal? (vector-ref (vector-ref help-pi-view 4) 4)
                          (vector-ref (vector-ref pi-view 4) 4))
            (check-equal? (vector-ref buffer-selection 1) '#(select-buffer messages))
            (check-equal? (vector-ref (vector-ref message-view 6) 1) '#(#(main messages)))
            (check-equal? (vector-ref (vector-ref help-view 6) 1) '#(#(main help)))
            (check-equal? (vector-ref (vector-ref help-pi-view 6) 1) '#(#(main help)))
            (check-equal? (vector-ref (vector-ref (vector-ref help-view 2) 3) 0) 'help)
            (check (and (member '#(normal #(M-x) open-commands)
                                (vector->list (vector-ref view 7)))
                        #t)
                   => #t)
            (check-equal? (vector-ref (vector-ref view 6) 1) '#(#(main conversation)))
            (let* ((boxed (vector-ref (dispatch (vector help-state 'select-profile 'tabs-box)) 0))
                   (boxed-view (project boxed))
                   (pi-boxed (vector-ref (dispatch (vector boxed 'select-profile 'pi)) 0))
                   (refreshed (refresh-labels (vector pi-boxed "new-model" "new-session")))
                   (slanted (vector-ref (dispatch (vector refreshed 'select-profile 'tabs-slant)) 0)))
              (check-equal? (.get boxed workbench theme-profile) 'doom-one)
              (check-equal? (.get boxed workbench tab-style) 'tabs-box)
              (check-equal? (vector-ref boxed-view 5) (vector-ref help-view 5))
              (check (equal? (vector-ref boxed-view 4) (vector-ref help-view 4)) => #f)
              (check-equal? (.get pi-boxed workbench theme-profile) 'pi)
              (check-equal? (.get pi-boxed workbench tab-style) 'tabs-box)
              (check-equal? (.get refreshed workbench tab-style) 'tabs-box)
              (check-equal? (.get slanted workbench theme-profile) 'pi)
              (check-equal? (.get slanted workbench tab-style) 'tabs-slant)
              (check-equal? (vector-ref (vector-ref (project slanted) 6) 1)
                            '#(#(main help))))))))))

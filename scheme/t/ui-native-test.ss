;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Run the workbench contract through the SDK's in-process GxTest entrypoint.
(import :std/test :gerbil/expander
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in "ui-workbench-test" ui-workbench-verified))
(export ui-native-test)

(def ui-native-test
  (test-suite "Marlin UI workbench"
    (test-case "the selected Scheme UI contract completed"
      (check ui-workbench-verified => #t))
    (test-case "standard module import retains the selected UI scope"
      (let ((expected (.get marlin-agent-core.init.exports Marlin ui)))
        (parameterize ((current-expander-context (make-top-context)))
          (eval-syntax '(import :gerbil/core))
          (eval-syntax '(import (only-in :marlin-agent-core/init marlin-agent-core.init.exports)))
          (let (selected (.get (eval 'marlin-agent-core.init.exports) Marlin ui))
            (check (eq? selected expected) => #t))
          (eval-syntax '(import (only-in :marlin-agent-core/c-abi/ui
                                        marlin-ui-session-open marlin-ui-session-project
                                        marlin-ui-session-dispatch
                                        marlin-ui-session-browse-command
                                        marlin-ui-session-command-candidates
                                        marlin-ui-session-review-menu)))
          (let* ((publication (eval 'marlin-agent-core.init.exports))
                 (open (eval 'marlin-ui-session-open))
                 (project (eval 'marlin-ui-session-project))
                 (dispatch (eval 'marlin-ui-session-dispatch))
                 (browse (eval 'marlin-ui-session-browse-command))
                 (candidates (eval 'marlin-ui-session-command-candidates))
                 (review-menu (eval 'marlin-ui-session-review-menu))
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
            (check-equal? (browse (vector state "" (car names) 'next 2 #f)) (caddr names))
            (check-equal? (browse (vector state "" (car names) 'previous 1 #f)) (car (reverse names)))
            (check-equal? (vector-ref (vector-ref (candidates (vector ranked "" #f)) 0) 0) "runtime")
            (check-equal? (browse (vector ranked "" #f 'next 1 #f)) "runtime")
            (check-equal? (browse (vector ranked "" #f 'next 1 #("clear"))) "clear")
            (check-equal? (vector->list (vector-ref (candidates (vector state "" #f)) 0)) names)
            (check-equal? (vector-ref (candidates (vector state "mode" "models")) 1) "models")
            (check-equal? (vector-ref (candidates (vector state "mode" "runtime")) 1) #f)
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
                   #(reject "Deny" "Do not run the requested tool"))))
            (check-equal? (vector-ref view 3) 'composer)
            (check-equal? (vector-ref selection 1) '#(focus composer))
            (check-equal? (vector-ref (vector-ref pi-view 5) 3) '#(255 186 104))
            (check-equal? (vector-ref (vector-ref view 4) 4)
                          '#(#t "› " "" "▸ Thought" "▾ Thought"))
            (check-equal? (vector-ref (vector-ref pi-view 4) 4)
                          '#(#t "› " "" "⋯" "⋯ Thought"))
            (check-equal? (vector-ref message-view 1) (vector-ref view 1))
            (check-equal? (vector-ref help-pi-view 1) (vector-ref view 1))
            (check-equal? (vector-ref (vector-ref help-pi-view 4) 4)
                          (vector-ref (vector-ref pi-view 4) 4))
            (check-equal? (vector-ref buffer-selection 1) '#(select-buffer messages))
            (check-equal? (vector-ref (vector-ref message-view 6) 1) '#(#(main messages)))
            (check-equal? (vector-ref (vector-ref help-view 6) 1) '#(#(main help)))
            (check-equal? (vector-ref (vector-ref help-pi-view 6) 1) '#(#(main help)))
            (check-equal? (vector-ref (vector-ref (vector-ref help-view 2) 3) 0) 'help)
            (check-equal? (vector-ref (vector-ref view 7) 0) '#(#(M-x) open-commands))
            (check-equal? (vector-ref (vector-ref view 6) 1) '#(#(main conversation)))))))))

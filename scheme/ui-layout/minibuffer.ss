;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; The echo area shares the minibuffer's space with the active input.
(import (only-in "../ui-elements/funcs.ss"
                 ui-binding-present ui-choice ui-columns ui-input ui-prompt
                 ui-rows ui-size-add ui-size-binding ui-size-cells ui-size-choice ui-size-flex
                 ui-size-min ui-sized-with ui-slot ui-template ui-text))
(export minibuffer-echo-policy minibuffer-status minibuffer-editor
        echo-area-owns-minibuffer dashboard-overhead)

(def (line spans) (ui-template 'none spans))
(def (text value) (line (list (list 'literal 'normal #f value))))
(def echo-display (line '((binding muted #f echo))))
(def echo-area-owns-minibuffer 'any/transient/keypad/key-undefined)

(def minibuffer-echo-policy
  (vector
    (ui-choice
      (list (list 'quit-armed (text "Press Ctrl-C again to quit"))
            (list 'draft-limit (text "Draft limit reached"))
            (list 'keypad
                  (line '((literal muted #f " Keypad: ")
                          (binding muted #f key-echo) (literal muted #f "-"))))
            (list 'key-undefined
                  (line '((literal muted #f " ")
                          (binding muted #f key-echo) (literal muted #f " is undefined"))))
            (list 'key-prefix-pending
                  (line '((binding muted #f key-echo) (literal muted #f "-"))))
            (list (ui-binding-present 'notice) (line '((binding muted #f notice))))
            (list (ui-binding-present 'key-echo) (line '((binding muted #f key-echo)))))
      (line '()))
    3000 #t))

(def (search-template prompt)
  (line (list (list 'literal 'normal #f prompt) '(binding normal #f query))))
(def search-view
  (ui-columns
    (list (ui-sized-with (ui-size-flex 1)
            (ui-prompt 'query
              (ui-choice (list (list 'searching (search-template " Searching: "))
                               (list 'search-match (search-template " Search: ")))
                         (search-template " No match: "))))
          (ui-sized-with
            (ui-size-choice (list (list 'width-at-least-40 (ui-size-cells 18)))
                            (ui-size-cells 0))
            echo-display))))

(def roomy-dashboard 'height-at-least-15)
(def dashboard-overhead
  (ui-size-choice (list (list roomy-dashboard (ui-size-cells 3))) (ui-size-cells 1)))
(def dashboard-spacer
  (ui-sized-with
    (ui-size-min (ui-size-choice (list (list roomy-dashboard (ui-size-cells 1)))
                                (ui-size-cells 0))
                 (ui-size-binding 'hint-rows))
    (ui-text '())))
(def dashboard-view
  (ui-rows
    (list dashboard-spacer
          (ui-sized-with (ui-size-flex 1) (ui-slot 'transient-hint))
          dashboard-spacer
          (ui-sized-with (ui-size-cells 1) echo-display))))

(def idle-prefix
  (ui-choice
    (list (list 'extra-two
                (line '((literal normal #f " › ") (binding normal #f extra-one)
                        (literal normal #f " · ") (binding normal #f extra-two)
                        (literal normal #f " · "))))
          (list 'extra-one
                (line '((literal normal #f " › ") (binding normal #f extra-one)
                        (literal normal #f " · ")))))
    (ui-choice
      (list (list 'normal-mode (text " › "))
            (list 'focused
                  (ui-prompt 'idle-input
                    (line '((literal normal #f " › ") (binding normal #f idle-input))))))
      (text " › "))))
(def idle-prefix-width
  (ui-size-choice
    (list (list 'extra-two
                (ui-size-add (ui-size-cells 9)
                             (ui-size-add (ui-size-binding 'extra-one-width)
                                          (ui-size-binding 'extra-two-width))))
          (list 'extra-one
                (ui-size-add (ui-size-cells 6) (ui-size-binding 'extra-one-width))))
    (ui-size-cells 3)))
(def idle-status
  (ui-choice
    (list (list 'workspace (text "Workspace: PageUp/PageDown scroll  Esc/Ctrl-C return  Enter submit draft"))
          (list 'messages (text "*Messages* · Esc returns to conversation"))
          (list 'static-buffer (line '((binding normal #f static-name)
                                       (literal normal #f " · Esc returns"))))
          (list 'runtime (text "Runtime: Esc/Ctrl-C return  Enter submit draft"))
          (list 'commands-focus
                (ui-choice (list (list 'normal-mode (text "Commands: Enter open")))
                           (text "Commands: Esc enters Normal")))
          (list 'transcript-focus (text "Transcript: scroll to browse"))
          (list 'extension-focus (text "Extension: Enter activate"))
          (list 'normal-mode (line '())))
    (text "Message: Enter send  Ctrl-R search")))

(def minibuffer-status
  (ui-choice
    (list (list echo-area-owns-minibuffer dashboard-view)
          (list 'search-no-match search-view)
          (list (ui-binding-present 'echo) echo-display))
    (ui-columns (list (ui-sized-with idle-prefix-width idle-prefix)
                      (ui-sized-with (ui-size-flex 1) idle-status)))))

(def inline-feedback (vector 18 echo-display))
(def minibuffer-editor
  (ui-choice
    (list (list 'command
                (ui-input #f 'bar
                  (ui-choice
                    (list (list 'width-at-least-20
                                (line '((binding accent #f selected-count)
                                        (literal accent #f "/")
                                        (binding accent #f candidate-count)
                                        (literal accent #f " ")
                                        (binding accent #f prompt)))))
                    (line '((binding accent #f compact-prompt))))
                  #f 'focused inline-feedback)))
    (ui-input #f 'none #f #f 'focused inline-feedback)))

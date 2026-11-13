;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme owns presentation choices; the native host clips them to cells.
(import (only-in :clan/poo/object .def .mix .o seal-object!)
        (only-in "../ui-elements/objects.ss"
                 ui-text ui-rows ui-columns ui-region ui-scroll-target ui-template ui-choice ui-sized-with
                 ui-size-cells ui-size-flex ui-size-binding ui-size-bounded ui-size-choice
                 ui-size-min ui-size-max ui-size-add ui-size-subtract
                 ui-input ui-slot ui-panel ui-list)
        (only-in "../ui-components/commands.ss" marlin-ui-candidate-templates marlin-ui-list-offset))
(export marlin-ui-layout-policy-for)

(def MessageFrameValue
  (seal-object! (.o min-transcript-rows: 2 fixed-editor-rows: 0
                    footer-rows: 1 extensions: #t prompt: " Message ")))
(def CommandFrameValue
  (seal-object! (.o min-transcript-rows: 4 fixed-editor-rows: 1
                    footer-rows: 0 extensions: #f prompt: " M-x ")))
(def (status-line spans)
  (ui-template 'none spans))
(def (status-text text)
  (status-line (list (list 'literal 'normal #f text))))
(def (search-line prompt)
  (ui-template 'none (list (list 'literal 'normal #f prompt)
                           '(binding normal #f query))))
(def status-prefix
  (ui-choice
    (list (list 'editor-collapsed
                (ui-choice
                  (list (list 'extra-two
                              (status-line '((literal normal #f " › ") (binding normal #f extra-one)
                                             (literal normal #f " · ") (binding normal #f extra-two)
                                             (literal normal #f " · "))))
                        (list 'extra-one
                              (status-line '((literal normal #f " › ") (binding normal #f extra-one)
                                             (literal normal #f " · ")))))
                  (status-text " › "))))
    (status-text " ")))
(def status-prefix-width
  (ui-size-choice
    (list (list 'editor-collapsed
                (ui-size-choice
                  (list (list 'extra-two
                              (ui-size-add (ui-size-cells 9)
                                           (ui-size-add (ui-size-binding 'extra-one-width)
                                                        (ui-size-binding 'extra-two-width))))
                        (list 'extra-one
                              (ui-size-add (ui-size-cells 6) (ui-size-binding 'extra-one-width))))
                  (ui-size-cells 3))))
    (ui-size-cells 1)))
(def status-body
  (ui-choice
    (list (list 'notice (status-line '((binding normal #f notice))))
          (list 'messages (status-text "*Messages* · Esc returns to conversation"))
          (list 'static-buffer (status-line '((binding normal #f static-name)
                                             (literal normal #f " · Esc returns"))))
          (list 'quit-armed (status-text "Press Ctrl-C again to quit; another key cancels. Draft will not be saved."))
          (list 'draft-limit (status-text "Draft limit: 1 MiB. Insertion rejected; draft retained."))
          (list 'runtime (status-text "Runtime: Esc/Ctrl-C return  Enter submit draft"))
          (list 'workspace (status-text "Workspace: PageUp/PageDown scroll  Esc/Ctrl-C return  Enter submit draft"))
          (list 'commands-focus (status-text "Commands: Enter open  M-x search  Ctrl-Q quit"))
          (list 'transcript-focus (status-text "Transcript: scroll to browse  M-x commands  Ctrl-Q quit"))
          (list 'extension-focus (status-text "Extension: Enter activate  M-x commands  Ctrl-Q quit"))
          (list 'normal-mode (status-text "NORMAL")))
    (status-text "Message: Enter send  Ctrl-R search  M-x commands  Ctrl-Q quit")))
(def minibuffer-status
  (ui-choice
    (list (list 'searching (search-line " Searching: "))
          (list 'search-match (search-line " Search: "))
          (list 'search-no-match (search-line " No match: ")))
    (ui-columns (list (ui-sized-with status-prefix-width status-prefix)
                      (ui-sized-with (ui-size-flex 1) status-body)))))
(def editor-view
  (ui-choice
    (list (list 'command
                (ui-input #f 'bar
                  (ui-choice
                    (list (list 'width-at-least-20
                                (ui-template 'none '((binding accent #f selected-count)
                                                     (literal accent #f "/")
                                                     (binding accent #f candidate-count)
                                                     (literal accent #f " ")
                                                     (binding accent #f prompt)))))
                    (ui-template 'none '((binding accent #f compact-prompt))))
                  #f 'focused)))
    (ui-input #t 'none #f 'title 'focused)))
(def extension-view
  (ui-list 'extensions marlin-ui-list-offset (ui-choice
    (list (list 'selected
                (ui-template 'none '((literal info #t " › ")
                                     (binding info #t label)))))
    (ui-template 'none '((literal normal #f " · ")
                         (binding normal #f label))))))
;;; The first true fact selects one bounded host-owned content source.
(def named-panel-title
  (ui-template 'none '((literal info #f " ") (binding info #f panel-title)
                       (literal info #f " "))))
(def runtime-panel-title
  (ui-template 'none '((literal info #f " Runtime policy "))))
(def panel-field
  (ui-template 'none '((binding info #f label) (literal info #f ": ")
                       (binding normal #f value))))
(def main-view
  (ui-choice
    (list (list 'review (ui-slot 'review))
          (list 'messages (ui-slot 'messages))
          (list 'static (ui-panel 'info panel-field named-panel-title (ui-slot 'static)))
          (list 'runtime (ui-panel 'info panel-field runtime-panel-title (ui-slot 'runtime)))
          (list 'extension (ui-panel 'info panel-field named-panel-title (ui-slot 'extension)))
          (list 'workspace (ui-slot 'workspace)))
    (ui-slot 'conversation)))

;;; Size expressions contain only named facts and host-provided cell metrics.
(def (size-choice cases) (ui-size-choice cases (ui-size-cells 0)))
;;; Keep the transcript and minibuffer visible before allocating editor rows.
(def terminal-height (ui-size-binding 'terminal-height))
(def transcript-reservation
  (ui-size-min (ui-size-binding 'transcript-min)
               (ui-size-subtract terminal-height (ui-size-cells 2))))
(def footer-reservation
  (ui-size-max (ui-size-binding 'footer-min) (ui-size-binding 'hint-rows)))
(def editor-reservation
  (ui-size-add (ui-size-add footer-reservation (ui-size-cells 1))
               transcript-reservation))
(def editor-budget
  (ui-size-min (ui-size-binding 'chatbox-limit)
               (ui-size-subtract terminal-height editor-reservation)))
(def editor-size
  (size-choice (list (list 'editor-fixed (ui-size-bounded 'fixed-editor 'terminal-height))
                     (list 'editor-expanded
                           (ui-size-min (ui-size-binding 'measured-editor)
                                        editor-budget)))))
;;; Named region bindings let the layout express priorities without host-side
;;; knowledge of any particular component or its neighbours.
(def editor-rows (ui-size-binding 'region/editor))
(def footer-size
  (ui-size-max
    (ui-size-min footer-reservation
                 (ui-size-subtract terminal-height
                   (ui-size-add (ui-size-add transcript-reservation editor-rows)
                                (ui-size-cells 1))))
    (ui-size-min (ui-size-binding 'footer-min) terminal-height)))
(def spare-rows
  (ui-size-subtract terminal-height
    (ui-size-add (ui-size-add (ui-size-binding 'region/minibuffer)
                              transcript-reservation)
                 editor-rows)))
(def modeline-size
  (size-choice (list (list 'modeline-present (ui-size-min (ui-size-cells 1) spare-rows)))))
(def candidate-budget
  (ui-size-subtract spare-rows (ui-size-binding 'region/modeline)))
(def candidate-size
  (size-choice (list (list 'review (ui-size-min (ui-size-binding 'review-count) candidate-budget))
                     (list 'command
                           (ui-size-min (ui-size-binding 'command-count)
                                        (ui-size-min (ui-size-binding 'command-limit)
                                                     candidate-budget))))))
(def extension-size
  (size-choice
    (list (list 'extensions-enabled
                (ui-size-min (ui-size-binding 'extension-count)
                             (ui-size-subtract candidate-budget
                                               (ui-size-binding 'region/candidates)))))))
(def (frame-region modeline-value slot)
  (let (name (car slot))
    (ui-sized-with
      (case name ((transcript) (ui-size-flex 1)) ((candidates) candidate-size)
        ((extensions) extension-size) ((editor) editor-size) ((modeline) modeline-size)
        ((minibuffer) footer-size) (else (ui-size-binding name)))
      (ui-region name (and (cadr slot) (ui-scroll-target (cadr slot) (caddr slot)))
                 (case name
                   ((transcript) main-view)
                   ((extensions) extension-view)
                   ((editor) editor-view)
                   ((candidates) marlin-ui-candidate-templates)
                   ((modeline) modeline-value)
                   ((minibuffer) minibuffer-status)
                   (else (ui-text '())))))))
;;; The hover region encloses exactly the editor, candidates and status row.
(.def UiLayoutBase.
  message-frame: MessageFrameValue
  command-frame: CommandFrameValue
  max-chatbox-rows: 10
  max-command-rows: 10)
(def (marlin-ui-layout-policy-for modeline-value)
  (let (region (lambda (slot) (frame-region modeline-value slot)))
    (seal-object!
      (.mix (.o frame-tree: (ui-rows
              (list (ui-rows (map region '((transcript transcript 3) (extensions #f 0))))
                    (region '(modeline #f 0))
                    (ui-region 'minibuffer-hover #f
                      (ui-rows (map region '((editor #f 0) (candidates candidates 1)
                                             (minibuffer #f 0))))))))
            UiLayoutBase.))))

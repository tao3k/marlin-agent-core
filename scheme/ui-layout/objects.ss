;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme owns presentation choices; the native host clips them to cells.
(import (only-in :clan/poo/object .def .o seal-object!)
        (only-in "../ui-elements/objects.ss"
                 ui-text ui-rows ui-region ui-template ui-choice ui-sized-with
                 ui-size-cells ui-size-flex ui-size-binding ui-size-bounded ui-size-choice
                 ui-size-min ui-size-max ui-size-add ui-size-subtract
                 ui-input ui-slot ui-panel)
        (only-in "../ui-components/commands.ss" marlin-ui-candidate-templates))
(export marlin-ui-layout-policy)

(def MessageFrameValue
  (seal-object! (.o min-transcript-rows: 2 fixed-editor-rows: 0
                    footer-rows: 1 extensions: #t prompt: " Message ")))
(def CommandFrameValue
  (seal-object! (.o min-transcript-rows: 4 fixed-editor-rows: 1
                    footer-rows: 0 extensions: #f prompt: " M-x ")))
(def (status-line spans)
  (ui-template 'none (cons '(binding normal #f prefix) spans)))
(def (status-text text)
  (status-line (list (list 'literal 'normal #f text))))
(def (search-line prompt)
  (ui-template 'none (list (list 'literal 'normal #f prompt)
                           '(binding normal #f query))))
(def minibuffer-status
  (ui-choice
    (list (list 'searching (search-line " Searching: "))
          (list 'search-match (search-line " Search: "))
          (list 'search-no-match (search-line " No match: "))
          (list 'notice (status-line '((binding normal #f notice))))
          (list 'messages (status-text "*Messages* · Esc returns to conversation"))
          (list 'static-buffer (status-line '((binding normal #f static-status))))
          (list 'quit-armed (status-text "Press Ctrl-C again to quit; another key cancels. Draft will not be saved."))
          (list 'draft-limit (status-text "Draft limit: 1 MiB. Insertion rejected; draft retained."))
          (list 'runtime (status-text "Runtime: Esc/Ctrl-C return  Enter submit draft"))
          (list 'workspace (status-text "Workspace: PageUp/PageDown scroll  Esc/Ctrl-C return  Enter submit draft"))
          (list 'commands-focus (status-text "Commands: Enter open  M-x search  Ctrl-Q quit"))
          (list 'transcript-focus (status-text "Transcript: scroll to browse  M-x commands  Ctrl-Q quit"))
          (list 'extension-focus (status-text "Extension: Enter activate  M-x commands  Ctrl-Q quit")))
    (status-text "Message: Enter send  Ctrl-R search  M-x commands  Ctrl-Q quit")))
(def editor-view
  (ui-choice
    (list (list 'command (ui-input #f 'bar 'prompt #f 'focused)))
    (ui-input #t 'none #f 'title 'focused)))
(def extension-view
  (ui-choice
    (list (list 'selected
                (ui-template 'none '((literal info #t " › ")
                                     (binding info #t label)))))
    (ui-template 'none '((literal normal #f " · ")
                         (binding normal #f label)))))
;;; The first true fact selects one bounded host-owned content source.
(def main-view
  (ui-choice
    (list (list 'review (ui-slot 'review))
          (list 'messages (ui-slot 'messages))
          (list 'static (ui-panel 'info 'info (ui-slot 'static)))
          (list 'runtime (ui-panel 'info 'info (ui-slot 'runtime)))
          (list 'extension (ui-panel 'info 'info (ui-slot 'extension)))
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
                     (list 'command (ui-size-min (ui-size-binding 'command-limit) candidate-budget)))))
(def extension-size
  (size-choice
    (list (list 'extensions-enabled
                (ui-size-min (ui-size-binding 'extension-count)
                             (ui-size-subtract candidate-budget
                                               (ui-size-binding 'region/candidates)))))))
(def (frame-region slot)
  (let (name (car slot))
    (ui-sized-with
      (case name ((transcript) (ui-size-flex 1)) ((candidates) candidate-size)
        ((extensions) extension-size) ((editor) editor-size) ((modeline) modeline-size)
        ((minibuffer) footer-size) (else (ui-size-binding name)))
      (ui-region name (cdr slot)
                 (case name
                   ((transcript) main-view)
                   ((extensions) extension-view)
                   ((editor) editor-view)
                   ((candidates) marlin-ui-candidate-templates)
                   ((minibuffer) minibuffer-status)
                   (else (ui-text '())))))))
;;; The hover region encloses exactly the editor, candidates and status row.
(.def UiLayoutBase.
  message-frame: MessageFrameValue
  command-frame: CommandFrameValue
  max-chatbox-rows: 10
  max-command-rows: 10
  frame-tree: (ui-rows
    (list (ui-rows (map frame-region '((transcript . transcript) (extensions . #f))))
          (frame-region '(modeline . #f))
          (ui-region 'minibuffer-hover #f
                     (ui-rows
                       (map frame-region '((editor . #f) (candidates . candidates)
                                           (minibuffer . #f))))))))
(def marlin-ui-layout-policy
  (seal-object! UiLayoutBase.))

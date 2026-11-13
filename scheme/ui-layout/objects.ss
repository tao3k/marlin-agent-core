;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme owns presentation choices; the native host clips them to cells.
(import (only-in :clan/poo/object .def .o seal-object!)
        (only-in "../ui-elements/objects.ss"
                 ui-text ui-rows ui-region ui-template ui-choice ui-sized
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
(def (frame-region slot)
  (let (name (car slot))
    (ui-sized (if (eq? name 'transcript) 'flex 'binding)
              (if (eq? name 'transcript) 1 name)
              (ui-region name (cdr slot)
                         (case name
                           ((transcript) main-view)
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
    (list (ui-rows (map frame-region '((transcript . #t) (extensions . #f))))
          (frame-region '(modeline . #f))
          (ui-region 'minibuffer-hover #f
                     (ui-rows
                       (map frame-region '((editor . #f) (candidates . #t)
                                           (minibuffer . #f))))))))
(def marlin-ui-layout-policy
  (seal-object! UiLayoutBase.))

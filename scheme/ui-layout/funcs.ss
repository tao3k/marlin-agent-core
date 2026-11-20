;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme owns presentation choices; the native host clips them to cells.
(import (only-in :clan/poo/object .mix .o .ref seal-object!)
        (only-in "./objects.ss" UiLayoutBase. frame-regions)
        (only-in "../ui-elements/funcs.ss"
                 ui-text ui-rows ui-region ui-scroll-target ui-template ui-choice ui-sized-with
                 ui-size-cells ui-size-flex ui-size-binding ui-size-bounded ui-size-choice
                 ui-size-min ui-size-max ui-size-add ui-size-subtract
                 ui-slot ui-panel ui-list)
        (only-in "../ui-components/commands.ss" marlin-ui-candidate-templates marlin-ui-list-offset)
        (only-in "./minibuffer.ss"
                 minibuffer-echo-policy minibuffer-editor minibuffer-status
                 echo-area-owns-minibuffer dashboard-overhead))
(export marlin-ui-layout-policy-for)

(def extension-view
  (ui-list 'extension-list 'extensions marlin-ui-list-offset (ui-choice
    (list (list 'selected
                (ui-template 'none '((literal info #t " › ")
                                     (binding info #t label)))))
    (ui-template 'none '((literal normal #f " · ")
                         (binding normal #f label))))))
;;; The first true fact selects one bounded host-owned content source.
(def named-panel-title
  (ui-template 'none '((literal normal #f " ") (binding normal #t panel-title))))
(def runtime-panel-title
  (ui-template 'none '((literal normal #t " Runtime policy"))))
(def panel-field
  (ui-template 'none '((binding muted #f label) (literal muted #f ": ")
                       (binding normal #f value))))
(def (main-view transcript-policy)
  (def row-template
    (ui-template 'none
      (list (list 'binding (.ref transcript-policy 'role-tone)
                  (.ref transcript-policy 'role-bold) 'role-text)
            '(binding normal #f body-text))))
  (def (transcript-list source)
    (ui-list source source (ui-size-cells 0) row-template))
  (ui-choice
    (list (list 'review (transcript-list 'review))
          (list 'workspace (transcript-list 'workspace))
          (list 'messages (transcript-list 'messages))
          (list 'static (ui-panel #f panel-field named-panel-title (ui-slot 'selected-buffer)))
          (list 'runtime (ui-panel #f panel-field runtime-panel-title (ui-slot 'runtime-policy)))
          (list 'extension (ui-panel #f panel-field named-panel-title (ui-slot 'focused-extension))))
    (transcript-list 'conversation)))

;;; Size expressions contain only named facts and host-provided cell metrics.
(def (size-choice cases) (ui-size-choice cases (ui-size-cells 0)))
;;; Keep the transcript and minibuffer visible before allocating editor rows.
(def terminal-height (ui-size-binding 'terminal-height))
(def transcript-reservation
  (ui-size-min (ui-size-binding 'transcript-min)
               (ui-size-subtract terminal-height (ui-size-cells 2))))
(def footer-reservation
  (ui-size-choice
    (list (list 'transient
                (ui-size-add (ui-size-binding 'hint-rows) dashboard-overhead))
          (list echo-area-owns-minibuffer (ui-size-cells 1))
          (list 'search-no-match (ui-size-cells 1))
          (list 'editor-expanded (ui-size-cells 0)))
    (ui-size-binding 'footer-min)))
(def tab-editor-rows
  (ui-size-choice
    (list (list 'editor-fixed (ui-size-binding 'fixed-editor))
          (list echo-area-owns-minibuffer (ui-size-cells 1))
          (list 'editor-expanded (ui-size-binding 'measured-editor)))
    (ui-size-cells 1)))
(def tab-size
  (ui-size-min (ui-size-cells 1)
    (ui-size-subtract terminal-height
      (ui-size-add
        (ui-size-add (ui-size-binding 'transcript-min) footer-reservation)
        (ui-size-add tab-editor-rows (ui-size-cells 1))))))
(def tab-rows (ui-size-binding 'region/buffer-tabs))
(def editor-reservation
  (ui-size-add
    (ui-size-add (size-choice (list (list 'search-no-match (ui-size-cells 1))))
                 (ui-size-add tab-rows (ui-size-cells 1)))
    transcript-reservation))
(def editor-budget
  (ui-size-min (ui-size-binding 'chatbox-limit)
               (ui-size-subtract terminal-height editor-reservation)))
(def editor-size
  (size-choice (list (list 'editor-fixed (ui-size-bounded 'fixed-editor 'terminal-height))
                     (list echo-area-owns-minibuffer (ui-size-cells 0))
                     (list 'editor-expanded
                           (ui-size-min (ui-size-binding 'measured-editor)
                                        editor-budget)))))
(def (frame-region-id role) (cdr (assq role frame-regions)))
(def (frame-region-extent role)
  (ui-size-binding (string->symbol (string-append "region/" (symbol->string (frame-region-id role))))))
(def editor-rows (frame-region-extent 'editor))
(def footer-size
  (ui-size-min footer-reservation
    (ui-size-max (ui-size-min (ui-size-cells 1) terminal-height)
                 (ui-size-subtract terminal-height
                   (ui-size-add (ui-size-add transcript-reservation editor-rows)
                                (ui-size-add tab-rows (ui-size-cells 1)))))))
(def spare-rows
  (ui-size-subtract terminal-height
    (ui-size-add (ui-size-add (ui-size-binding 'region/minibuffer)
                              transcript-reservation)
                 (ui-size-add tab-rows editor-rows))))
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
                                               (frame-region-extent 'candidates)))))))
(def (frame-region modeline-value transcript-value slot)
  (let (name (car slot))
    (ui-sized-with
      (case name ((transcript) (ui-size-flex 1)) ((candidates) candidate-size)
        ((extensions) extension-size) ((editor) editor-size) ((modeline) modeline-size)
        ((minibuffer) footer-size) (else (ui-size-binding name)))
      (ui-region (frame-region-id name) (and (cadr slot) (ui-scroll-target (cadr slot) (caddr slot)))
                 (case name
                   ((transcript) (main-view transcript-value))
                   ((extensions) extension-view)
                   ((editor) minibuffer-editor)
                   ((candidates) marlin-ui-candidate-templates)
                   ((modeline) modeline-value)
                   ((minibuffer) minibuffer-status)
                   (else (ui-text '())))))))
;;; The hover region encloses exactly the editor, candidates and status row.
(def (marlin-ui-layout-policy-for modeline-value transcript-value tab-bar)
  (let (region (lambda (slot) (frame-region modeline-value transcript-value slot)))
    (seal-object!
      (.mix (.o echo-policy: minibuffer-echo-policy
                frame-tree:
                (ui-rows
                  (list (ui-sized-with tab-size (ui-region 'buffer-tabs #f tab-bar))
                        (ui-rows (map region '((transcript transcript 3) (extensions #f 0))))
                        (region '(modeline #f 0))
                        (ui-region (frame-region-id 'minibuffer-hover) #f
                          (ui-rows (map region '((editor #f 0) (candidates candidates 1)
                                                 (minibuffer #f 0))))))))
            UiLayoutBase.))))

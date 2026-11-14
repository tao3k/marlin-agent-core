;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Component constructors copy and admit presentation state. Runtime effects
;;; and terminal cells are outside this module's authority.
(import (only-in :clan/poo/object .def .o .ref seal-object!)
        (only-in :tao3k/yuanxu/poo-clos/types clos-instance?)
        (only-in "./bounds.ss" marlin-ui-max-content-items marlin-ui-bounded-label?
                 marlin-ui-bounded-list? marlin-ui-required-object)
        (only-in "./buffers.ss" marlin-ui-buffer-static-entries))
(export UiComponent. UiWorkbench. UiTranscript. UiComposer. UiCommands. UiStatus.
        UiStatusItem. UiExtension. UiText. UiField.
        marlin-ui-contribution-node marlin-ui-modeline-node marlin-ui-extension-node
        marlin-ui-static-buffer-nodes)

(.def UiComponent.)
(.def (UiWorkbench. @ UiComponent.))
(.def (UiTranscript. @ UiComponent.))
(.def (UiComposer. @ UiComponent.))
(.def (UiCommands. @ UiComponent.))
(.def (UiStatus. @ UiComponent.))
(.def (UiStatusItem. @ UiComponent.))
(.def (UiExtension. @ UiComponent.))
(.def (UiText. @ UiComponent.))
(.def (UiField. @ UiComponent.))

(def reserved-identities '(transcript composer status commands))
(def modeline-identities
  '(modeline/left/brand modeline/left/model
    modeline/left/activity modeline/right/turns modeline/right/session
    modeline/state/online modeline/state/offline modeline/state/unknown
    modeline/state/runtime modeline/state/workspace modeline/state/recovery
    modeline/state/continuation modeline/state/history modeline/state/working
    modeline/state/reading))

(def (marlin-ui-dynamic-modeline-template? value)
  (and (string? value)
       (let ((open (string-index value #\{))
             (close (string-index value #\})))
         ;; Exactly one complete placeholder and no unmatched braces.
         (and open close (= close (+ open 6))
              (string=? (substring value open (+ close 1)) "{value}")
              (not (string-index value #\{ (+ close 1)))
              (not (string-index value #\} (+ close 1)))))))

(def (marlin-ui-modeline-node descriptor)
  (marlin-ui-required-object descriptor '(identity label compact tone active-tone min-width compact-below keep-priority))
  (let ((identity-value (.ref descriptor 'identity))
        (label-value (.ref descriptor 'label))
        (compact-value (.ref descriptor 'compact))
        (tone-value (.ref descriptor 'tone))
        (active-tone-value (.ref descriptor 'active-tone))
        (min-width-value (.ref descriptor 'min-width))
        (compact-below-value (.ref descriptor 'compact-below))
        (keep-priority-value (.ref descriptor 'keep-priority)))
    (unless (and (memq identity-value modeline-identities)
                 (marlin-ui-bounded-label? label-value)
                 (marlin-ui-bounded-label? compact-value)
                 (memq tone-value '(accent info success warning muted))
                 (memq active-tone-value '(accent info success warning muted))
                 (integer? min-width-value) (<= 0 min-width-value 256)
                 (integer? compact-below-value) (<= 0 compact-below-value 256)
                 (integer? keep-priority-value) (<= 0 keep-priority-value 3)
                 (if (memq identity-value '(modeline/left/model modeline/right/turns
                                             modeline/right/session))
                   (> keep-priority-value 0)
                   (= keep-priority-value 0))
                 (if (memq identity-value '(modeline/left/model modeline/right/turns
                                             modeline/right/session))
                   (and (marlin-ui-dynamic-modeline-template? label-value)
                        (marlin-ui-dynamic-modeline-template? compact-value))
                   (and (not (equal? label-value ""))
                        (not (equal? compact-value "")))))
      (error "invalid UI modeline descriptor"))
    (.o (:: @ UiStatusItem.) identity: identity-value kind: 'status-item
        label: (string-copy label-value) compact: (string-copy compact-value)
        tone: tone-value active-tone: active-tone-value
        min-width: min-width-value compact-below: compact-below-value
        keep-priority: keep-priority-value)))

(def (marlin-ui-contribution-node contribution)
  (marlin-ui-required-object contribution '(identity label))
  (let ((identity-value (.ref contribution 'identity)) (label-value (.ref contribution 'label)))
    (unless (and (symbol? identity-value) (marlin-ui-bounded-label? label-value)
                 (not (memq identity-value reserved-identities))
                 (not (string-prefix? "modeline/" (symbol->string identity-value))))
      (error "invalid UI contribution"))
    (.o (:: @ UiStatusItem.) identity: identity-value kind: 'status-item
        label: (string-copy label-value) compact: (string-copy label-value)
        tone: 'muted active-tone: 'muted min-width: 0 compact-below: 0
        keep-priority: 0)))

(def (checked-content item)
  (marlin-ui-required-object item '(kind))
  (cond
   ((and (clos-instance? UiText. item) (eq? (.ref item 'kind) 'text))
    (marlin-ui-required-object item '(text))
    (unless (marlin-ui-bounded-label? (.ref item 'text)) (error "invalid UI text content"))
    (seal-object! (.o (:: @ UiText.) kind: 'text text: (string-copy (.ref item 'text)))))
   ((and (clos-instance? UiField. item) (eq? (.ref item 'kind) 'field))
    (marlin-ui-required-object item '(label value))
    (unless (and (marlin-ui-bounded-label? (.ref item 'label))
                 (marlin-ui-bounded-label? (.ref item 'value)))
      (error "invalid UI field content"))
    (seal-object! (.o (:: @ UiField.) kind: 'field
                      label: (string-copy (.ref item 'label))
                      value: (string-copy (.ref item 'value)))))
   (else (error "unsupported UI content component"))))

(def (marlin-ui-extension-node component)
  (marlin-ui-required-object component '(identity kind label focusable content))
  (unless (and (clos-instance? UiExtension. component)
               (symbol? (.ref component 'identity))
               (not (memq (.ref component 'identity) reserved-identities))
               (eq? (.ref component 'kind) 'extension)
               (marlin-ui-bounded-label? (.ref component 'label))
               (boolean? (.ref component 'focusable))
               (marlin-ui-bounded-list? (.ref component 'content) marlin-ui-max-content-items))
    (error "invalid UI extension component"))
  ;; Preserve the contributor's prototype for generic dispatch while owning
  ;; every mutable field that can be projected to the host.
  (let ((identity-value (.ref component 'identity))
        (label-value (string-copy (.ref component 'label)))
        (focusable-value (.ref component 'focusable))
        (content-value (map checked-content (.ref component 'content))))
    (seal-object! (.o (:: @ component) identity: identity-value kind: 'extension
                      label: label-value focusable: focusable-value content: content-value))))

;;; Static Buffer content shares the same bounded component contract as host
;;; extensions. The Buffer catalog is the sole source of its identity and text.
(def (marlin-ui-static-buffer-nodes)
  (map (lambda (entry)
         (let* ((presentation (caddr entry))
                (lines (cadr presentation)))
           (marlin-ui-extension-node
             (.o (:: @ UiExtension.) identity: (car entry) kind: 'extension
                 label: (car presentation) focusable: #f
                 content: (map (lambda (line-value)
                                 (.o (:: @ UiText.) kind: 'text
                                     text: (string-copy line-value)))
                               lines)))))
       (marlin-ui-buffer-static-entries)))

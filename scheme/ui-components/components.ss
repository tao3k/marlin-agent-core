;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Component constructors copy and admit presentation state. Runtime effects
;;; and terminal cells are outside this module's authority.
(import (only-in :clan/poo/object .def .o .ref seal-object!)
        (only-in :yuanxu/poo-clos/types clos-instance?)
        (only-in "./bounds.ss" marlin-ui-max-content-items marlin-ui-bounded-label?
                 marlin-ui-bounded-list? marlin-ui-required-object))
(export UiComponent. UiWorkbench. UiTranscript. UiComposer. UiCommands. UiStatus.
        UiStatusItem. UiExtension. UiGuide. UiText. UiField.
        marlin-ui-contribution-node marlin-ui-extension-node marlin-ui-guide-node)

(.def UiComponent.)
(.def (UiWorkbench. @ UiComponent.))
(.def (UiTranscript. @ UiComponent.))
(.def (UiComposer. @ UiComponent.))
(.def (UiCommands. @ UiComponent.))
(.def (UiStatus. @ UiComponent.))
(.def (UiStatusItem. @ UiComponent.))
(.def (UiExtension. @ UiComponent.))
(.def (UiGuide. @ UiExtension.))
(.def (UiText. @ UiComponent.))
(.def (UiField. @ UiComponent.))

(def reserved-identities '(transcript composer status commands guide))

(def (marlin-ui-contribution-node contribution)
  (marlin-ui-required-object contribution '(identity label))
  (let ((identity-value (.ref contribution 'identity)) (label-value (.ref contribution 'label)))
    (unless (and (symbol? identity-value) (marlin-ui-bounded-label? label-value)
                 (not (memq identity-value reserved-identities)))
      (error "invalid UI contribution"))
    (.o (:: @ UiStatusItem.) identity: identity-value kind: 'status-item
        label: (string-copy label-value))))

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

;;; The selected package contributes a host-neutral guide. A renderer supplies
;;; keyboard shortcuts, mouse gestures or other device-specific affordances.
(def (marlin-ui-guide-node)
  (seal-object!
   (.o (:: @ UiGuide.) identity: 'guide kind: 'extension label: "Guide"
       focusable: #t
       content: (map checked-content
                     (list (.o (:: @ UiText.) kind: 'text text: "Get started")
                           (.o (:: @ UiField.) kind: 'field label: "Conversation"
                               value: "Write a message")
                           (.o (:: @ UiField.) kind: 'field label: "Actions"
                               value: "Browse commands")
                           (.o (:: @ UiField.) kind: 'field label: "Navigation"
                               value: "Focus a region")
                           (.o (:: @ UiText.) kind: 'text
                               text: "Choose Message to begin."))))))

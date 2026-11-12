;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Multi-dispatch of a component and an event. Extensions compose their own
;;; methods into a fresh generic at module construction, not during painting.
(import (only-in :clan/poo/object .def .o .ref)
        (only-in :yuanxu/poo-clos/objects poo-clos-generic-protocol
                 poo-clos-generic-binding-current poo-clos-compose-method-bundle)
        (only-in :yuanxu/poo-clos/generic-syntax .defgeneric .defmethod)
        (only-in :yuanxu/poo-clos/method-bundle-syntax .defmethod-bundle)
        (only-in :yuanxu/poo-clos/dispatch poo-clos-call))
(export UiComponent. UiWorkbench. UiTranscript. UiComposer. UiCommands. UiStatus. UiStatusItem. UiExtension.
        UiEvent. UiNextFocus. UiSubmit. UiDismiss. UiScroll. UiEventProtocol make-ui-event-generic
        ui-event-dispatch)

(.def UiComponent.)
(.def (UiWorkbench. @ UiComponent.))
(.def (UiTranscript. @ UiComponent.))
(.def (UiComposer. @ UiComponent.))
(.def (UiCommands. @ UiComponent.))
(.def (UiStatus. @ UiComponent.))
(.def (UiStatusItem. @ UiComponent.))
(.def (UiExtension. @ UiComponent.))
(.def UiEvent.)
(.def (UiNextFocus. @ UiEvent.))
(.def (UiSubmit. @ UiEvent.))
(.def (UiDismiss. @ UiEvent.))
(.def (UiScroll. @ UiEvent.))
(def UiEventProtocol (poo-clos-generic-protocol 'marlin/ui-event))

;;; Focus order belongs to the composed workbench, not to fixed widget names.
(def (next-focus component event)
  (def identity-value (.ref component 'identity))
  (def order (.ref event 'order))
  (let check ((rest order) (seen '()) (count 0))
    (cond
     ((null? rest)
      (let (entry (memq identity-value order))
        (unless entry (error "focused component is not in the focus order"))
        (.o kind: 'focus identity: (if (pair? (cdr entry)) (cadr entry) (car order)))))
     ((or (not (pair? rest)) (>= count 64) (not (symbol? (car rest)))
          (memq (car rest) seen))
      (error "invalid UI focus order"))
     (else (check (cdr rest) (cons (car rest) seen) (+ count 1))))))

(def (make-ui-event-generic bundles)
  (unless (and (vector? bundles) (<= (vector-length bundles) 16))
    (error "invalid UI method bundles"))
  ;; Each UI instance owns its own declarative generic and method bundle.
  (.defgeneric ui-event (component event) (protocol UiEventProtocol))
  (.defmethod-bundle UiCoreEventMethods UiEventProtocol
    (.defmethod (ui-event (component (prototype UiComponent.))
                          (event (prototype UiNextFocus.)))
      (next-focus component event))
    ;; These are intents only: the host still owns draft submission and I/O.
    (.defmethod (ui-event (component (prototype UiComposer.))
                          (event (prototype UiSubmit.)))
      (.o kind: 'submit-draft))
    (.defmethod (ui-event (component (prototype UiCommands.))
                          (event (prototype UiDismiss.)))
      (.o kind: 'focus identity: 'composer))
    (.defmethod (ui-event (component (prototype UiTranscript.))
                          (event (prototype UiScroll.)))
      (let (direction-value (.ref event 'direction))
        (unless (memq direction-value '(up down)) (error "invalid UI scroll direction"))
        (.o kind: 'scroll direction: direction-value))))
  (let (generic (poo-clos-compose-method-bundle
                  (poo-clos-generic-binding-current ui-event$poo-clos-generic)
                  UiCoreEventMethods))
    ;; Composition is ordered and occurs on a fresh unpublished generic.
    ;; A failing bundle cannot partially modify an existing session.
    (for-each (lambda (bundle) (poo-clos-compose-method-bundle generic bundle)) (vector->list bundles))
    generic))

(def (ui-event-dispatch generic component event)
  (poo-clos-call generic component event))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Multi-dispatch of a component and an event. Extensions compose their own
;;; methods into a fresh generic at module construction, not during painting.
(import (only-in :clan/poo/object .def .o .ref)
        (only-in :clan/poo/mop
                 .defgeneric .defmethod .defmethod-bundle
                 clos-generic-protocol clos-generic-binding-current
                 clos-compose-method-bundle clos-call)
        (only-in :std/list/list duplicates)
        (only-in "../ui-components/bounds.ss" marlin-ui-bounded-list?)
        (only-in "../ui-components/commands.ss" marlin-ui-browse-command)
        (only-in "./types.ss" ui-method-factories?)
        "../ui-components/components.ss")
(export UiEvent. UiNextFocus. UiPreviousFocus. UiSubmit. UiDismiss. UiScroll.
        UiBrowseCommand. UiSelectListRow. UiEventProtocol make-ui-event-generic
        ui-event-dispatch)
(.def UiEvent.)
(.def (UiNextFocus. @ UiEvent.))
(.def (UiPreviousFocus. @ UiNextFocus.))
(.def (UiSubmit. @ UiEvent.))
(.def (UiDismiss. @ UiEvent.))
(.def (UiScroll. @ UiEvent.))
(.def (UiBrowseCommand. @ UiEvent.))
(.def (UiSelectListRow. @ UiEvent.))
(def UiEventProtocol (clos-generic-protocol 'marlin/ui-event))

;;; Focus order belongs to the composed workbench, not to fixed widget names.
(def (move-focus component event backwards?)
  (def identity-value (.ref component 'identity))
  (def order (.ref event 'order))
  (unless (and (marlin-ui-bounded-list? order 64) (pair? order)
               (andmap symbol? order) (null? (duplicates order eq?)))
    (error "invalid UI focus order"))
  ;; Reverse only for backward navigation; memq keeps the existing ordered tail.
  (let* ((sequence (if backwards? (reverse order) order))
         (entry (memq identity-value sequence)))
    (unless entry (error "focused component is not in the focus order"))
    (.o kind: 'focus identity:
        (if (pair? (cdr entry)) (cadr entry) (car sequence)))))

(def (make-ui-event-generic host-factories package-factories)
  (unless (ui-method-factories? host-factories)
    (error "invalid UI method factories"))
  (def factories
    (if (zero? (vector-length package-factories)) host-factories
        (vector-append package-factories host-factories)))
  (unless (or (eq? factories host-factories) (ui-method-factories? factories))
    (error "invalid UI method factories"))
  ;; Each UI instance owns its own declarative generic and method bundle.
  (.defgeneric ui-event (component event) (protocol UiEventProtocol))
  (.defmethod-bundle UiCoreEventMethods UiEventProtocol
    (.defmethod (ui-event (component (prototype UiComponent.))
                          (event (prototype UiNextFocus.)))
      (move-focus component event #f))
    (.defmethod (ui-event (component (prototype UiComponent.))
                          (event (prototype UiPreviousFocus.)))
      (move-focus component event #t))
    (.defmethod (ui-event (component (prototype UiCommands.))
                          (event (prototype UiBrowseCommand.)))
      (marlin-ui-browse-command (.ref component 'entries) event))
    (.defmethod (ui-event list-id (event (prototype UiSelectListRow.)))
      (.o kind: 'focus identity: (.ref event 'current-focus)))
    ;; These are intents only: the host still owns draft submission and I/O.
    (.defmethod (ui-event (component (prototype UiComposer.))
                          (event (prototype UiSubmit.)))
      (.o kind: 'submit-draft))
    (.defmethod (ui-event (component (prototype UiCommands.))
                          (event (prototype UiDismiss.)))
      (.o kind: 'focus identity: 'composer))
    (.defmethod (ui-event (component (prototype UiExtension.))
                          (event (prototype UiSubmit.)))
      (.o kind: 'focus identity: 'composer))
    (.defmethod (ui-event (component (prototype UiTranscript.))
                          (event (prototype UiScroll.)))
      (let (direction-value (.ref event 'direction))
        (unless (memq direction-value '(up down)) (error "invalid UI scroll direction"))
        (.o kind: 'scroll direction: direction-value))))
  (let (generic (clos-compose-method-bundle
                  (clos-generic-binding-current ui-event$clos-generic)
                  UiCoreEventMethods))
    ;; POO methods belong to one generic. Each factory creates fresh methods
    ;; while retaining its package's component/protocol identities. Compose
    ;; before publication so failure cannot alter an existing session.
    (for-each (lambda (factory)
                (clos-compose-method-bundle generic (factory)))
              (vector->list factories))
    generic))

(def (ui-event-dispatch generic component event)
  (clos-call generic component event))

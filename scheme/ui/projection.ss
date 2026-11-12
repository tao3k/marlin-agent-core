;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Structured native projection of the selected UI publication. This is a
;;; vector of Scheme values, not a text serialization or a terminal command.
(import (only-in :clan/poo/object .get .o)
        (only-in :std/list/list find)
        (only-in :yuanxu/poo-clos/types clos-instance?))
(export marlin-ui-native-facts marlin-ui-project-workbench marlin-ui-apply-native
        marlin-ui-project-intent marlin-ui-apply-event)

(def (contribution row)
  (unless (and (vector? row) (= (vector-length row) 2))
    (error "invalid UI native contribution"))
  (let ((identity-value (vector-ref row 0)) (label-value (vector-ref row 1)))
    (.o identity: identity-value label: label-value)))

(def (node-value node)
  (let ((identity-value (.get node identity)) (kind-value (.get node kind)))
    (case kind-value
      ((status)
       (vector identity-value kind-value
         (list->vector (map (lambda (child)
                             (vector (.get child identity) (.get child kind) (.get child label)))
                           (.get node children)))))
      ((extension)
       (vector identity-value kind-value
         (vector (.get node label) (.get node focusable)
                 (list->vector (map content-value (.get node content))))))
      ((commands) (vector identity-value kind-value
                          (list->vector (map list->vector (.get node entries)))))
      (else (vector identity-value kind-value #())))))

(def (content-value item)
  (case (.get item kind)
    ((text) (vector 'text (.get item text)))
    ((field) (vector 'field (.get item label) (.get item value)))
    (else (error "unsupported UI content projection"))))

(def (marlin-ui-native-facts input)
  (unless (and (vector? input) (= (vector-length input) 4)
               (vector? (vector-ref input 2)) (vector? (vector-ref input 3)))
    (error "invalid UI native request"))
  (let* ((model-value (vector-ref input 0))
         (session-value (vector-ref input 1))
         (native-contributions (vector-ref input 2))
         (native-extensions (vector-ref input 3))
         (_contribution-limit (when (> (vector-length native-contributions) 8)
                                (error "too many UI native contributions")))
         (_extension-limit (when (> (vector-length native-extensions) 16)
                             (error "too many UI native extensions")))
         (contribution-values (map contribution (vector->list native-contributions))))
    (.o model: model-value session: session-value contributions: contribution-values
        extensions: (vector->list native-extensions))))

(def (marlin-ui-project-workbench workbench)
  (vector (.get workbench model) (.get workbench session)
          (list->vector (map node-value (.get workbench regions)))))

;;; The caller supplies the already-linked scope, so extensions and admission
;;; remain owned by the Module System rather than a private default instance.
(def (marlin-ui-apply-native ui input)
  (let (workbench ((.get ui workbench) (marlin-ui-native-facts input)))
    ((.get ui project-workbench) workbench)))

;;; Only established host intents cross this boundary. An extension's new
;;; intent requires an explicit host protocol rather than an arbitrary effect.
(def (marlin-ui-project-intent intent)
  (case (.get intent kind)
    ((submit-draft) (vector 'submit-draft))
    ((focus)
     (let (identity-value (.get intent identity))
       (unless (symbol? identity-value) (error "invalid UI focus intent"))
       (vector 'focus identity-value)))
    ((scroll)
     (let (direction-value (.get intent direction))
       (unless (memq direction-value '(up down)) (error "invalid UI scroll intent"))
       (vector 'scroll direction-value)))
    (else (error "unsupported UI host intent"))))

(def (marlin-ui-apply-event ui generic workbench focus-value event)
  (let* ((order (.get workbench focus-order))
         (_focus-check (unless (and (symbol? focus-value) (memq focus-value order))
                         (error "UI focus is not in the workbench")))
         (component (find (lambda (region) (eq? (.get region identity) focus-value))
                          (.get workbench regions))))
    (unless component (error "focused UI component is missing"))
    (when (and (clos-instance? (.get ui events next-focus) event)
               (not (equal? (.get event order) order)))
      (error "UI focus event does not match the workbench"))
    (let (intent (marlin-ui-project-intent ((.get ui dispatch) generic component event)))
      (when (and (eq? (vector-ref intent 0) 'focus)
                 (not (memq (vector-ref intent 1) order)))
        (error "UI focus intent leaves the workbench"))
      intent)))

;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; POO dynamic hook actions produced by Scheme-owned policy decisions.

package: marlin

(import (only-in :clan/poo/object .get .o)
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-matcher)

(export marlin-deck-runtime-dynamic-hook-action-kind
        marlin-deck-runtime-dynamic-hook-case-kind
        marlin-deck-runtime-dynamic-hook-selector-kind
        marlin-deck-runtime-dynamic-hook-selection-kind
        marlin-deck-runtime-dynamic-hook-action-names
        make-marlin-deck-runtime-dynamic-hook-action
        make-marlin-deck-runtime-dynamic-hook-case
        make-marlin-deck-runtime-dynamic-hook-selector
        marlin-deck-runtime-dynamic-hook-selector-selection
        marlin-deck-runtime-dynamic-hook-selector-select
        marlin-deck-runtime-dynamic-hook-decision-selection
        make-marlin-deck-runtime-register-hook-action
        make-marlin-deck-runtime-unregister-hook-action
        make-marlin-deck-runtime-defer-hook-action
        make-marlin-deck-runtime-deny-hook-action
        make-marlin-deck-runtime-rewrite-hook-action
        make-marlin-deck-runtime-allow-hook-action)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-dynamic-hook-action-kind
  "marlin-deck-runtime.dynamic-hook-action.v1")

;;; Boundary: Selector cases pair Scheme policy predicates with hook actions.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-dynamic-hook-case-kind
  "marlin-deck-runtime.dynamic-hook-case.v1")

;;; Boundary: Selectors decide runtime hook actions without Rust DSL logic.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-dynamic-hook-selector-kind
  "marlin-deck-runtime.dynamic-hook-selector.v1")

;;; Boundary: Selection receipts preserve Scheme selector provenance.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-dynamic-hook-selection-kind
  "marlin-deck-runtime.dynamic-hook-selection.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-hook-action-names)
  '("allow" "defer" "deny" "rewrite" "register" "unregister"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-dynamic-hook-action
      action-value
      hook-id-value
      registration-value
      defer-reason-value
      deny-reason-value
      rewrite-command-value)
  (.o kind: marlin-deck-runtime-dynamic-hook-action-kind
      action: action-value
      hook-id: hook-id-value
      registration: registration-value
      defer-reason: defer-reason-value
      deny-reason: deny-reason-value
      rewrite-command: rewrite-command-value))

;;; Boundary: Hook cases keep runtime decisions as typed POO objects.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-dynamic-hook-case
      case-name-value
      condition-value
      matcher-value
      action-value)
  (.o kind: marlin-deck-runtime-dynamic-hook-case-kind
      name: case-name-value
      condition: condition-value
      matcher: matcher-value
      dynamic-hook-action: action-value))

;;; Boundary: Selectors hold ordered runtime hook decision cases.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-dynamic-hook-selector
      selector-name-value
      case-values
      default-action-value)
  (.o kind: marlin-deck-runtime-dynamic-hook-selector-kind
      name: selector-name-value
      cases: case-values
      default-action: default-action-value))

;;; Boundary: Runtime hook selection stays in Scheme before typed projection.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-hook-case-match?
      hook-case context policy command agent-scope)
  (and
   (marlin-deck-runtime-condition-policy-match?
    (.get hook-case condition)
    context)
   (marlin-deck-runtime-high-order-matcher-match?
    (.get hook-case matcher)
    context
    policy
    command
    agent-scope)))

;;; Boundary: First matching case decides the hook action with provenance.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-hook-selector-selection
      selector context policy command agent-scope)
  (let (selected-case
        (find (lambda (hook-case)
                (marlin-deck-runtime-dynamic-hook-case-match?
                 hook-case
                 context
                 policy
                 command
                 agent-scope))
              (.get selector cases)))
    (if selected-case
      (let ((selector-name-value (.get selector name))
            (case-name-value (.get selected-case name))
            (action-value (.get selected-case dynamic-hook-action)))
        (.o kind: marlin-deck-runtime-dynamic-hook-selection-kind
            source: "selector-case"
            selector: selector-name-value
            matched: #t
            matched-case: case-name-value
            dynamic-hook-action: action-value))
      (let ((selector-name-value (.get selector name))
            (action-value (.get selector default-action)))
        (.o kind: marlin-deck-runtime-dynamic-hook-selection-kind
            source: "selector-default"
            selector: selector-name-value
            matched: #f
            matched-case: #f
            dynamic-hook-action: action-value)))))

;;; Boundary: Selector-select keeps the user-facing API action-shaped.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-hook-selector-select
      selector context policy command agent-scope)
  (.get (marlin-deck-runtime-dynamic-hook-selector-selection
         selector
         context
         policy
         command
         agent-scope)
        dynamic-hook-action))

;;; Boundary: Hook decisions normalize direct actions and selectors.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-hook-decision-selection
      hook-decision direct-source-value context policy command agent-scope)
  (if (and hook-decision
           (string=? (.get hook-decision kind)
                     marlin-deck-runtime-dynamic-hook-selector-kind))
    (marlin-deck-runtime-dynamic-hook-selector-selection
     hook-decision
     context
     policy
     command
     agent-scope)
    (let ((action-value hook-decision)
          (matched-value (if hook-decision #t #f)))
      (.o kind: marlin-deck-runtime-dynamic-hook-selection-kind
          source: direct-source-value
          selector: #f
          matched: matched-value
          matched-case: #f
          dynamic-hook-action: action-value))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-register-hook-action hook-id registration)
  (make-marlin-deck-runtime-dynamic-hook-action
   "register" hook-id registration #f #f #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-unregister-hook-action hook-id)
  (make-marlin-deck-runtime-dynamic-hook-action
   "unregister" hook-id #f #f #f #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-defer-hook-action reason)
  (make-marlin-deck-runtime-dynamic-hook-action
   "defer" #f #f reason #f #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-deny-hook-action reason)
  (make-marlin-deck-runtime-dynamic-hook-action
   "deny" #f #f #f reason #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-rewrite-hook-action command)
  (make-marlin-deck-runtime-dynamic-hook-action
   "rewrite" #f #f #f #f command))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-allow-hook-action)
  (make-marlin-deck-runtime-dynamic-hook-action
   "allow" #f #f #f #f #f))

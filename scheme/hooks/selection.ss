;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; POO dynamic hook actions produced by Scheme-owned policy decisions.

(import (only-in :clan/poo/object .get .o)
        "./conditions.ss"
        "./matchers.ss")

(export hook-action-kind
        hook-case-kind
        hook-selector-kind
        hook-selection-kind
        hook-action-names
        make-hook-action
        make-hook-case
        make-hook-selector
        hook-selector-selection
        hook-selector-select
        select-hook
        make-register-hook-action
        make-unregister-hook-action
        make-defer-hook-action
        make-deny-hook-action
        make-rewrite-hook-action
        make-allow-hook-action)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def hook-action-kind
  "marlin-deck-runtime.dynamic-hook-action.v1")

;;; Boundary: Selector cases pair Scheme policy predicates with hook actions.
;; MarlinResult <- MarlinInput
(def hook-case-kind
  "marlin-deck-runtime.dynamic-hook-case.v1")

;;; Boundary: Selectors decide runtime hook actions without Rust DSL logic.
;; MarlinResult <- MarlinInput
(def hook-selector-kind
  "marlin-deck-runtime.dynamic-hook-selector.v1")

;;; Boundary: Selection receipts preserve Scheme selector provenance.
;; MarlinResult <- MarlinInput
(def hook-selection-kind
  "marlin-deck-runtime.dynamic-hook-selection.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (hook-action-names)
  '("allow" "defer" "deny" "rewrite" "register" "unregister"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-hook-action
      action-value
      hook-id-value
      defer-reason-value
      deny-reason-value
      rewrite-command-value)
  (.o kind: hook-action-kind
      action: action-value
      hook-id: hook-id-value
      defer-reason: defer-reason-value
      deny-reason: deny-reason-value
      rewrite-command: rewrite-command-value))

;;; Boundary: Hook cases keep runtime decisions as typed POO objects.
;; MarlinResult <- MarlinInput
(def (make-hook-case
      case-name-value
      condition-value
      matcher-value
      action-value)
  (.o kind: hook-case-kind
      name: case-name-value
      condition: condition-value
      matcher: matcher-value
      dynamic-hook-action: action-value))

;;; Boundary: Selectors hold ordered runtime hook decision cases.
;; MarlinResult <- MarlinInput
(def (make-hook-selector
      selector-name-value
      case-values
      default-action-value)
  (.o kind: hook-selector-kind
      name: selector-name-value
      cases: case-values
      default-action: default-action-value))

;;; Boundary: Runtime hook selection stays in Scheme before typed projection.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-hook-case-match?
      hook-case context policy command agent-scope)
  (and
   (hook-condition-match?
    (.get hook-case condition)
    context)
   (hook-matcher-match?
    (.get hook-case matcher)
    context
    policy
    command
    agent-scope)))

(def (hook-selection source-value selector-name case-name matched? action-value)
  (.o kind: hook-selection-kind
      source: source-value
      selector: selector-name
      matched: matched?
      matched-case: case-name
      dynamic-hook-action: action-value))

;;; Boundary: First matching case decides the hook action with provenance.
;; MarlinResult <- MarlinInput
(def (hook-selector-selection
      selector context policy command agent-scope)
  (def selected-case
    (find (cut marlin-deck-runtime-dynamic-hook-case-match? <> context policy command agent-scope)
          (.get selector cases)))
  (def selector-name (.get selector name))
  (if selected-case
    (hook-selection "selector-case" selector-name (.get selected-case name)
                    #t (.get selected-case dynamic-hook-action))
    (hook-selection "selector-default" selector-name #f #f (.get selector default-action))))

;;; Boundary: Selector-select keeps the user-facing API action-shaped.
;; MarlinResult <- MarlinInput
(def (hook-selector-select
      selector context policy command agent-scope)
  (.get (hook-selector-selection
         selector
         context
         policy
         command
         agent-scope)
        dynamic-hook-action))

;;; Boundary: Hook decisions normalize direct actions and selectors.
;; MarlinResult <- MarlinInput
(def (select-hook
      hook-decision direct-source-value context policy command agent-scope)
  (if (and hook-decision
           (string=? (.get hook-decision kind)
                     hook-selector-kind))
    (hook-selector-selection
     hook-decision
     context
     policy
     command
     agent-scope)
    (hook-selection direct-source-value #f #f (and hook-decision #t) hook-decision)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-register-hook-action catalog-id)
  (make-hook-action
   "register" catalog-id #f #f #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-unregister-hook-action hook-id)
  (make-hook-action
   "unregister" hook-id #f #f #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-defer-hook-action reason)
  (make-hook-action
   "defer" #f reason #f #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-deny-hook-action reason)
  (make-hook-action
   "deny" #f #f reason #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-rewrite-hook-action command)
  (make-hook-action
   "rewrite" #f #f #f command))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-allow-hook-action)
  (make-hook-action
   "allow" #f #f #f #f))

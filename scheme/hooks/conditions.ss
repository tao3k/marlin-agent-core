;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; POO condition policies for Scheme-owned strategy matching.

(import (only-in :clan/poo/object .def .get .o .slot?)
        (only-in :std/list/list append-map)
        (only-in :yuanxu/poo-clos/syntax .defgeneric .defmethod))

(export hook-condition-kind
        hook-condition-combinator-kind
        HookCondition. HookCombination. evaluate-hook-condition
        make-hook-condition
        all-hook-condition
        any-hook-condition
        not-hook-condition
        hook-condition-from-rule
        hook-condition-match?
        hook-condition-signal-names)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def hook-condition-kind
  "marlin-deck-runtime.condition-policy.v1")

;;; Boundary: Combinator condition policies keep complex logic in Scheme.
;; MarlinResult <- MarlinInput
(def hook-condition-combinator-kind
  "marlin-deck-runtime.condition-combinator.v1")

(.def HookCondition.
  (kind hook-condition-kind)
  (operator 'fields))

(.def HookCombination.
  (kind hook-condition-combinator-kind))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-hook-condition
      required-session-id-value
      required-agent-lineage-values
      required-workspace-state-values
      required-org-memory-hit-values
      required-agent-class-value)
  (.o (:: @ HookCondition.)
      required-session-id: required-session-id-value
      required-agent-lineage: required-agent-lineage-values
      required-workspace-state: required-workspace-state-values
      required-org-memory-hits: required-org-memory-hit-values
      required-agent-class: required-agent-class-value))

;;; Boundary: Composite conditions are POO objects over child conditions.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-condition-combinator operator-value conditions-value)
  (.o (:: @ HookCombination.)
      operator: operator-value
      conditions: conditions-value))

;;; Boundary: All conditions must match for strict policy gates.
;; MarlinResult <- MarlinInput
(def (all-hook-condition conditions)
  (make-marlin-deck-runtime-condition-combinator 'all conditions))

;;; Boundary: Any condition can match for fallback policy gates.
;; MarlinResult <- MarlinInput
(def (any-hook-condition conditions)
  (make-marlin-deck-runtime-condition-combinator 'any conditions))

;;; Boundary: Negated conditions model deny/defer carve-outs.
;; MarlinResult <- MarlinInput
(def (not-hook-condition condition)
  (make-marlin-deck-runtime-condition-combinator 'not (list condition)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (hook-condition-from-rule rule)
  (make-hook-condition
   (.get rule required-session-id)
   (.get rule required-agent-lineage)
   (.get rule required-workspace-state)
   (.get rule required-org-memory-hits)
   (.get rule required-agent-class)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (condition-string-active? value)
  (and value (not (string=? value ""))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (condition-required-string-match? required actual)
  (if (condition-string-active? required)
    (string=? required actual)
    #t))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (condition-string-member? value values)
  (if (member value values) #t #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (condition-all-strings-member? required actual)
  (andmap (cut condition-string-member? <> actual) required))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(.defgeneric evaluate-hook-condition (condition operator context))

(.defmethod (evaluate-hook-condition
             (condition (prototype HookCondition.)) (_operator (eql 'fields)) context)
  (and (condition-required-string-match?
        (.get condition required-session-id)
        (.get context session-id))
       (condition-all-strings-member?
        (.get condition required-agent-lineage)
        (.get context agent-lineage))
       (condition-all-strings-member?
        (.get condition required-workspace-state)
        (.get context workspace-state))
       (condition-all-strings-member?
        (.get condition required-org-memory-hits)
        (.get context org-memory-hits))
       (condition-required-string-match?
        (.get condition required-agent-class)
        (.get context agent-class))))

(.defmethod (evaluate-hook-condition
             (condition (prototype HookCombination.)) (_operator (eql 'all)) context)
  (andmap (cut hook-condition-match? <> context) (.get condition conditions)))

(.defmethod (evaluate-hook-condition
             (condition (prototype HookCombination.)) (_operator (eql 'any)) context)
  (ormap (cut hook-condition-match? <> context) (.get condition conditions)))

(.defmethod (evaluate-hook-condition
             (condition (prototype HookCombination.)) (_operator (eql 'not)) context)
  (not (hook-condition-match? (car (.get condition conditions)) context)))

(.defmethod (evaluate-hook-condition _condition _operator _context) #f)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (hook-condition-match? condition context)
  (and condition (.slot? condition 'operator)
       (evaluate-hook-condition condition (.get condition operator) context)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (hook-condition-signal-names condition)
  (cond
   ((string=? (.get condition kind) hook-condition-kind)
    (append
     (if (condition-string-active? (.get condition required-session-id))
       '("session")
       '())
     (if (null? (.get condition required-agent-lineage))
       '()
       '("agent-lineage"))
     (if (null? (.get condition required-workspace-state))
       '()
       '("workspace-state"))
     (if (null? (.get condition required-org-memory-hits))
       '()
       '("org-memory"))
     (if (condition-string-active? (.get condition required-agent-class))
       '("customer-agent")
       '())))
   ((string=? (.get condition kind) hook-condition-combinator-kind)
    (append-map hook-condition-signal-names
                (.get condition conditions)))
   (else '())))

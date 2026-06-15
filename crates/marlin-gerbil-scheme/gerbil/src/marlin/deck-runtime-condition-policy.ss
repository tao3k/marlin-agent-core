;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; POO condition policies for Scheme-owned strategy matching.

package: marlin

(import (only-in :clan/poo/object .get .o))

(export marlin-deck-runtime-condition-policy-kind
        marlin-deck-runtime-condition-combinator-kind
        make-marlin-deck-runtime-condition-policy
        marlin-deck-runtime-all-condition-policy
        marlin-deck-runtime-any-condition-policy
        marlin-deck-runtime-not-condition-policy
        marlin-deck-runtime-condition-policy-from-rule
        marlin-deck-runtime-condition-policy-match?
        marlin-deck-runtime-condition-policy-signal-names)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-condition-policy-kind
  "marlin-deck-runtime.condition-policy.v1")

;;; Boundary: Combinator condition policies keep complex logic in Scheme.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-condition-combinator-kind
  "marlin-deck-runtime.condition-combinator.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-condition-policy
      required-session-id-value
      required-agent-lineage-values
      required-workspace-state-values
      required-org-memory-hit-values
      required-agent-class-value)
  (.o kind: marlin-deck-runtime-condition-policy-kind
      required-session-id: required-session-id-value
      required-agent-lineage: required-agent-lineage-values
      required-workspace-state: required-workspace-state-values
      required-org-memory-hits: required-org-memory-hit-values
      required-agent-class: required-agent-class-value))

;;; Boundary: Composite conditions are POO objects over child conditions.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-condition-combinator operator-value conditions-value)
  (.o kind: marlin-deck-runtime-condition-combinator-kind
      operator: operator-value
      conditions: conditions-value))

;;; Boundary: All conditions must match for strict policy gates.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-all-condition-policy conditions)
  (make-marlin-deck-runtime-condition-combinator "all" conditions))

;;; Boundary: Any condition can match for fallback policy gates.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-any-condition-policy conditions)
  (make-marlin-deck-runtime-condition-combinator "any" conditions))

;;; Boundary: Negated conditions model deny/defer carve-outs.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-not-condition-policy condition)
  (make-marlin-deck-runtime-condition-combinator "not" (list condition)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-condition-policy-from-rule rule)
  (make-marlin-deck-runtime-condition-policy
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
(def (marlin-deck-runtime-condition-policy-match? condition context)
  (cond
   ((string=? (.get condition kind) marlin-deck-runtime-condition-policy-kind)
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
   ((string=? (.get condition kind) marlin-deck-runtime-condition-combinator-kind)
    (let ((operator (.get condition operator))
          (conditions (.get condition conditions)))
      (cond
       ((string=? operator "all")
        (andmap (lambda (child-condition)
                  (marlin-deck-runtime-condition-policy-match?
                   child-condition
                   context))
                conditions))
       ((string=? operator "any")
        (ormap (lambda (child-condition)
                 (marlin-deck-runtime-condition-policy-match?
                  child-condition
                  context))
               conditions))
       ((string=? operator "not")
        (not
         (marlin-deck-runtime-condition-policy-match?
          (car conditions)
          context)))
       (else #f))))
   (else #f)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-condition-policy-signal-names condition)
  (cond
   ((string=? (.get condition kind) marlin-deck-runtime-condition-policy-kind)
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
   ((string=? (.get condition kind) marlin-deck-runtime-condition-combinator-kind)
    (foldr append
           '()
           (map marlin-deck-runtime-condition-policy-signal-names
                (.get condition conditions))))
   (else '())))

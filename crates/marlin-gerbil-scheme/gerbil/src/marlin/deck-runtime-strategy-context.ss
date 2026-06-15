;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; POO context objects for Scheme-owned Deck runtime strategy decisions.

package: marlin

(import :clan/poo/object)

(export marlin-deck-runtime-strategy-context-kind
        marlin-deck-runtime-strategy-capability-names
        make-marlin-deck-runtime-strategy-context
        strategy-string-active?
        strategy-required-string-match?
        strategy-string-member?
        strategy-all-strings-member?
        marlin-deck-runtime-find-policy-by-name)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-strategy-context-kind
  "marlin-deck-runtime.strategy-context.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-strategy-capability-names)
  '("session-policy"
    "agent-lineage-policy"
    "workspace-state-policy"
    "org-memory-policy"
    "dynamic-hook-action"
    "subagent-policy"
    "customer-agent-policy"
    "high-order-matcher"
    "strategy-template-macro"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-strategy-context
      session-id-value
      agent-lineage-values
      workspace-state-values
      org-memory-hit-values
      agent-class-value)
  (.o kind: marlin-deck-runtime-strategy-context-kind
      session-id: session-id-value
      agent-lineage: agent-lineage-values
      workspace-state: workspace-state-values
      org-memory-hits: org-memory-hit-values
      agent-class: agent-class-value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (strategy-string-active? value)
  (and value (not (string=? value ""))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (strategy-required-string-match? required actual)
  (if (strategy-string-active? required)
    (string=? required actual)
    #t))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (strategy-string-member? value values)
  (if (member value values) #t #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (strategy-all-strings-member? required actual)
  (andmap (cut strategy-string-member? <> actual) required))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-find-policy-by-name policies policy-name)
  (find (lambda (policy)
          (string=? (.get policy name) policy-name))
        policies))

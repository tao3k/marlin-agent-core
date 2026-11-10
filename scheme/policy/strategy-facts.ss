;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Facts supplied to Scheme-owned Deck policy decisions.

(import (only-in :clan/poo/object .get .o))

(export marlin-deck-runtime-strategy-facts-kind
        marlin-deck-runtime-strategy-capability-names
        make-marlin-deck-runtime-strategy-facts
        marlin-deck-runtime-find-policy-by-name)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-strategy-facts-kind
  "marlin-deck-runtime.strategy-facts.v1")

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
(def (make-marlin-deck-runtime-strategy-facts
      session-id-value
      agent-lineage-values
      workspace-state-values
      org-memory-hit-values
      agent-class-value)
  (.o kind: marlin-deck-runtime-strategy-facts-kind
      session-id: session-id-value
      agent-lineage: agent-lineage-values
      workspace-state: workspace-state-values
      org-memory-hits: org-memory-hit-values
      agent-class: agent-class-value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-find-policy-by-name policies policy-name)
  (find (lambda (policy)
          (string=? (.get policy name) policy-name))
        policies))

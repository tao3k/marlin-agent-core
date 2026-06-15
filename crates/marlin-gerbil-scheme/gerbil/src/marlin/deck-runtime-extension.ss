;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; User-facing POO extension prototypes for Deck runtime policy.

package: marlin

(import :clan/poo/object
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-matcher)

(export marlin-deck-runtime-extension-kind
        make-marlin-deck-runtime-extension
        make-marlin-deck-runtime-subagent-policy-extension
        marlin-deck-runtime-extension-match?)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-extension-kind
  "marlin-deck-runtime.extension.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-extension
      extension-id-value
      capability-values
      condition-value
      matcher-value
      dynamic-hook-action-value
      metadata-value)
  (.o kind: marlin-deck-runtime-extension-kind
      id: extension-id-value
      capabilities: capability-values
      condition: condition-value
      matcher: matcher-value
      dynamic-hook-action: dynamic-hook-action-value
      metadata: metadata-value))

;;; Boundary: Upstream maintains extension object prototypes, not module wrappers.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-subagent-policy-extension
      extension-id-value
      subagent-profile-value
      policy-value
      condition-value
      matcher-value
      dynamic-hook-action-value
      metadata-value)
  (.o (:: @ (list (make-marlin-deck-runtime-extension
                   extension-id-value
                   '("subagent-policy"
                     "customer-agent-policy"
                     "dynamic-hook-action"
                     "high-order-matcher")
                   condition-value
                   matcher-value
                   dynamic-hook-action-value
                   metadata-value)))
      subagent-profile: subagent-profile-value
      policy: policy-value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-extension-match?
      extension context policy command agent-scope)
  (and (string=? (.get extension kind) marlin-deck-runtime-extension-kind)
       (marlin-deck-runtime-condition-policy-match?
        (.get extension condition)
        context)
       (marlin-deck-runtime-high-order-matcher-match?
        (.get extension matcher)
        context
        policy
        command
        agent-scope)))

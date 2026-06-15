;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Typed POO receipts produced from user-facing extension prototypes.

package: marlin

(import (only-in :clan/poo/object .get .o)
        :marlin/deck-runtime-extension-catalog
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-strategy-context)

(export marlin-deck-runtime-extension-receipt-kind
        make-marlin-deck-runtime-extension-receipt
        marlin-deck-runtime-extension-action-selection
        marlin-deck-runtime-extension-policy-receipt)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-extension-receipt-kind
  "marlin-deck-runtime.extension-receipt.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-extension-receipt
      matched-value
      extension-id-value
      command-value
      agent-scope-value
      capability-values
      dynamic-hook-action-value
      dynamic-hook-selection-value
      metadata-value)
  (.o kind: marlin-deck-runtime-extension-receipt-kind
      matched: matched-value
      extension-id: extension-id-value
      command: command-value
      agent-scope: agent-scope-value
      capabilities: capability-values
      dynamic-hook-action: dynamic-hook-action-value
      dynamic-hook-selection: dynamic-hook-selection-value
      metadata: metadata-value
      policy-engine: "scheme-poo-extension"))

;;; Boundary: Extension hook decisions use the shared selector receipt path.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-extension-action-selection
      extension context policy command agent-scope)
  (marlin-deck-runtime-dynamic-hook-decision-selection
   (.get extension dynamic-hook-action)
   "extension-action"
   context
   policy
   command
   agent-scope))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-extension-policy-receipt
      catalog context policy command agent-scope)
  (let (extension
        (marlin-deck-runtime-extension-catalog-select
         catalog
         context
         policy
         command
         agent-scope))
    (if extension
      (let* ((hook-selection
              (marlin-deck-runtime-extension-action-selection
               extension
               context
               policy
               command
               agent-scope))
             (hook-action (.get hook-selection dynamic-hook-action)))
        (make-marlin-deck-runtime-extension-receipt
         #t
         (.get extension id)
         command
         agent-scope
         (.get extension capabilities)
         hook-action
         hook-selection
         (.get extension metadata)))
      (make-marlin-deck-runtime-extension-receipt
       #f
       #f
       command
       agent-scope
       '()
       #f
       #f
       #f))))

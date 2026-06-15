;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Macro templates for user-facing POO extension prototypes.

package: marlin

(import :marlin/deck-runtime-extension)

(export defmarlin-deck-runtime-extension)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-extension ()
  ((_ binding
      extension-id
      capabilities
      condition
      matcher
      dynamic-hook-action
      metadata)
   (def binding
     (make-marlin-deck-runtime-extension
      extension-id
      capabilities
      condition
      matcher
      dynamic-hook-action
      metadata))))

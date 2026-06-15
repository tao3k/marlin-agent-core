;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Safe catalog for user-facing POO extension prototypes.

package: marlin

(import :clan/poo/object
        :marlin/deck-runtime-extension
        :marlin/deck-runtime-extension-safety)

(export marlin-deck-runtime-extension-catalog-kind
        make-marlin-deck-runtime-extension-catalog
        marlin-deck-runtime-extension-catalog-add
        marlin-deck-runtime-extension-catalog-find
        marlin-deck-runtime-extension-catalog-select)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-extension-catalog-kind
  "marlin-deck-runtime.extension-catalog.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-extension-catalog
      allowed-hook-id-values
      extension-values)
  (.o kind: marlin-deck-runtime-extension-catalog-kind
      allowed-hook-ids: allowed-hook-id-values
      extensions: extension-values))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-extension-catalog-add catalog extension)
  (if (marlin-deck-runtime-extension-safe?
       extension
       (.get catalog allowed-hook-ids))
    (make-marlin-deck-runtime-extension-catalog
     (.get catalog allowed-hook-ids)
     (append (.get catalog extensions) (list extension)))
    (error "marlin deck runtime extension rejected by safety policy"
           (.get extension id)
           (.get
            (marlin-deck-runtime-validate-extension
             extension
             (.get catalog allowed-hook-ids))
            errors))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-extension-catalog-find catalog extension-id)
  (find (lambda (extension)
          (string=? (.get extension id) extension-id))
        (.get catalog extensions)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-extension-catalog-select
      catalog context policy command agent-scope)
  (find (lambda (extension)
          (marlin-deck-runtime-extension-match?
           extension
           context
           policy
           command
           agent-scope))
        (.get catalog extensions)))

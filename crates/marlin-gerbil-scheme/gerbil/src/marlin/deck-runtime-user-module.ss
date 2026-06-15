;;; -*- Gerbil -*-
;;; Boundary: Module owns NixOS-style user module composition for Gerbil POO extensions.

package: marlin

(import :clan/poo/object
        :marlin/deck-runtime-extension-catalog
        :marlin/deck-runtime-script
        :marlin/deck-runtime-user-option)

(export marlin-deck-runtime-user-module-kind
        marlin-deck-runtime-user-module-evaluation-kind
        make-marlin-deck-runtime-user-module
        defmarlin-deck-runtime-user-module
        marlin-deck-runtime-user-module-merge-module-ids
        marlin-deck-runtime-user-module-merge-export-values
        marlin-deck-runtime-user-module-merge-option-values
        marlin-deck-runtime-user-module-linearize
        marlin-deck-runtime-user-module-evaluate
        marlin-deck-runtime-user-module-extension-catalog
        marlin-deck-runtime-user-module-find-script
        marlin-deck-runtime-user-module-run-script
        marlin-deck-runtime-user-module-script-interface-receipts)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-user-module-kind
  "marlin-deck-runtime.user-module.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-user-module-evaluation-kind
  "marlin-deck-runtime.user-module-evaluation.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-user-module
      module-id-value
      import-values
      extension-values
      script-values
      option-values
      metadata-value)
  (.o (:: @ import-values)
      kind: marlin-deck-runtime-user-module-kind
      id: module-id-value
      imports: import-values
      (module-ids ? '())
      (module-ids => marlin-deck-runtime-user-module-merge-module-ids (list module-id-value))
      (extensions ? '())
      (extensions => marlin-deck-runtime-user-module-merge-export-values extension-values)
      (scripts ? '())
      (scripts => marlin-deck-runtime-user-module-merge-export-values script-values)
      (options ? '())
      (options => marlin-deck-runtime-user-module-merge-option-values option-values)
      metadata: metadata-value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-user-module ()
  ((_ binding
      module-id
      imports
      extensions
      scripts
      options
      metadata)
   (def binding
     (make-marlin-deck-runtime-user-module
      module-id
      imports
      extensions
      scripts
      options
      metadata))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-object-id value)
  (.get value id))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-seen? values id-value)
  (find (lambda (value)
          (string=? (marlin-deck-runtime-user-module-object-id value)
                    id-value))
        values))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-append-new values value)
  (if (marlin-deck-runtime-user-module-seen?
       values
       (marlin-deck-runtime-user-module-object-id value))
    values
    (append values (list value))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-deduplicate values)
  (foldl (lambda (value unique-values)
           (marlin-deck-runtime-user-module-append-new unique-values value))
         '()
         values))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-merge-module-ids
      inherited-module-ids
      direct-module-ids)
  (foldl (lambda (module-id module-ids)
           (if (member module-id module-ids)
             module-ids
             (append module-ids (list module-id))))
         inherited-module-ids
         direct-module-ids))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-merge-export-values
      inherited-export-values
      direct-export-values)
  (marlin-deck-runtime-user-module-deduplicate
   (append direct-export-values inherited-export-values)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-merge-option-values
      inherited-option-values
      direct-option-values)
  (marlin-deck-runtime-user-module-deduplicate
   (append direct-option-values inherited-option-values)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-linearize module)
  (.get module module-ids))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-evaluate module)
  (let* ((linearized-module-id-values (.get module module-ids))
         (collected-extension-values (.get module extensions))
         (collected-script-values (.get module scripts))
         (collected-option-values (.get module options)))
    (.o kind: marlin-deck-runtime-user-module-evaluation-kind
        root-module-id: (.get module id)
        module-ids: linearized-module-id-values
        modules: linearized-module-id-values
        extensions: collected-extension-values
        scripts: collected-script-values
        options: collected-option-values
        metadata: (.get module metadata))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-extension-catalog
      evaluation
      allowed-hook-id-values)
  (foldl (lambda (extension catalog)
           (marlin-deck-runtime-extension-catalog-add catalog extension))
         (make-marlin-deck-runtime-extension-catalog
          allowed-hook-id-values
          '())
         (.get evaluation extensions)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-find-script evaluation script-id)
  (find (lambda (script)
          (string=? (.get script id) script-id))
        (.get evaluation scripts)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-run-script
      evaluation
      script-id
      context)
  (let (script (marlin-deck-runtime-user-module-find-script evaluation script-id))
    (and script
         (marlin-deck-runtime-script-run script context))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-user-module-script-interface-receipts evaluation)
  (map marlin-deck-runtime-script-interface-receipt
       (.get evaluation scripts)))

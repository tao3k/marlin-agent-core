;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Safety checks for user-facing POO extension prototypes.

package: marlin

(import :clan/poo/object
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-extension
        :marlin/deck-runtime-strategy-context)

(export marlin-deck-runtime-extension-safety-report-kind
        make-marlin-deck-runtime-extension-safety-report
        marlin-deck-runtime-extension-safe?
        marlin-deck-runtime-validate-extension)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-extension-safety-report-kind
  "marlin-deck-runtime.extension-safety-report.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-extension-safety-report
      valid-value
      error-values)
  (.o kind: marlin-deck-runtime-extension-safety-report-kind
      valid: valid-value
      errors: error-values))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (extension-string-active? value)
  (and value (not (string=? value ""))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (extension-list-member? value values)
  (if (member value values) #t #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (extension-all-members? values allowed-values)
  (andmap (cut extension-list-member? <> allowed-values) values))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (extension-capabilities-valid? extension)
  (extension-all-members?
   (.get extension capabilities)
   (marlin-deck-runtime-strategy-capability-names)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (extension-action-valid? action)
  (and action
       (string=? (.get action kind) marlin-deck-runtime-dynamic-hook-action-kind)
       (extension-list-member?
        (.get action action)
        (marlin-deck-runtime-dynamic-hook-action-names))))

;;; Boundary: Selectors are safe only when every produced action is safe.
;; MarlinResult <- MarlinInput
(def (extension-selector-actions selector)
  (cons (.get selector default-action)
        (map (lambda (hook-case)
               (.get hook-case dynamic-hook-action))
             (.get selector cases))))

;;; Boundary: Selector validation stays action-shaped, not handler-shaped.
;; MarlinResult <- MarlinInput
(def (extension-selector-valid? selector)
  (and selector
       (string=? (.get selector kind)
                 marlin-deck-runtime-dynamic-hook-selector-kind)
       (andmap extension-action-valid?
               (extension-selector-actions selector))))

;;; Boundary: Extension hook decisions may be direct actions or selectors.
;; MarlinResult <- MarlinInput
(def (extension-hook-decision-valid? hook-decision)
  (or (extension-action-valid? hook-decision)
      (extension-selector-valid? hook-decision)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (extension-catalog-hook-valid? action allowed-hook-ids)
  (let ((action-name (.get action action))
        (hook-id (.get action hook-id)))
    (cond
      ((or (string=? action-name "register")
           (string=? action-name "unregister"))
       (extension-list-member? hook-id allowed-hook-ids))
      (else #t))))

;;; Boundary: Catalog validation descends into selector-produced actions.
;; MarlinResult <- MarlinInput
(def (extension-hook-decision-catalog-valid? hook-decision allowed-hook-ids)
  (cond
    ((extension-action-valid? hook-decision)
     (extension-catalog-hook-valid? hook-decision allowed-hook-ids))
    ((extension-selector-valid? hook-decision)
     (andmap (cut extension-catalog-hook-valid? <> allowed-hook-ids)
             (extension-selector-actions hook-decision)))
    (else #f)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (extension-rewrite-valid? action)
  (if (string=? (.get action action) "rewrite")
    (extension-string-active? (.get action rewrite-command))
    #t))

;;; Boundary: Rewrite validation descends into selector-produced actions.
;; MarlinResult <- MarlinInput
(def (extension-hook-decision-rewrite-valid? hook-decision)
  (cond
    ((extension-action-valid? hook-decision)
     (extension-rewrite-valid? hook-decision))
    ((extension-selector-valid? hook-decision)
     (andmap extension-rewrite-valid?
             (extension-selector-actions hook-decision)))
    (else #f)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (extension-safety-errors extension allowed-hook-ids)
  (let ((hook-decision (.get extension dynamic-hook-action)))
    (append
     (if (string=? (.get extension kind) marlin-deck-runtime-extension-kind)
       '()
       '("extension-kind-invalid"))
     (if (extension-string-active? (.get extension id))
       '()
       '("extension-id-empty"))
     (if (extension-capabilities-valid? extension)
       '()
       '("extension-capability-not-allowed"))
     (if (extension-hook-decision-valid? hook-decision)
       '()
       '("dynamic-hook-action-invalid"))
     (if (extension-hook-decision-valid? hook-decision)
       (if (extension-hook-decision-catalog-valid?
            hook-decision
            allowed-hook-ids)
         '()
         '("dynamic-hook-catalog-id-not-allowed"))
       '())
     (if (extension-hook-decision-valid? hook-decision)
       (if (extension-hook-decision-rewrite-valid? hook-decision)
         '()
         '("rewrite-command-empty"))
       '()))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-validate-extension extension allowed-hook-ids)
  (let (errors (extension-safety-errors extension allowed-hook-ids))
    (make-marlin-deck-runtime-extension-safety-report
     (null? errors)
     errors)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-extension-safe? extension allowed-hook-ids)
  (.get (marlin-deck-runtime-validate-extension extension allowed-hook-ids)
        valid))

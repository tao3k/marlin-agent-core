;;; -*- Gerbil -*-
;;; Boundary: POO policy extension objects authored by user modules.

package: marlin/modules

(import (only-in :clan/poo/object .get .o object?)
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-module-object-has-slot?
                 poo-flow-module-system-owner
                 poo-flow-scheme-owner)
        :marlin/modules/kinds)

(export marlinPolicyExtension
        defmarlin-policy-extension
        marlin-policy-extension?
        marlin-policy-extension-object-count)

;;; Boundary: Policy extensions are POO objects authored by .ss files.
;; MarlinResult <- MarlinInput
(def (marlinPolicyExtension extension-value source-value . maybe-metadata-value)
  (let (metadata-value
        (if (null? maybe-metadata-value)
          '()
          (car maybe-metadata-value)))
    (.o (:: @ (list extension-value))
        policy-extension-kind: marlin-policy-extension-kind
        policy-extension-object: #t
        policy-extension-source: source-value
        policy-extension-managed-by: poo-flow-module-system-owner
        policy-extension-projection-owner: poo-flow-scheme-owner
        policy-extension-runtime-owner: "rust"
        policy-extension-metadata: metadata-value)))

;;; Boundary: Level-1 user API names exported POO extension objects directly.
;; MarlinResult <- MarlinInput
(defrules defmarlin-policy-extension ()
  ((_ binding
      (source source-value)
      (object extension-object)
      (metadata metadata-value))
   (def binding
     (marlinPolicyExtension
      extension-object
      source-value
      metadata-value)))
  ((_ binding
      (source source-value)
      (object extension-object))
   (def binding
     (marlinPolicyExtension
      extension-object
      source-value
      '()))))

;;; Boundary: Predicate identifies module-managed policy extension objects.
;; MarlinResult <- MarlinInput
(def (marlin-policy-extension? value)
  (and (object? value)
       (poo-flow-module-object-has-slot? value 'policy-extension-kind)
       (string=? (.get value policy-extension-kind)
                 marlin-policy-extension-kind)))

;;; Boundary: Receipts count extension objects without inspecting policy internals.
;; MarlinResult <- MarlinInput
(def (marlin-policy-extension-object-count extension-values)
  (length (filter marlin-policy-extension? extension-values)))

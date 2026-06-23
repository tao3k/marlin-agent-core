;;; -*- Gerbil -*-
;;; Boundary: Catalog projection policy objects for prefab packs.

package: marlin/modules

(import (only-in :clan/poo/object .o)
        :marlin/modules/policy-object)

(export marlin-catalog-projection-policy-family
        marlinCatalogProjectionPolicy
        marlinDefaultCatalogProjectionPolicy)

;;; Boundary: Catalog projection policy names Rust-owned handler resolution.
;; String
(def marlin-catalog-projection-policy-family
  "catalog-projection-policy")

;;; Boundary: User-authored .ss files declare catalog projection intent only.
;; PolicyObject <- String POOObject [Metadata]
(def (marlinCatalogProjectionPolicy
      object-id-value
      payload-value
      . maybe-metadata)
  (let (metadata-value
        (if (null? maybe-metadata)
          '((owner . "marlin") (surface . "catalog-projection-policy"))
          (car maybe-metadata)))
    (marlinPolicyObject
     marlin-catalog-projection-policy-family
     object-id-value
     payload-value
     metadata-value)))

;;; Boundary: Furnished projection is a Rust catalog lookup contract.
;; PolicyObject <- Void
(def (marlinDefaultCatalogProjectionPolicy)
  (marlinCatalogProjectionPolicy
   "default-catalog-projection"
   (.o projection-target: "rust-catalog-handlers"
       resolution-owner: "rust")
   '((owner . "marlin") (surface . "default-catalog-projection-policy"))))

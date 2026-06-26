;;; -*- Gerbil -*-
;;; Boundary: Fixture for user-authored .ss policy entrypoint loading.

(import (only-in :clan/poo/object .o)
        :marlin/deck-runtime-policy-receipt-gate-cli
        :config-interface/modules/lib)

;;; Boundary: The loaded file can define POO policy furniture before emission.
;; MarlinResult <- MarlinInput
(def user-authored-debug-route-object
  (marlinPolicyObject
   "model-route-policy"
   "user-authored-debug-route"
   (.o provider: "fixture"
       model: "fixture-model"
       route: "user-authored")
   '((owner . "debug-cli-fixture") (surface . "user-authored-entrypoint"))))

;;; Boundary: Debug CLI calls a user-selected expression after loading this file.
;; Void <- Void
(def (emit-user-authored-policy-receipt-gate-cli-report)
  (emit-policy-receipt-gate-cli-report))

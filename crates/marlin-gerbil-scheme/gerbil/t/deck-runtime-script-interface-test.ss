;;; -*- Gerbil -*-
;;; Boundary: Test owns downstream-facing quick script interface contracts.

(import :clan/poo/object
        :marlin/deck-runtime
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-extension
        :marlin/deck-runtime-matcher
        :marlin/deck-runtime-native-projection
        :marlin/deck-runtime-script
        :std/test)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def ui-script-condition
  (make-marlin-deck-runtime-condition-policy
   "session-ui"
   '("root-agent" "ui-agent")
   '("workspace-clean")
   '("org-memory-ui")
   "customer-ui"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def ui-script-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "ui-command"
   (lambda (context policy command agent-scope)
     (and (string=? (.get policy provider) "openai")
          (string=? command "marlin ui-script apply")
          (string=? agent-scope "ui-agent")))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def ui-script-extension
  (make-marlin-deck-runtime-extension
   "downstream-ui-extension"
   '("dynamic-hook-action" "customer-agent-policy" "high-order-matcher")
   ui-script-condition
   ui-script-matcher
   (make-marlin-deck-runtime-register-hook-action
    "runtime-catalog-ui-hook"
    "runtime-catalog-ui-hook")
   '((owner . "downstream") (surface . "macro"))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-script downstream-ui-script
  "downstream-ui-script"
  ui-script-extension
  "register"
  '((owner . "downstream") (entry . "user-interface"))
  (request)
  (.o command: (.get request command)
      agent-scope: (.get request agent-scope)
      extension-id:
      (.get (marlin-deck-runtime-script-extension downstream-ui-script) id)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-downstream-script-user-interface)
  (let* ((request (.o command: "marlin ui-script apply"
                     agent-scope: "ui-agent"))
         (result (marlin-deck-runtime-script-run downstream-ui-script request))
         (receipt
          (marlin-deck-runtime-script-interface-receipt
           downstream-ui-script))
         (projection (.get receipt native-projection)))
    (check (.get receipt kind)
           => marlin-deck-runtime-script-interface-receipt-kind)
    (check (.get receipt script-id) => "downstream-ui-script")
    (check (.get receipt interface)
           => marlin-deck-runtime-script-interface-kind)
    (check (.get receipt action) => "register")
    (check (.get receipt extension-id) => "downstream-ui-extension")
    (check (.get projection type_id)
           => marlin-deck-runtime-poo-policy-projection-type-id)
    (check (.get projection schema_id)
           => marlin-deck-runtime-poo-policy-projection-schema-id)
    (check (.get projection policy_id) => "downstream-ui-script")
    (check (.get projection action) => "register")
    (check (.get result command) => "marlin ui-script apply")
    (check (.get result agent-scope) => "ui-agent")
    (check (.get result extension-id) => "downstream-ui-extension")))

(check-downstream-script-user-interface)

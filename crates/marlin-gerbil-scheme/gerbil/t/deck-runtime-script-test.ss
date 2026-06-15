;;; -*- Gerbil -*-
;;; Boundary: Test owns quick Gerbil script interfaces for downstream POO extensions.

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
(def script-test-policy
  (make-marlin-deck-runtime-model-route-policy
   "script-policy"
   "openai"
   "gpt-5.4"
   '("codex")
   '("script-agent")
   "shared-context"
   "workspace-isolated"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def script-test-condition
  (make-marlin-deck-runtime-condition-policy
   "session-script"
   '("root-agent" "script-agent")
   '("workspace-clean")
   '("org-memory-script")
   "customer-script"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def script-test-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "script-command"
   (lambda (context policy command agent-scope)
     (and (string=? (.get policy provider) "openai")
          (string=? command "codex script apply")
          (string=? agent-scope "script-agent")))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def script-test-extension
  (make-marlin-deck-runtime-extension
   "customer-script-extension"
   '("dynamic-hook-action" "customer-agent-policy" "high-order-matcher")
   script-test-condition
   script-test-matcher
   (make-marlin-deck-runtime-register-hook-action
    "runtime-catalog-script-hook"
    "runtime-catalog-script-hook")
   '((owner . "customer") (mode . "script"))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-script customer-script
  "customer-script"
  script-test-extension
  "register"
  '((owner . "customer") (entry . "quick-script"))
  (context)
  (list (.get context command)
        (.get context agent-scope)
        (.get (marlin-deck-runtime-script-extension customer-script) id)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-deck-runtime-script)
  (let* ((context (.o command: "codex script apply"
                     agent-scope: "script-agent"))
         (projection (marlin-deck-runtime-script-native-projection customer-script)))
    (check (.get customer-script kind)
           => marlin-deck-runtime-script-kind)
    (check (.get customer-script interface)
           => marlin-deck-runtime-script-interface-kind)
    (check (.get (marlin-deck-runtime-script-extension customer-script) id)
           => "customer-script-extension")
    (check (.get projection type_id)
           => marlin-deck-runtime-poo-policy-projection-type-id)
    (check (.get projection schema_id)
           => marlin-deck-runtime-poo-policy-projection-schema-id)
    (check (.get projection policy_id) => "customer-script")
    (check (.get projection action) => "register")
    (check (marlin-deck-runtime-script-run customer-script context)
           => '("codex script apply"
                "script-agent"
                "customer-script-extension"))))

(check-deck-runtime-script)

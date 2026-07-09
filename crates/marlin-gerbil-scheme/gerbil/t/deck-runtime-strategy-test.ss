;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.

(import :clan/poo/object
        :marlin/deck-runtime
        :marlin/deck-runtime-strategy
        :std/test)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-test-policy)
  (make-marlin-deck-runtime-model-route-policy
   "customer-review-policy"
   "openai"
   "gpt-5.4"
   '("marlin review")
   '("customer-agent")
   "shared-context"
   "workspace-isolated"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-test-context)
  (make-marlin-deck-runtime-strategy-context
   "session-42"
   '("root-agent" "review-agent")
   '("workspace-dirty" "direnv-trusted")
   '("org-memory-hit")
   "customer-reviewer"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-test-matcher)
  (make-marlin-deck-runtime-high-order-matcher
   "customer-review-exact-command"
   (lambda (context policy command agent-scope)
     (and (string=? (.get context session-id) "session-42")
          (string=? (.get policy provider) "openai")
          (string=? command "marlin review src/marlin")
          (string=? agent-scope "customer-agent")))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-agent-policy-template customer-review-template
  "customer-review-template"
  "customer-reviewer"
  "customer-review-policy"
  '("workspace-dirty" "direnv-trusted")
  '("org-memory-hit")
  "rewrite"
  "marlin review --policy scheme-poo src/marlin")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-test-rule)
  (marlin-deck-runtime-agent-policy-template->dynamic-rule
   customer-review-template
   "customer-review-dynamic-hook"
   "session-42"
   '("root-agent" "review-agent")
   (make-test-matcher)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-dynamic-strategy-policy-receipt)
  (let* ((receipt
          (marlin-deck-runtime-dynamic-strategy-policy-receipt
           (list (make-test-policy))
           (list (make-test-rule))
           (make-test-context)
           "marlin review src/marlin"
           "customer-agent"))
         (action (.get receipt dynamic-hook-action)))
    (check (.get receipt kind)
           => marlin-deck-runtime-strategy-policy-receipt-kind)
    (check (.get receipt matched) => #t)
    (check (.get receipt policy-engine) => "scheme-poo")
    (check (.get action kind) => marlin-deck-runtime-dynamic-hook-action-kind)
    (check (.get action action) => "rewrite")
    (check (.get action rewrite-command)
           => "marlin review --policy scheme-poo src/marlin")
    (check (member "scheme-high-order-matcher" (.get receipt matched-signals))
           => '("scheme-high-order-matcher" "dynamic-hook"))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-dynamic-strategy-policy-miss)
  (let ((receipt
         (marlin-deck-runtime-dynamic-strategy-policy-receipt
          (list (make-test-policy))
          (list (make-test-rule))
          (make-test-context)
          "cargo test"
          "customer-agent")))
    (check (.get receipt matched) => #f)
    (check (.get receipt dynamic-hook-action) => #f)
    (check (.get receipt policy-engine) => "scheme-poo")))

(check-dynamic-strategy-policy-receipt)
(check-dynamic-strategy-policy-miss)

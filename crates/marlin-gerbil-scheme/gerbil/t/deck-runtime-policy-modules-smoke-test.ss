;;; -*- Gerbil -*-
;;; Boundary: Fast smoke for Scheme module policy projection into runtime receipts.

(import :clan/poo/object
        :marlin/deck-runtime
        :marlin/deck-runtime-agent-policy
        :marlin/deck-runtime-matcher
        :marlin/deck-runtime-policy-engine
        :marlin/deck-runtime-strategy-context
        :std/test)

;;; Boundary: Smoke policy keeps the user-facing module path in the fast gate.
;; : Object
(def smoke-policy
  (make-marlin-deck-runtime-model-route-policy
   "module-policy"
   "openai"
   "gpt-5.4"
   '("codex")
   '("worker")
   "forked-context"
   "workspace-isolated"))

;;; Boundary: Smoke context exercises the same agent/workspace selectors as the
;;; full policy module integration test without importing the full prefab pack.
;; : Object
(def smoke-context
  (make-marlin-deck-runtime-strategy-context
   "session-module"
   '("root-agent" "worker-agent")
   '("workspace-clean")
   '("org-memory-module")
   "customer-worker"))

;;; Boundary: Matcher remains Scheme-owned policy composition.
;; : Object
(def smoke-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "module-session"
   (lambda (context policy command agent-scope)
     (and (string=? (.get context session-id) "session-module")
          (string=? (.get policy name) "module-policy")
          (string=? command "codex apply")
          (string=? agent-scope "worker")))))

;;; Boundary: Template expands to a typed Rust-facing dynamic strategy rule.
;; : Object
(def smoke-template
  (make-marlin-deck-runtime-agent-policy-template
   "module-template"
   "customer-worker"
   "module-policy"
   '("workspace-clean")
   '("org-memory-module")
   "register"
   #f))

;;; Boundary: Dynamic rule is the native projection target for the smoke chain.
;; : Object
(def smoke-rule
  (marlin-deck-runtime-agent-policy-template->dynamic-rule
   smoke-template
   "module-register-hook"
   "session-module"
   '("root-agent" "worker-agent")
   smoke-matcher))

;;; Boundary: Receipt proves Scheme policy composition projects to Rust-owned
;;; typed runtime evidence.
;; : Object
(def smoke-receipt
  (marlin-deck-runtime-dynamic-strategy-policy-receipt
   (list smoke-policy)
   (list smoke-rule)
   smoke-context
   "codex apply"
   "worker"))

(check (.get smoke-receipt matched) => #t)
(check (.get (.get smoke-receipt dynamic-hook-action) action) => "register")
(check (.get (.get smoke-receipt dynamic-hook-action) registration)
       => "module-register-hook")
(check (.get (.get smoke-receipt dynamic-hook-selection) source)
       => "rule-action")
(check (.get (.get smoke-receipt policy-projection-chain) kind)
       => marlin-deck-runtime-policy-projection-chain-kind)

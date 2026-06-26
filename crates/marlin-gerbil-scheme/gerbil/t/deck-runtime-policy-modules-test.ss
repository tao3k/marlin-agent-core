;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.

(import :clan/poo/object
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-module-system-owner)
        :marlin/deck-runtime
        :marlin/deck-runtime-agent-policy
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-debug-policy-extension
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-extension
        :marlin/deck-runtime-extension-catalog
        :marlin/deck-runtime-extension-receipt
        :marlin/deck-runtime-matcher
        :config-interface/lib
        :marlin/deck-runtime-policy-engine
        :marlin/deck-runtime-strategy-context
        :std/test)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (module-test-policy)
  (make-marlin-deck-runtime-model-route-policy
   "module-policy"
   "openai"
   "gpt-5.4"
   '("codex")
   '("worker")
   "forked-context"
   "workspace-isolated"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (module-test-context)
  (make-marlin-deck-runtime-strategy-context
   "session-module"
   '("root-agent" "worker-agent")
   '("workspace-clean")
   '("org-memory-module")
   "customer-worker"))

;;; Boundary: Alternate workspace state drives deny/defer hook decisions.
;; MarlinResult <- MarlinInput
(def (module-locked-context)
  (make-marlin-deck-runtime-strategy-context
   "session-module"
   '("root-agent" "worker-agent")
   '("workspace-locked")
   '("org-memory-module")
   "customer-worker"))

;;; Boundary: Custom agent context selects different policy templates.
;; MarlinResult <- MarlinInput
(def (module-custom-context)
  (make-marlin-deck-runtime-strategy-context
   "session-module"
   '("root-agent" "custom-agent")
   '("workspace-clean")
   '("org-memory-custom")
   "custom-auditor"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def module-session-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "module-session"
   (lambda (context policy command agent-scope)
     (and (string=? (.get context session-id) "session-module")
          (string=? (.get policy model) "gpt-5.4")
          (string=? command "codex apply")
          (string=? agent-scope "worker")))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def module-agent-template
  (make-marlin-deck-runtime-agent-policy-template
   "module-template"
   "customer-worker"
   "module-policy"
   '("workspace-clean")
   '("org-memory-module")
   "register"
   #f))

;;; Boundary: Custom agent template routes a different agent class.
;; MarlinResult <- MarlinInput
(def module-custom-template
  (make-marlin-deck-runtime-agent-policy-template
   "custom-template"
   "custom-auditor"
   "module-policy"
   '("workspace-clean")
   '("org-memory-custom")
   "rewrite"
   "codex audit --policy custom-auditor"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def module-dynamic-rule
  (marlin-deck-runtime-agent-policy-template->dynamic-rule
   module-agent-template
   "module-register-hook"
   "session-module"
   '("root-agent" "worker-agent")
   module-session-matcher))

;;; Boundary: Macro-generated template sets manage complex Scheme config.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-agent-policy-template-set module-generated-rules
  (module-agent-template module-custom-template)
  "generated"
  "session-module"
  '("root-agent")
  module-session-matcher)

;;; Boundary: Complex condition policy combines session, workspace, and memory.
;; MarlinResult <- MarlinInput
(def module-clean-condition
  (make-marlin-deck-runtime-condition-policy
   "session-module"
   '("root-agent")
   '("workspace-clean")
   '("org-memory-module")
   "customer-worker"))

;;; Boundary: Locked state is used by negative condition and defer action cases.
;; MarlinResult <- MarlinInput
(def module-locked-condition
  (make-marlin-deck-runtime-condition-policy
   "session-module"
   '("root-agent")
   '("workspace-locked")
   '("org-memory-module")
   "customer-worker"))

;;; Boundary: Combined condition models multi-signal gating.
;; MarlinResult <- MarlinInput
(def module-combined-condition
  (marlin-deck-runtime-all-condition-policy
   (list module-clean-condition
         (marlin-deck-runtime-not-condition-policy module-locked-condition))))

;;; Boundary: Command matcher avoids ad hoc TOML-style string matching.
;; MarlinResult <- MarlinInput
(def module-command-matcher
  (marlin-deck-runtime-command-prefix-matcher
   "module-command-prefix"
   '("codex apply" "codex audit")))

;;; Boundary: Customer matcher composes agent class and policy name.
;; MarlinResult <- MarlinInput
(def module-customer-matcher
  (marlin-deck-runtime-and-matcher
   "module-customer-matcher"
   (list
    (marlin-deck-runtime-agent-class-matcher
     "module-agent-class"
     '("customer-worker" "custom-auditor"))
    (marlin-deck-runtime-policy-name-matcher
     "module-policy-name"
     '("module-policy"))
    module-command-matcher)))

;;; Boundary: Selector rule matching stays broad; selector cases decide commands.
;; MarlinResult <- MarlinInput
(def module-selector-matcher
  (marlin-deck-runtime-and-matcher
   "module-selector-matcher"
   (list
    (marlin-deck-runtime-agent-class-matcher
     "module-selector-agent-class"
     '("customer-worker"))
    (marlin-deck-runtime-policy-name-matcher
     "module-selector-policy-name"
     '("module-policy")))))

;;; Boundary: Dynamic hook selector chooses action at runtime.
;; MarlinResult <- MarlinInput
(def module-hook-selector
  (make-marlin-deck-runtime-dynamic-hook-selector
   "module-hook-selector"
   (list
    (make-marlin-deck-runtime-dynamic-hook-case
     "module-register"
     module-combined-condition
     (marlin-deck-runtime-command-prefix-matcher
      "register-prefix"
      '("codex apply"))
     (make-marlin-deck-runtime-register-hook-action
      "module-register-hook"
      "runtime-catalog-entry"))
    (make-marlin-deck-runtime-dynamic-hook-case
     "module-unregister"
     module-combined-condition
     (marlin-deck-runtime-command-prefix-matcher
      "unregister-prefix"
      '("codex cleanup"))
     (make-marlin-deck-runtime-unregister-hook-action
      "module-register-hook"))
    (make-marlin-deck-runtime-dynamic-hook-case
     "module-defer"
     module-locked-condition
     (marlin-deck-runtime-command-prefix-matcher
      "defer-prefix"
      '("codex apply"))
     (make-marlin-deck-runtime-defer-hook-action
      "workspace locked"))
    (make-marlin-deck-runtime-dynamic-hook-case
     "module-deny"
     module-combined-condition
     (marlin-deck-runtime-command-prefix-matcher
      "deny-prefix"
      '("codex deny"))
     (make-marlin-deck-runtime-deny-hook-action
      "policy denied"))
    (make-marlin-deck-runtime-dynamic-hook-case
     "module-rewrite"
     module-combined-condition
     (marlin-deck-runtime-command-prefix-matcher
      "rewrite-prefix"
      '("codex rewrite"))
     (make-marlin-deck-runtime-rewrite-hook-action
      "codex apply --rewritten")))
   (make-marlin-deck-runtime-allow-hook-action)))

;;; Boundary: Policy engine accepts selector objects as dynamic hook decisions.
;; MarlinResult <- MarlinInput
(def module-selector-dynamic-rule
  (make-marlin-deck-runtime-dynamic-strategy-rule
   "module-selector-rule"
   "module-policy"
   "session-module"
   '("root-agent")
   '()
   '("org-memory-module")
   "customer-worker"
   module-selector-matcher
   module-hook-selector))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-context-module)
  (let (context (module-test-context))
    (check (.get context kind) => marlin-deck-runtime-strategy-context-kind)
    (check (.get context agent-class) => "customer-worker")
    (check (strategy-all-strings-member?
            '("workspace-clean")
            (.get context workspace-state))
           => #t)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-condition-policy-module)
  (let ((condition
         (marlin-deck-runtime-condition-policy-from-rule module-dynamic-rule)))
    (check (.get condition kind)
           => marlin-deck-runtime-condition-policy-kind)
    (check (marlin-deck-runtime-condition-policy-match?
            condition
            (module-test-context))
           => #t)
    (check (marlin-deck-runtime-condition-policy-match?
            module-combined-condition
            (module-test-context))
           => #t)
    (check (marlin-deck-runtime-condition-policy-match?
            module-combined-condition
            (module-locked-context))
           => #f)
    (check (marlin-deck-runtime-condition-policy-match?
            (marlin-deck-runtime-any-condition-policy
             (list module-locked-condition module-clean-condition))
            (module-test-context))
           => #t)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-dynamic-hook-module)
  (let* ((context (module-test-context))
         (locked-context (module-locked-context))
         (policy (module-test-policy))
         (register-action
          (make-marlin-deck-runtime-register-hook-action
           "module-register-hook"
           "runtime-catalog-entry"))
         (deny-action
          (make-marlin-deck-runtime-deny-hook-action
           "workspace is locked"))
         (register-selection
          (marlin-deck-runtime-dynamic-hook-selector-select
           module-hook-selector context policy "codex apply file" "worker"))
         (cleanup-selection
          (marlin-deck-runtime-dynamic-hook-selector-select
           module-hook-selector context policy "codex cleanup" "worker"))
         (defer-selection
          (marlin-deck-runtime-dynamic-hook-selector-select
           module-hook-selector locked-context policy "codex apply file" "worker"))
         (deny-selection
          (marlin-deck-runtime-dynamic-hook-selector-select
           module-hook-selector context policy "codex deny" "worker"))
         (rewrite-selection
          (marlin-deck-runtime-dynamic-hook-selector-select
           module-hook-selector context policy "codex rewrite" "worker")))
    (check (.get register-action kind)
           => marlin-deck-runtime-dynamic-hook-action-kind)
    (check (.get register-action action) => "register")
    (check (.get register-action registration) => "runtime-catalog-entry")
    (check (.get deny-action action) => "deny")
    (check (.get deny-action deny-reason) => "workspace is locked")
    (check (.get register-selection action) => "register")
    (check (.get cleanup-selection action) => "unregister")
    (check (.get defer-selection action) => "defer")
    (check (.get deny-selection action) => "deny")
    (check (.get rewrite-selection action) => "rewrite")
    (let ((selection
           (marlin-deck-runtime-dynamic-hook-selector-selection
            module-hook-selector
            context
            policy
            "codex rewrite"
            "worker")))
      (check (.get selection kind)
             => marlin-deck-runtime-dynamic-hook-selection-kind)
      (check (.get selection source) => "selector-case")
      (check (.get selection selector) => "module-hook-selector")
      (check (.get selection matched) => #t)
      (check (.get selection matched-case) => "module-rewrite")
      (check (.get (.get selection dynamic-hook-action) action)
             => "rewrite"))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-matcher-module)
  (let* ((context (module-test-context))
         (policy (module-test-policy))
         (combined
          (marlin-deck-runtime-and-matcher
           "module-combined"
           (list module-session-matcher)))
         (not-command-matcher
          (marlin-deck-runtime-not-matcher
           "not-command"
           module-command-matcher))
         (combined-matched?
          (marlin-deck-runtime-high-order-matcher-match?
           combined context policy "codex apply" "worker"))
         (customer-matched?
          (marlin-deck-runtime-high-order-matcher-match?
           module-customer-matcher context policy "codex apply" "worker"))
         (not-command-matched?
          (marlin-deck-runtime-high-order-matcher-match?
           not-command-matcher context policy "cargo test" "worker")))
    (unless combined-matched?
      (error "expected combined matcher to match"))
    (unless customer-matched?
      (error "expected customer matcher to match"))
    (unless not-command-matched?
      (error "expected negated command matcher to match"))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-agent-policy-module)
  (unless (equal? (.get module-agent-template kind)
                  marlin-deck-runtime-agent-policy-template-kind)
    (error "unexpected agent policy template kind"))
  (unless (equal? (.get module-dynamic-rule kind)
                  marlin-deck-runtime-dynamic-strategy-rule-kind)
    (error "unexpected dynamic rule kind"))
  (unless (equal? (.get module-dynamic-rule required-agent-class)
                  "customer-worker")
    (error "unexpected dynamic rule agent class"))
  (let (selected-template
        (marlin-deck-runtime-agent-policy-template-select
         (list module-agent-template module-custom-template)
         (module-custom-context)))
    (unless (equal? (.get selected-template name) "custom-template")
      (error "unexpected selected agent policy template")))
  (unless (= (length module-generated-rules) 2)
    (error "unexpected generated rule count"))
  (unless (equal? (.get (car module-generated-rules) name)
                  "generated-module-template")
    (error "unexpected generated rule name")))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-policy-engine-module)
  (let* ((receipt
          (marlin-deck-runtime-dynamic-strategy-policy-receipt
           (list (module-test-policy))
           (list module-dynamic-rule)
           (module-test-context)
           "codex apply"
           "worker"))
         (action (.get receipt dynamic-hook-action))
         (selection (.get receipt dynamic-hook-selection))
         (projection-chain (.get receipt policy-projection-chain))
         (module-receipt (.get receipt module-evaluation-receipt))
         (projection-receipt (.get receipt policy-projection-receipt))
         (native-payload (.get receipt native-projection-payload))
         (budget-receipt (.get receipt budget-receipt))
         (catalog-receipt (.get receipt catalog-resolution-receipt)))
    (check (.get receipt kind)
           => marlin-deck-runtime-strategy-policy-receipt-kind)
    (check (.get receipt matched) => #t)
    (check (.get action action) => "register")
    (check (.get action registration) => "module-register-hook")
    (check (.get selection source) => "rule-action")
    (check (.get selection selector) => #f)
    (check (.get projection-chain kind)
           => marlin-deck-runtime-policy-projection-chain-kind)
    (check (.get projection-chain module-evaluation-receipt)
           => module-receipt)
    (check (.get projection-chain policy-projection-receipt)
           => projection-receipt)
    (check (.get projection-chain native-projection-payload)
           => native-payload)
    (check (.get projection-chain budget-receipt)
           => budget-receipt)
    (check (.get projection-chain catalog-resolution-receipt)
           => catalog-receipt)
    (check (.get module-receipt kind)
           => marlin-deck-runtime-module-evaluation-receipt-kind)
    (check (.get module-receipt evaluator) => "poo-flow.scheme")
    (check (.get projection-receipt kind)
           => marlin-deck-runtime-policy-projection-receipt-kind)
    (check (.get projection-receipt projection)
           => "dynamic-strategy-policy")
    (check (.get projection-receipt target-receipt-kind)
           => marlin-deck-runtime-strategy-policy-receipt-kind)
    (check (.get native-payload kind)
           => marlin-deck-runtime-native-projection-payload-kind)
    (check (.get native-payload owner) => "rust")
    (check (.get native-payload action) => "register")
    (check (.get native-payload hook-id) => "module-register-hook")
    (check (.get budget-receipt kind)
           => marlin-deck-runtime-policy-budget-receipt-kind)
    (check (.get budget-receipt budget-owner) => "rust")
    (check (.get budget-receipt scheme-budget-enforced) => #f)
    (check (.get catalog-receipt kind)
           => marlin-deck-runtime-catalog-resolution-receipt-kind)
    (check (.get catalog-receipt catalog-owner) => "rust")
    (check (.get catalog-receipt registration-source)
           => "dynamic-hook-action.registration")
    (check (.get catalog-receipt resolved-by-scheme) => #f)
    (check (.get receipt policy-engine) => "scheme-poo"))
  (let* ((receipt
          (marlin-deck-runtime-dynamic-strategy-policy-receipt
           (list (module-test-policy))
           '()
           (module-test-context)
           "codex apply"
           "worker"))
         (projection-chain (.get receipt policy-projection-chain))
         (module-receipt (.get receipt module-evaluation-receipt))
         (projection-receipt (.get receipt policy-projection-receipt))
         (native-payload (.get receipt native-projection-payload))
         (catalog-receipt (.get receipt catalog-resolution-receipt)))
    (check (.get receipt matched) => #f)
    (check (.get receipt strategy-rule) => #f)
    (check (.get projection-chain kind)
           => marlin-deck-runtime-policy-projection-chain-kind)
    (check (.get module-receipt matched) => #f)
    (check (.get projection-receipt matched) => #f)
    (check (.get native-payload action) => #f)
    (check (.get native-payload hook-id) => #f)
    (check (.get catalog-receipt resolved-by-scheme) => #f)))

;;; Boundary: Policy-engine user flow resolves selector objects to actions.
;; MarlinResult <- MarlinInput
(def (check-policy-engine-selector-module)
  (let* ((receipt
          (marlin-deck-runtime-dynamic-strategy-policy-receipt
           (list (module-test-policy))
           (list module-selector-dynamic-rule)
           (module-test-context)
           "codex rewrite"
           "worker"))
         (action (.get receipt dynamic-hook-action))
         (selection (.get receipt dynamic-hook-selection)))
    (check (.get receipt matched) => #t)
    (check (.get action action) => "rewrite")
    (check (.get selection source) => "selector-case")
    (check (.get selection selector) => "module-hook-selector")
    (check (.get selection matched-case) => "module-rewrite"))
  (let* ((receipt
          (marlin-deck-runtime-dynamic-strategy-policy-receipt
           (list (module-test-policy))
           (list module-selector-dynamic-rule)
           (module-locked-context)
           "codex apply file"
           "worker"))
         (action (.get receipt dynamic-hook-action))
         (selection (.get receipt dynamic-hook-selection)))
    (check (.get receipt matched) => #t)
    (check (.get action action) => "defer")
    (check (.get selection source) => "selector-case")
    (check (.get selection matched-case) => "module-defer"))
  (let* ((receipt
          (marlin-deck-runtime-dynamic-strategy-policy-receipt
           (list (module-test-policy))
           (list module-selector-dynamic-rule)
           (module-test-context)
           "codex status"
           "worker"))
         (action (.get receipt dynamic-hook-action))
         (selection (.get receipt dynamic-hook-selection)))
    (check (.get receipt matched) => #t)
    (check (.get action action) => "allow")
    (check (.get selection source) => "selector-default")
    (check (.get selection matched) => #f)
    (check (.get selection matched-case) => #f)))

;;; Boundary: Debug .ss exports a POO extension object consumed by the Rust CLI.
;; MarlinResult <- MarlinInput
(def (check-debug-policy-extension-module)
  (let* ((extension marlin-deck-runtime-debug-policy-extension)
         (policy-module marlin-deck-runtime-debug-policy-module)
         (module-catalog
          (marlin-deck-runtime-debug-policy-module-catalog))
         (module-evaluation
          (marlin-deck-runtime-debug-policy-module-evaluation))
         (module-presentation
          (marlin-deck-runtime-debug-policy-policy-facade-presentation))
         (policy-workflow
          (marlin-deck-runtime-debug-policy-module-workflow))
         (substrate-gate (.get policy-workflow substrate-gate))
         (catalog (marlin-deck-runtime-debug-policy-extension-catalog))
         (receipt (marlin-deck-runtime-debug-policy-extension-receipt))
         (loop-receipt
          (marlin-deck-runtime-debug-policy-extension-receipt-loop 3)))
    (check (.get extension kind)
           => marlin-deck-runtime-extension-kind)
    (check (.get extension id) => "debug-policy-extension")
    (check (.get extension policy-extension-kind)
           => marlin-policy-extension-kind)
    (check (.get extension policy-extension-object) => #t)
    (check (.get extension policy-extension-source)
           => marlin-deck-runtime-debug-policy-extension-source)
    (check (.get extension policy-extension-managed-by)
           => poo-flow-module-system-owner)
    (check (.get extension policy-extension-projection-owner)
           => "poo-flow.scheme")
    (check (.get extension policy-extension-runtime-owner)
           => "rust")
    (check marlin-deck-runtime-debug-policy-extension-source
           => ":marlin/deck-runtime-debug-policy-extension")
    (check (.get policy-module policy-module-kind)
           => marlin-policy-module-kind)
    (check (.get policy-module id)
           => "debug-policy-extension-module")
    (check (.get policy-module policy-family)
           => "subagent-policy-extension")
    (check (.get module-catalog kind)
           => marlin-module-catalog-kind)
    (check (length (.get module-catalog modules)) => 1)
    (check (.get module-evaluation kind)
           => marlin-eval-modules-result-kind)
    (check (.get module-evaluation workflow-kind)
           => marlin-policy-module-workflow-kind)
    (check (.get module-evaluation projection-target)
           => "extension-policy-receipt")
    (check (.get module-evaluation policy-extension-object-count)
           => 1)
    (check (.get module-presentation kind)
           => marlin-policy-facade-presentation-kind)
    (check (.get module-presentation projection-chain-kind)
           => marlin-module-projection-chain-kind)
    (check (.get module-presentation root-extension-count) => 1)
    (check (.get module-presentation root-policy-extension-object-count)
           => 1)
    (check (.get module-presentation import-graph-owner)
           => poo-flow-module-system-owner)
    (check (.get module-presentation option-policy-owner)
           => poo-flow-module-system-owner)
    (check (.get module-presentation native-projection-payload-owner)
           => "rust")
    (check (.get module-presentation rust-parses-scheme-source)
           => #f)
    (check (.get module-presentation scheme-manufactures-rust-handlers)
           => #f)
    (check (.get substrate-gate kind)
           => marlin-policy-substrate-gate-kind)
    (check (.get substrate-gate projection-target)
           => "extension-policy-receipt")
    (check (.get substrate-gate receipt-kind)
           => marlin-deck-runtime-extension-receipt-kind)
    (check (.get substrate-gate module-evaluation-kind)
           => "poo-flow.modules.runtime-evaluation.v1")
    (check (.get substrate-gate extension-count) => 1)
    (check (.get substrate-gate policy-extension-object-count) => 1)
    (check (.get substrate-gate script-count) => 0)
    (check (.get substrate-gate option-count) => 2)
    (check (.get substrate-gate validation-receipt-count) => 2)
    (check (.get substrate-gate replayable) => #t)
    (check (.get catalog kind)
           => marlin-deck-runtime-extension-catalog-kind)
    (check (.get receipt matched) => #t)
    (check (.get receipt extension-id) => "debug-policy-extension")
    (check (.get (.get receipt dynamic-hook-action) action)
           => "register")
    (check (.get (.get receipt dynamic-hook-action) hook-id)
           => "debug-runtime-catalog-hook")
    (check (.get loop-receipt matched) => #t)
    (check (.get loop-receipt extension-id) => "debug-policy-extension")))

(check-context-module)
(check-condition-policy-module)
(check-dynamic-hook-module)
(check-matcher-module)
(check-agent-policy-module)
(check-policy-engine-module)
(check-policy-engine-selector-module)
(check-debug-policy-extension-module)

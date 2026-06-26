;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Scheme-owned POO policy engine for Deck runtime strategy decisions.

package: marlin

(import (only-in :clan/poo/object .get .o)
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-scheme-owner)
        :marlin/deck-runtime
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-matcher
        :marlin/deck-runtime-strategy-context)

(export marlin-deck-runtime-strategy-rule-kind
        marlin-deck-runtime-strategy-selection-kind
        marlin-deck-runtime-dynamic-strategy-rule-kind
        marlin-deck-runtime-strategy-policy-receipt-kind
        marlin-deck-runtime-policy-projection-chain-kind
        marlin-deck-runtime-module-evaluation-receipt-kind
        marlin-deck-runtime-policy-projection-receipt-kind
        marlin-deck-runtime-native-projection-payload-kind
        marlin-deck-runtime-policy-budget-receipt-kind
        marlin-deck-runtime-catalog-resolution-receipt-kind
        make-marlin-deck-runtime-strategy-rule
        make-marlin-deck-runtime-dynamic-strategy-rule
        defmarlin-deck-runtime-strategy-rule
        marlin-deck-runtime-strategy-rule-match?
        marlin-deck-runtime-dynamic-strategy-rule-match?
        marlin-deck-runtime-select-model-route-policy/strategy
        marlin-deck-runtime-select-dynamic-strategy-rule
        marlin-deck-runtime-strategy-rule-signal-names
        marlin-deck-runtime-dynamic-strategy-rule-signal-names
        marlin-deck-runtime-dynamic-strategy-rule-action-selection
        marlin-deck-runtime-dynamic-strategy-rule-action
        marlin-deck-runtime-strategy-selection
        marlin-deck-runtime-dynamic-strategy-policy-receipt)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-strategy-rule-kind
  "marlin-deck-runtime.strategy-rule.v1")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-strategy-selection-kind
  "marlin-deck-runtime.strategy-selection.v1")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-dynamic-strategy-rule-kind
  "marlin-deck-runtime.dynamic-strategy-rule.v1")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-strategy-policy-receipt-kind
  "marlin-deck-runtime.strategy-policy-receipt.v1")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-policy-projection-chain-kind
  "marlin-deck-runtime.policy-projection-chain.v1")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-module-evaluation-receipt-kind
  "marlin-deck-runtime.module-evaluation-receipt.v1")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-policy-projection-receipt-kind
  "marlin-deck-runtime.policy-projection-receipt.v1")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-native-projection-payload-kind
  "marlin-deck-runtime.native-projection-payload.v1")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-policy-budget-receipt-kind
  "marlin-deck-runtime.policy-budget-receipt.v1")
;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-catalog-resolution-receipt-kind
  "marlin-deck-runtime.catalog-resolution-receipt.v1")

;;; Boundary: Scheme reports module evaluation without owning runtime transport.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-module-evaluation-receipt matched-value)
  (.o kind: marlin-deck-runtime-module-evaluation-receipt-kind
      module: ":marlin/deck-runtime-policy-engine"
      evaluator: poo-flow-scheme-owner
      matched: matched-value
      imports: '(":marlin/deck-runtime"
                 ":marlin/deck-runtime-condition-policy"
                 ":marlin/deck-runtime-dynamic-hook"
                 ":marlin/deck-runtime-matcher"
                 ":marlin/deck-runtime-strategy-context")))

;;; Boundary: Scheme exposes the policy projection shape, not Rust handlers.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-policy-projection-receipt
      matched-value
      strategy-rule-value)
  (.o kind: marlin-deck-runtime-policy-projection-receipt-kind
      projection: "dynamic-strategy-policy"
      source-module: ":marlin/deck-runtime-policy-engine"
      target-receipt-kind: marlin-deck-runtime-strategy-policy-receipt-kind
      matched: matched-value
      strategy-rule: strategy-rule-value
      policy-engine: "scheme-poo"))

;;; Boundary: Projection helpers avoid treating #f as a POO object.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-hook-action-name hook-action-value)
  (if hook-action-value (.get hook-action-value action) #f))
(def (marlin-deck-runtime-dynamic-hook-action-hook-id hook-action-value)
  (if hook-action-value (.get hook-action-value hook-id) #f))

;;; Boundary: Native payload is data for Rust validation, not Scheme transport.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-native-projection-payload
      dynamic-hook-action-value
      dynamic-hook-selection-value)
  (.o kind: marlin-deck-runtime-native-projection-payload-kind
      owner: "rust"
      payload: "dynamic-hook-action"
      action: (marlin-deck-runtime-dynamic-hook-action-name dynamic-hook-action-value)
      hook-id: (marlin-deck-runtime-dynamic-hook-action-hook-id dynamic-hook-action-value)
      dynamic-hook-selection: dynamic-hook-selection-value))

;;; Boundary: Runtime budgets remain Rust-owned.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-policy-budget-receipt)
  (.o kind: marlin-deck-runtime-policy-budget-receipt-kind
      budget-owner: "rust"
      scheme-budget-enforced: #f))

;;; Boundary: Catalog resolution records Rust ownership of handler lookup.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-catalog-resolution-receipt
      dynamic-hook-action-value)
  (.o kind: marlin-deck-runtime-catalog-resolution-receipt-kind
      catalog-owner: "rust"
      action: (marlin-deck-runtime-dynamic-hook-action-name dynamic-hook-action-value)
      hook-id: (marlin-deck-runtime-dynamic-hook-action-hook-id dynamic-hook-action-value)
      registration-source: "dynamic-hook-action.registration"
      resolved-by-scheme: #f))

;;; Boundary: Fixed chain pattern for Scheme policy projection receipts.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-policy-projection-chain
      matched-value
      strategy-rule-value
      dynamic-hook-action-value
      dynamic-hook-selection-value)
  (.o kind: marlin-deck-runtime-policy-projection-chain-kind
      module-evaluation-receipt:
      (make-marlin-deck-runtime-module-evaluation-receipt matched-value)
      policy-projection-receipt:
      (make-marlin-deck-runtime-policy-projection-receipt
       matched-value
       strategy-rule-value)
      native-projection-payload:
      (make-marlin-deck-runtime-native-projection-payload
       dynamic-hook-action-value
       dynamic-hook-selection-value)
      budget-receipt: (make-marlin-deck-runtime-policy-budget-receipt)
      catalog-resolution-receipt:
      (make-marlin-deck-runtime-catalog-resolution-receipt
       dynamic-hook-action-value)))

;;; Boundary: Strategy policy receipts expose a stable projection chain.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-strategy-policy-receipt
      matched-value
      command-value
      agent-scope-value
      strategy-rule-value
      model-policy-value
      dynamic-hook-action-value
      dynamic-hook-selection-value
      matched-signals-value)
  (let* ((module-evaluation-receipt
          (make-marlin-deck-runtime-module-evaluation-receipt matched-value))
         (policy-projection-receipt
          (make-marlin-deck-runtime-policy-projection-receipt
           matched-value
           strategy-rule-value))
         (native-projection-payload
          (make-marlin-deck-runtime-native-projection-payload
           dynamic-hook-action-value
           dynamic-hook-selection-value))
         (budget-receipt
          (make-marlin-deck-runtime-policy-budget-receipt))
         (catalog-resolution-receipt
          (make-marlin-deck-runtime-catalog-resolution-receipt
           dynamic-hook-action-value))
         (projection-chain
          (.o kind: marlin-deck-runtime-policy-projection-chain-kind
              module-evaluation-receipt: module-evaluation-receipt
              policy-projection-receipt: policy-projection-receipt
              native-projection-payload: native-projection-payload
              budget-receipt: budget-receipt
              catalog-resolution-receipt: catalog-resolution-receipt)))
    (.o kind: marlin-deck-runtime-strategy-policy-receipt-kind
        matched: matched-value
        command: command-value
        agent-scope: agent-scope-value
        strategy-rule: strategy-rule-value
        model-policy: model-policy-value
        dynamic-hook-action: dynamic-hook-action-value
        dynamic-hook-selection: dynamic-hook-selection-value
        policy-projection-chain: projection-chain
        module-evaluation-receipt: module-evaluation-receipt
        policy-projection-receipt: policy-projection-receipt
        native-projection-payload: native-projection-payload
        budget-receipt: budget-receipt
        catalog-resolution-receipt: catalog-resolution-receipt
        matched-signals: matched-signals-value
        capabilities: (marlin-deck-runtime-strategy-capability-names)
        policy-engine: "scheme-poo")))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-strategy-rule
      rule-name-value
      policy-name-value
      required-session-id-value
      required-agent-lineage-values
      required-workspace-state-values
      required-org-memory-hit-values
      required-agent-class-value
      hook-action-value
      rewrite-command-value)
  (.o kind: marlin-deck-runtime-strategy-rule-kind
      name: rule-name-value
      policy-name: policy-name-value
      required-session-id: required-session-id-value
      required-agent-lineage: required-agent-lineage-values
      required-workspace-state: required-workspace-state-values
      required-org-memory-hits: required-org-memory-hit-values
      required-agent-class: required-agent-class-value
      hook-action: hook-action-value
      rewrite-command: rewrite-command-value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-dynamic-strategy-rule
      rule-name-value
      policy-name-value
      required-session-id-value
      required-agent-lineage-values
      required-workspace-state-values
      required-org-memory-hit-values
      required-agent-class-value
      high-order-matcher-value
      dynamic-hook-action-value)
  (.o kind: marlin-deck-runtime-dynamic-strategy-rule-kind
      name: rule-name-value
      policy-name: policy-name-value
      required-session-id: required-session-id-value
      required-agent-lineage: required-agent-lineage-values
      required-workspace-state: required-workspace-state-values
      required-org-memory-hits: required-org-memory-hit-values
      required-agent-class: required-agent-class-value
      high-order-matcher: high-order-matcher-value
      dynamic-hook-action: dynamic-hook-action-value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-strategy-rule ()
  ((_ binding
      rule-name
      policy-name
      required-session-id
      required-agent-lineage
      required-workspace-state
      required-org-memory-hits
      required-agent-class
      hook-action
      rewrite-command)
   (def binding
     (make-marlin-deck-runtime-strategy-rule
      rule-name
      policy-name
      required-session-id
      required-agent-lineage
      required-workspace-state
      required-org-memory-hits
      required-agent-class
      hook-action
      rewrite-command))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-strategy-rule-match?
      rule context policy command agent-scope)
  (and policy
       (string=? (.get rule kind) marlin-deck-runtime-strategy-rule-kind)
       (marlin-deck-runtime-route-policy-match? policy command agent-scope)
       (marlin-deck-runtime-condition-policy-match?
        (marlin-deck-runtime-condition-policy-from-rule rule)
        context)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-strategy-rule-match?
      rule context policy command agent-scope)
  (and policy
       (string=? (.get rule kind) marlin-deck-runtime-dynamic-strategy-rule-kind)
       (marlin-deck-runtime-route-policy-match? policy command agent-scope)
       (marlin-deck-runtime-condition-policy-match?
        (marlin-deck-runtime-condition-policy-from-rule rule)
        context)
       (marlin-deck-runtime-high-order-matcher-match?
        (.get rule high-order-matcher)
        context
        policy
        command
        agent-scope)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-select-model-route-policy/strategy
      policies rules context command agent-scope)
  (let (matches
        (filter-map
         (lambda (rule)
           (let (policy
                 (marlin-deck-runtime-find-policy-by-name
                  policies
                  (.get rule policy-name)))
             (and (marlin-deck-runtime-strategy-rule-match?
                   rule context policy command agent-scope)
                  (.o selected-rule: rule selected-policy: policy))))
         rules))
    (and (pair? matches) (car matches))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-select-dynamic-strategy-rule
      policies rules context command agent-scope)
  (let (matches
        (filter-map
         (lambda (rule)
           (let (policy
                 (marlin-deck-runtime-find-policy-by-name
                  policies
                  (.get rule policy-name)))
             (and (marlin-deck-runtime-dynamic-strategy-rule-match?
                   rule context policy command agent-scope)
                  (.o selected-rule: rule selected-policy: policy))))
         rules))
    (and (pair? matches) (car matches))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-strategy-rule-signal-names rule)
  (append
   '("model-route" "command-prefix" "agent-scope" "high-order-matcher")
   (marlin-deck-runtime-condition-policy-signal-names
    (marlin-deck-runtime-condition-policy-from-rule rule))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-strategy-rule-signal-names rule)
  (append
   (marlin-deck-runtime-strategy-rule-signal-names rule)
   (if (.get rule high-order-matcher)
     '("scheme-high-order-matcher")
     '())
   '("dynamic-hook")))

;;; Boundary: Strategy rules may carry a direct action or Scheme selector.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-strategy-rule-action-selection
      rule context policy command agent-scope)
  (marlin-deck-runtime-dynamic-hook-decision-selection
   (.get rule dynamic-hook-action)
   "rule-action"
   context
   policy
   command
   agent-scope))

;;; Boundary: Rust projection receives the final action, not selector internals.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-strategy-rule-action
      rule context policy command agent-scope)
  (.get (marlin-deck-runtime-dynamic-strategy-rule-action-selection
         rule
         context
         policy
         command
         agent-scope)
        dynamic-hook-action))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-strategy-selection
      policies rules context command agent-scope)
  (let ((selection
         (marlin-deck-runtime-select-model-route-policy/strategy
          policies rules context command agent-scope)))
    (if selection
      (let ((matched-rule (.get selection selected-rule))
            (matched-policy (.get selection selected-policy)))
        (.o kind: marlin-deck-runtime-strategy-selection-kind
            command: command
            agent-scope: agent-scope
            matched: #t
            strategy-rule: (.get matched-rule name)
            hook-action: (.get matched-rule hook-action)
            rewrite-command: (.get matched-rule rewrite-command)
            matched-signals: (marlin-deck-runtime-strategy-rule-signal-names matched-rule)
            capabilities: (marlin-deck-runtime-strategy-capability-names)
            policy: matched-policy))
      (.o kind: marlin-deck-runtime-strategy-selection-kind
          command: command
          agent-scope: agent-scope
          matched: #f
          strategy-rule: #f
          hook-action: #f
          rewrite-command: #f
          matched-signals: '()
          capabilities: (marlin-deck-runtime-strategy-capability-names)
          policy: #f))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-dynamic-strategy-policy-receipt
      policies rules context command agent-scope)
  (let ((selection
         (marlin-deck-runtime-select-dynamic-strategy-rule
          policies rules context command agent-scope)))
    (if selection
      (let ((matched-rule (.get selection selected-rule))
            (matched-policy (.get selection selected-policy)))
        (let* ((hook-selection
                (marlin-deck-runtime-dynamic-strategy-rule-action-selection
                 matched-rule
                 context
                 matched-policy
                 command
                 agent-scope))
               (hook-action (.get hook-selection dynamic-hook-action)))
          (make-marlin-deck-runtime-strategy-policy-receipt
           #t
           command
           agent-scope
           (.get matched-rule name)
           matched-policy
           hook-action
           hook-selection
           (marlin-deck-runtime-dynamic-strategy-rule-signal-names matched-rule))))
      (make-marlin-deck-runtime-strategy-policy-receipt
       #f
       command
       agent-scope
       #f
       #f
       #f
       #f
       '()))))

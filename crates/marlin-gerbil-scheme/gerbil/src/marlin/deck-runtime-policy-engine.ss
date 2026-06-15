;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Scheme-owned POO policy engine for Deck runtime strategy decisions.

package: marlin

(import (only-in :clan/poo/object .get .o)
        :marlin/deck-runtime
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-matcher
        :marlin/deck-runtime-strategy-context)

(export marlin-deck-runtime-strategy-rule-kind
        marlin-deck-runtime-strategy-selection-kind
        marlin-deck-runtime-dynamic-strategy-rule-kind
        marlin-deck-runtime-strategy-policy-receipt-kind
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
        (.o kind: marlin-deck-runtime-strategy-policy-receipt-kind
            matched: #t
            command: command
            agent-scope: agent-scope
            strategy-rule: (.get matched-rule name)
            model-policy: matched-policy
            dynamic-hook-action: hook-action
            dynamic-hook-selection: hook-selection
            matched-signals: (marlin-deck-runtime-dynamic-strategy-rule-signal-names matched-rule)
            capabilities: (marlin-deck-runtime-strategy-capability-names)
            policy-engine: "scheme-poo")))
      (.o kind: marlin-deck-runtime-strategy-policy-receipt-kind
          matched: #f
          command: command
          agent-scope: agent-scope
          strategy-rule: #f
          model-policy: #f
          dynamic-hook-action: #f
          dynamic-hook-selection: #f
          matched-signals: '()
          capabilities: (marlin-deck-runtime-strategy-capability-names)
          policy-engine: "scheme-poo"))))

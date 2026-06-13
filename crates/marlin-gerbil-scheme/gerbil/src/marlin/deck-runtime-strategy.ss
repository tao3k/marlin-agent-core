;;; -*- Gerbil -*-
;;; Complex strategy plane for Deck runtime policy decisions.

package: marlin

(import :clan/poo/object
        :marlin/deck-runtime)

(export marlin-deck-runtime-strategy-rule-kind
        marlin-deck-runtime-strategy-context-kind
        marlin-deck-runtime-strategy-selection-kind
        marlin-deck-runtime-strategy-capability-names
        make-marlin-deck-runtime-strategy-context
        make-marlin-deck-runtime-strategy-rule
        defmarlin-deck-runtime-strategy-rule
        marlin-deck-runtime-strategy-rule-match?
        marlin-deck-runtime-select-model-route-policy/strategy
        marlin-deck-runtime-strategy-rule-signal-names
        marlin-deck-runtime-strategy-selection)

(def marlin-deck-runtime-strategy-rule-kind
  "marlin-deck-runtime.strategy-rule.v1")
(def marlin-deck-runtime-strategy-context-kind
  "marlin-deck-runtime.strategy-context.v1")
(def marlin-deck-runtime-strategy-selection-kind
  "marlin-deck-runtime.strategy-selection.v1")

(def (marlin-deck-runtime-strategy-capability-names)
  '("session-policy"
    "agent-lineage-policy"
    "workspace-state-policy"
    "org-memory-policy"
    "dynamic-hook-action"
    "customer-agent-policy"
    "high-order-matcher"
    "strategy-template-macro"))

(def (make-marlin-deck-runtime-strategy-context
      session-id-value
      agent-lineage-values
      workspace-state-values
      org-memory-hit-values
      agent-class-value)
  (.o kind: marlin-deck-runtime-strategy-context-kind
      session-id: session-id-value
      agent-lineage: agent-lineage-values
      workspace-state: workspace-state-values
      org-memory-hits: org-memory-hit-values
      agent-class: agent-class-value))

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

(def (strategy-string-active? value)
  (and value (not (string=? value ""))))

(def (strategy-required-string-match? required actual)
  (if (strategy-string-active? required)
    (string=? required actual)
    #t))

(def (strategy-string-member? value values)
  (if (member value values) #t #f))

(def (strategy-all-strings-member? required actual)
  (let loop ((remaining required))
    (cond
      ((null? remaining) #t)
      ((strategy-string-member? (car remaining) actual) (loop (cdr remaining)))
      (else #f))))

(def (marlin-deck-runtime-find-policy-by-name policies policy-name)
  (let loop ((remaining policies))
    (cond
      ((null? remaining) #f)
      ((string=? (.get (car remaining) name) policy-name) (car remaining))
      (else (loop (cdr remaining))))))

(def (marlin-deck-runtime-strategy-rule-match?
      rule context policy command agent-scope)
  (and policy
       (string=? (.get rule kind) marlin-deck-runtime-strategy-rule-kind)
       (marlin-deck-runtime-route-policy-match? policy command agent-scope)
       (strategy-required-string-match?
        (.get rule required-session-id)
        (.get context session-id))
       (strategy-all-strings-member?
        (.get rule required-agent-lineage)
        (.get context agent-lineage))
       (strategy-all-strings-member?
        (.get rule required-workspace-state)
        (.get context workspace-state))
       (strategy-all-strings-member?
        (.get rule required-org-memory-hits)
        (.get context org-memory-hits))
       (strategy-required-string-match?
        (.get rule required-agent-class)
        (.get context agent-class))))

(def (marlin-deck-runtime-select-model-route-policy/strategy
      policies rules context command agent-scope)
  (let loop ((remaining rules))
    (cond
      ((null? remaining) #f)
      (else
       (let ((rule (car remaining)))
         (let ((policy
                (marlin-deck-runtime-find-policy-by-name
                 policies
                 (.get rule policy-name))))
           (if (marlin-deck-runtime-strategy-rule-match?
                rule context policy command agent-scope)
             (.o selected-rule: rule selected-policy: policy)
             (loop (cdr remaining)))))))))

(def (marlin-deck-runtime-strategy-rule-signal-names rule)
  (append
   '("model-route" "command-prefix" "agent-scope" "high-order-matcher")
   (if (strategy-string-active? (.get rule required-session-id))
     '("session")
     '())
   (if (null? (.get rule required-agent-lineage))
     '()
     '("agent-lineage"))
   (if (null? (.get rule required-workspace-state))
     '()
     '("workspace-state"))
   (if (null? (.get rule required-org-memory-hits))
     '()
     '("org-memory"))
   (if (strategy-string-active? (.get rule required-agent-class))
     '("customer-agent")
     '())))

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

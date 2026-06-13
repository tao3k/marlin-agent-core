;;; -*- Gerbil -*-
;;; Sample complex hook policies for the Marlin Gerbil extension plane.

package: marlin/hooks

(import :std/text/json)

(export marlin-hook-policy-sample-capability-names
        decide-hook-policy-sample)

(defrules defmarlin-hook-policy-action-template ()
  ((_ binding kind target replacement reason)
   (def binding
     (string-append
      "{\"kind\":\"" kind "\""
      (if target (string-append ",\"target\":\"" target "\"") "")
      (if replacement (string-append ",\"replacement\":\"" replacement "\"") "")
      (if reason (string-append ",\"reason\":\"" reason "\"") "")
      "}"))))

(defmarlin-hook-policy-action-template register-customer-agent-hook-action
  "Register"
  "catalog:customer-agent-hook"
  #f
  "customer agent session requires runtime catalog hook")

(defmarlin-hook-policy-action-template defer-release-session-action
  "Defer"
  "session:release"
  #f
  "release lineage waits for org memory review")

(defmarlin-hook-policy-action-template deny-dangerous-shell-action
  "Deny"
  "dangerous-shell"
  #f
  "dirty workspace blocks dangerous shell hook")

(defmarlin-hook-policy-action-template rewrite-locked-test-action
  "Rewrite"
  "command"
  "cargo test --locked"
  "session policy prefers locked tests")

(def (marlin-hook-policy-sample-capability-names)
  '("session-policy"
    "agent-lineage-policy"
    "workspace-state-policy"
    "org-memory-policy"
    "customer-agent-policy"
    "runtime-catalog-register"
    "dynamic-defer-deny-rewrite"))

(def (sample-field object field default)
  (hash-ref object field default))

(def (sample-object-field object field)
  (and object (sample-field object field #f)))

(def (sample-decision-context request)
  (sample-object-field
   (sample-object-field request "policy_receipt")
   "decision_context"))

(def (sample-context-field request field default)
  (let ((context (sample-decision-context request)))
    (if context
      (sample-field context field default)
      default)))

(def (sample-string-value value default)
  (if (string? value) value default))

(def (sample-list-value value default)
  (if (list? value) value default))

(def (sample-context-string-field request field default)
  (sample-string-value
   (sample-context-field request field default)
   default))

(def (sample-context-list-field request field default)
  (sample-list-value
   (sample-context-field request field default)
   default))

(def (sample-list-member? value values)
  (let loop ((remaining values))
    (cond
     ((null? remaining) #f)
     ((and (string? (car remaining))
           (string=? value (car remaining)))
      #t)
     (else (loop (cdr remaining))))))

(def (sample-json-array items)
  (string-append
   "["
   (let loop ((remaining items) (first? #t) (acc ""))
     (cond
      ((null? remaining) acc)
      (first? (loop (cdr remaining) #f (string-append acc (car remaining))))
      (else (loop (cdr remaining) #f (string-append acc "," (car remaining))))))
   "]"))

(def (sample-actions request)
  (let ((agent-scope (sample-string-value (sample-field request "agent_scope" "Any") "Any"))
        (session-id (sample-context-string-field request "session_id" ""))
        (agent-lineage (sample-context-list-field request "agent_lineage" '()))
        (workspace-state (sample-context-list-field request "workspace_state" '()))
        (org-memory-hits (sample-context-list-field request "org_memory_hits" '()))
        (agent-class (sample-context-string-field request "agent_class" "")))
    (let loop ((candidates
                (list
                 (if (or (string=? agent-scope "CustomerAgent")
                         (string=? agent-class "customer-agent"))
                   register-customer-agent-hook-action
                   #f)
                 (if (and (sample-list-member? "release" agent-lineage)
                          (sample-list-member? "needs-human-review" org-memory-hits))
                   defer-release-session-action
                   #f)
                 (if (sample-list-member? "dirty" workspace-state)
                   deny-dangerous-shell-action
                   #f)
                 (if (string=? session-id "cheap-test-session")
                   rewrite-locked-test-action
                   #f)))
               (actions '()))
      (cond
       ((null? candidates) (reverse actions))
       ((car candidates) (loop (cdr candidates) (cons (car candidates) actions)))
       (else (loop (cdr candidates) actions))))))

(def (decide-hook-policy-sample request-json)
  (let* ((request (string->json-object request-json))
         (actions (sample-actions request)))
    (string-append
     "{\"decision\":\"Allowed\","
     "\"diagnostics\":[{\"message\":\"sample Gerbil hook policy evaluated\"}],"
     "\"actions\":"
     (sample-json-array actions)
     "}")))

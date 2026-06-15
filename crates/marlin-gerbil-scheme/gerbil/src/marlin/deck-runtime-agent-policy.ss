;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Customer/custom agent policy templates for Scheme-owned POO policy.

package: marlin

(import (only-in :clan/poo/object .get .o)
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-policy-engine)

(export marlin-deck-runtime-agent-policy-template-kind
        make-marlin-deck-runtime-agent-policy-template
        defmarlin-deck-runtime-agent-policy-template
        defmarlin-deck-runtime-agent-policy-template-set
        marlin-deck-runtime-agent-policy-template-select
        marlin-deck-runtime-agent-policy-templates->dynamic-rules
        marlin-deck-runtime-agent-policy-template->dynamic-rule)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-agent-policy-template-kind
  "marlin-deck-runtime.agent-policy-template.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-agent-policy-template
      template-name-value
      agent-class-value
      policy-name-value
      workspace-state-values
      org-memory-hit-values
      hook-action-value
      rewrite-command-value)
  (.o kind: marlin-deck-runtime-agent-policy-template-kind
      name: template-name-value
      agent-class: agent-class-value
      policy-name: policy-name-value
      workspace-state: workspace-state-values
      org-memory-hits: org-memory-hit-values
      hook-action: hook-action-value
      rewrite-command: rewrite-command-value))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-agent-policy-template ()
  ((_ binding
      template-name
      agent-class
      policy-name
      workspace-state
      org-memory-hits
      hook-action
      rewrite-command)
   (def binding
     (make-marlin-deck-runtime-agent-policy-template
      template-name
      agent-class
      policy-name
      workspace-state
      org-memory-hits
      hook-action
      rewrite-command))))

;;; Boundary: Agent class selection keeps custom/customer routing in Scheme.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-agent-policy-template-select templates context)
  (find (lambda (template)
          (string=? (.get template agent-class)
                    (.get context agent-class)))
        templates))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-template-action->dynamic-hook
      template rule-name)
  (make-marlin-deck-runtime-dynamic-hook-action
   (.get template hook-action)
   rule-name
   rule-name
   #f
   #f
   (.get template rewrite-command)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-agent-policy-template->dynamic-rule
      template rule-name session-id agent-lineage matcher)
  (make-marlin-deck-runtime-dynamic-strategy-rule
   rule-name
   (.get template policy-name)
   session-id
   agent-lineage
   (.get template workspace-state)
   (.get template org-memory-hits)
   (.get template agent-class)
   matcher
   (marlin-deck-runtime-template-action->dynamic-hook template rule-name)))

;;; Boundary: Template sets generate dynamic rules without Rust-side DSLs.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-agent-policy-templates->dynamic-rules
      templates rule-prefix session-id agent-lineage matcher)
  (map (lambda (template)
         (marlin-deck-runtime-agent-policy-template->dynamic-rule
          template
          (string-append rule-prefix "-" (.get template name))
          session-id
          agent-lineage
          matcher))
       templates))

;;; Boundary: Macro-managed template sets stay in the Scheme extension plane.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-agent-policy-template-set ()
  ((_ binding
      (template ...)
      rule-prefix
      session-id
      agent-lineage
      matcher)
   (def binding
     (marlin-deck-runtime-agent-policy-templates->dynamic-rules
      (list template ...)
      rule-prefix
      session-id
      agent-lineage
      matcher))))

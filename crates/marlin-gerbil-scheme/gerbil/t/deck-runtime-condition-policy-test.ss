;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.

(import :clan/poo/object
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-strategy-context
        :std/test)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def matching-context
  (make-marlin-deck-runtime-strategy-context
   "session-condition"
   '("root-agent" "worker-agent")
   '("workspace-dirty" "direnv-trusted")
   '("org-memory-hit" "task-memory-hit")
   "customer-worker"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def strict-condition
  (make-marlin-deck-runtime-condition-policy
   "session-condition"
   '("root-agent" "worker-agent")
   '("workspace-dirty")
   '("org-memory-hit")
   "customer-worker"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def missing-lineage-condition
  (make-marlin-deck-runtime-condition-policy
   "session-condition"
   '("missing-agent")
   '("workspace-dirty")
   '("org-memory-hit")
   "customer-worker"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def open-condition
  (make-marlin-deck-runtime-condition-policy
   #f
   '()
   '()
   '()
   #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-condition-policy-match)
  (check (.get strict-condition kind)
         => marlin-deck-runtime-condition-policy-kind)
  (check (marlin-deck-runtime-condition-policy-match?
          strict-condition
          matching-context)
         => #t)
  (check (marlin-deck-runtime-condition-policy-match?
          missing-lineage-condition
          matching-context)
         => #f)
  (check (marlin-deck-runtime-condition-policy-match?
          open-condition
          matching-context)
         => #t))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-condition-policy-signals)
  (check (marlin-deck-runtime-condition-policy-signal-names strict-condition)
         => '("session"
              "agent-lineage"
              "workspace-state"
              "org-memory"
              "customer-agent"))
  (check (marlin-deck-runtime-condition-policy-signal-names open-condition)
         => '()))

(check-condition-policy-match)
(check-condition-policy-signals)

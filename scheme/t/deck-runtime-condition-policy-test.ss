;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.

(import :clan/poo/object
        "../hooks/conditions.ss"
        "../policy/strategy-facts.ss"
        :std/test)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def matching-context
  (make-marlin-deck-runtime-strategy-facts
   "session-condition"
   '("root-agent" "worker-agent")
   '("workspace-dirty" "direnv-trusted")
   '("org-memory-hit" "task-memory-hit")
   "customer-worker"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def strict-condition
  (make-hook-condition
   "session-condition"
   '("root-agent" "worker-agent")
   '("workspace-dirty")
   '("org-memory-hit")
   "customer-worker"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def missing-lineage-condition
  (make-hook-condition
   "session-condition"
   '("missing-agent")
   '("workspace-dirty")
   '("org-memory-hit")
   "customer-worker"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def open-condition
  (make-hook-condition
   #f
   '()
   '()
   '()
   #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-condition-policy-match)
  (check (.get strict-condition kind)
         => hook-condition-kind)
  (check (hook-condition-match?
          strict-condition
          matching-context)
         => #t)
  (check (hook-condition-match?
          missing-lineage-condition
          matching-context)
         => #f)
  (check (hook-condition-match?
          open-condition
          matching-context)
         => #t))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (check-condition-policy-signals)
  (check (hook-condition-signal-names strict-condition)
         => '("session"
              "agent-lineage"
              "workspace-state"
              "org-memory"
              "customer-agent"))
  (check (hook-condition-signal-names open-condition)
         => '()))

(check-condition-policy-match)
(check-condition-policy-signals)

(check (hook-condition-match?
        (all-hook-condition (list strict-condition (not-hook-condition missing-lineage-condition)))
        matching-context)
       => #t)
(check (hook-condition-match?
        (any-hook-condition (list missing-lineage-condition strict-condition))
        matching-context)
       => #t)

;;; -*- Gerbil -*-
;;; Boundary: Package-visible helpers for deck runtime script performance gates.

package: marlin

(import :clan/poo/object
        :marlin/deck-runtime-condition-policy
        :marlin/deck-runtime-dynamic-hook
        :marlin/deck-runtime-extension
        :marlin/deck-runtime-matcher
        :marlin/deck-runtime-script)

(export deck-runtime-script-performance-context
        deck-runtime-script-performance-count-runs
        deck-runtime-script-performance-run-batch)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (deck-runtime-script-performance-context)
  (.o command: "marlin perf-script apply"
      agent-scope: "performance-agent"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def performance-script-condition
  (make-marlin-deck-runtime-condition-policy
   "session-performance"
   '("root-agent" "performance-agent")
   '("workspace-clean")
   '()
   "customer-performance"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def performance-script-matcher
  (make-marlin-deck-runtime-high-order-matcher
   "performance-command"
   (lambda (_context _policy command agent-scope)
     (and (string=? command "marlin perf-script apply")
          (string=? agent-scope "performance-agent")))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def performance-script-extension
  (make-marlin-deck-runtime-extension
   "performance-script-extension"
   '("dynamic-hook-action" "high-order-matcher")
   performance-script-condition
   performance-script-matcher
   (make-marlin-deck-runtime-allow-hook-action)
   '((owner . "bench") (surface . "script"))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-script performance-script
  "performance-script"
  performance-script-extension
  "allow"
  '((owner . "bench") (entry . "batch"))
  (context)
  (string-append (.get context command)
                 " :: "
                 (.get context agent-scope)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (deck-runtime-script-performance-run-batch iterations context)
  (marlin-deck-runtime-script-batch-metrics
   performance-script
   iterations
   context))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (deck-runtime-script-performance-count-runs iterations context)
  (count-marlin-deck-runtime-script-runs
   performance-script
   iterations
   context))

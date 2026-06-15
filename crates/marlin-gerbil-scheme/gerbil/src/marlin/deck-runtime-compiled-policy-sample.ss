;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Package-compiled sample policy template for Deck runtime selector benchmarks.

package: marlin

(import :marlin/deck-runtime
        :marlin/deck-runtime-compiled-policy)

(export select-marlin-deck-runtime-sample-compiled-policy
        select-marlin-deck-runtime-sample-compiled-policy-index
        marlin-deck-runtime-sample-compiled-policy-match?
        marlin-deck-runtime-sample-compiled-policy-index-match?
        count-marlin-deck-runtime-sample-compiled-policy-matches
        count-marlin-deck-runtime-sample-compiled-policy-index-matches
        display-marlin-deck-runtime-sample-compiled-policy-batch-metrics
        display-marlin-deck-runtime-sample-compiled-policy-index-batch-metrics)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-cached-compiled-route-selector
  select-marlin-deck-runtime-sample-compiled-policy
  ("compiled-cheap-test-runner"
   "openai"
   "gpt-5-mini"
   ("cargo test" "just test")
   ("sub-agent" "hook")
   "forked-context"
   "workspace-isolated")
  ("compiled-deep-reviewer"
   "anthropic"
   "claude-opus-4-8"
   ("codex customer-review" "cargo clippy")
   ("reviewer")
   "shared-context"
   "isolated-session"))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defmarlin-deck-runtime-direct-compiled-route-index-selector
  select-marlin-deck-runtime-sample-compiled-policy-index
  (0
   ("cargo test" "just test")
   ("sub-agent" "hook"))
  (1
   ("codex customer-review" "cargo clippy")
   ("reviewer")))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-sample-compiled-policy-match? command agent-scope)
  (if (select-marlin-deck-runtime-sample-compiled-policy command agent-scope) #t #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-sample-compiled-policy-index-match? command agent-scope)
  (if (select-marlin-deck-runtime-sample-compiled-policy-index command agent-scope) #t #f))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (count-marlin-deck-runtime-sample-compiled-policy-matches iterations command agent-scope)
  (foldl (lambda (_ matches)
           (if (marlin-deck-runtime-sample-compiled-policy-match? command agent-scope)
             (+ matches 1)
             matches))
         0
         (list-tabulate iterations identity)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (count-marlin-deck-runtime-sample-compiled-policy-index-matches iterations command agent-scope)
  (foldl (lambda (_ matches)
           (if (marlin-deck-runtime-sample-compiled-policy-index-match? command agent-scope)
             (+ matches 1)
             matches))
         0
         (list-tabulate iterations identity)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-sample-compiled-policy-elapsed-us start-jiffy end-jiffy)
  (quotient (* (- end-jiffy start-jiffy) 1000000) (jiffies-per-second)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (display-marlin-deck-runtime-sample-compiled-policy-batch-metrics iterations command agent-scope)
  (let ((start-jiffy (current-jiffy)))
    (let ((matches
           (count-marlin-deck-runtime-sample-compiled-policy-matches
            iterations
            command
            agent-scope)))
      (let ((end-jiffy (current-jiffy)))
        (display "compiled-policy-template-batch ")
        (display marlin-deck-runtime-compiled-policy-kind)
        (display " iterations=")
        (display iterations)
        (display " matches=")
        (display matches)
        (display " policy_elapsed_us=")
        (display
         (marlin-deck-runtime-sample-compiled-policy-elapsed-us
          start-jiffy
          end-jiffy))
        (newline)))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (display-marlin-deck-runtime-sample-compiled-policy-index-batch-metrics iterations command agent-scope)
  (let ((start-jiffy (current-jiffy)))
    (let ((matches
           (count-marlin-deck-runtime-sample-compiled-policy-index-matches
            iterations
            command
            agent-scope)))
      (let ((end-jiffy (current-jiffy)))
        (display "compiled-policy-template-index-batch ")
        (display marlin-deck-runtime-compiled-policy-kind)
        (display " iterations=")
        (display iterations)
        (display " matches=")
        (display matches)
        (display " index_elapsed_us=")
        (display
         (marlin-deck-runtime-sample-compiled-policy-elapsed-us
          start-jiffy
          end-jiffy))
        (newline)))))

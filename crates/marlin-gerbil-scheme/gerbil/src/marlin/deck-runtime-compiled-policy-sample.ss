;;; -*- Gerbil -*-
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

(defmarlin-deck-runtime-direct-compiled-route-index-selector
  select-marlin-deck-runtime-sample-compiled-policy-index
  (0
   ("cargo test" "just test")
   ("sub-agent" "hook"))
  (1
   ("codex customer-review" "cargo clippy")
   ("reviewer")))

(def (marlin-deck-runtime-sample-compiled-policy-match? command agent-scope)
  (if (select-marlin-deck-runtime-sample-compiled-policy command agent-scope) #t #f))

(def (marlin-deck-runtime-sample-compiled-policy-index-match? command agent-scope)
  (if (select-marlin-deck-runtime-sample-compiled-policy-index command agent-scope) #t #f))

(def (count-marlin-deck-runtime-sample-compiled-policy-matches iterations command agent-scope)
  (let loop ((remaining iterations) (matches 0))
    (if (= remaining 0)
      matches
      (loop
       (- remaining 1)
       (if (marlin-deck-runtime-sample-compiled-policy-match? command agent-scope)
         (+ matches 1)
         matches)))))

(def (count-marlin-deck-runtime-sample-compiled-policy-index-matches iterations command agent-scope)
  (let loop ((remaining iterations) (matches 0))
    (if (= remaining 0)
      matches
      (loop
       (- remaining 1)
       (if (marlin-deck-runtime-sample-compiled-policy-index-match? command agent-scope)
         (+ matches 1)
         matches)))))

(def (marlin-deck-runtime-sample-compiled-policy-elapsed-us start-jiffy end-jiffy)
  (quotient (* (- end-jiffy start-jiffy) 1000000) (jiffies-per-second)))

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

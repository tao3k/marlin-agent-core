;;; -*- Gerbil -*-
;;; Package-compiled sample policy template for Deck runtime selector benchmarks.

package: marlin

(import :marlin/deck-runtime
        :marlin/deck-runtime-compiled-policy)

(export select-marlin-deck-runtime-sample-compiled-policy
        marlin-deck-runtime-sample-compiled-policy-match?
        count-marlin-deck-runtime-sample-compiled-policy-matches
        display-marlin-deck-runtime-sample-compiled-policy-selection-json
        display-marlin-deck-runtime-sample-compiled-policy-batch-metrics)

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

(def (marlin-deck-runtime-sample-compiled-policy-match? command agent-scope)
  (if (select-marlin-deck-runtime-sample-compiled-policy command agent-scope) #t #f))

(def (count-marlin-deck-runtime-sample-compiled-policy-matches iterations command agent-scope)
  (let loop ((remaining iterations) (matches 0))
    (if (= remaining 0)
      matches
      (loop
       (- remaining 1)
       (if (marlin-deck-runtime-sample-compiled-policy-match? command agent-scope)
         (+ matches 1)
         matches)))))

(def (display-marlin-deck-runtime-sample-compiled-policy-selection-json command agent-scope)
  (display-marlin-deck-runtime-compiled-selection-json
   select-marlin-deck-runtime-sample-compiled-policy
   command
   agent-scope))

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
        (display " elapsed_us=")
        (display
         (marlin-deck-runtime-sample-compiled-policy-elapsed-us
          start-jiffy
          end-jiffy))
        (newline)))))

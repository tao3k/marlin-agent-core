;;; -*- Gerbil -*-
;;; Boundary: Config-interface policy pack compiler receipts execute in real gxi.

(import (only-in :clan/poo/object .get)
        :config-interface/lib)

;;; Boundary: Local scalar assertions avoid extra std/test runner behavior.
;; MarlinResult <- MarlinInput
(defrules check ()
  ((_ expression => expected)
   (let ((actual-value expression)
         (expected-value expected))
     (unless (equal? actual-value expected-value)
       (error "check failed" actual-value expected-value)))))

(def failure-retry-compiler-receipt
  (marlinFailureRetryLoopProgramCompilerReceipt))

(def failure-retry-policy-pack
  (.get failure-retry-compiler-receipt resolved-policy-pack))

(def failure-retry-hot-pack
  (.get failure-retry-policy-pack hot))

(def failure-retry-budget-caps
  (.get failure-retry-hot-pack budget_caps))

(def failure-retry-continuation-table
  (.get failure-retry-hot-pack continuation_table))

(def failure-retry-loop-program
  (.get failure-retry-compiler-receipt loop-program))

(check (.get failure-retry-compiler-receipt kind)
       => marlin-poo-loop-program-compiler-receipt-kind)
(check (.get failure-retry-compiler-receipt profile-id)
       => "marlin-failure-retry-profile/typed-recovery")
(check (.get failure-retry-compiler-receipt scheme-boundary)
       => "scheme-types-to-rust-types")
(check (.get failure-retry-compiler-receipt serialization-boundary)
       => "rust-owned-cli-trace-cross-process")
(check (.get failure-retry-policy-pack policy_epoch) => 21)
(check (.get failure-retry-budget-caps max_attempts) => 3)
(check (.get (vector-ref failure-retry-continuation-table 0) op) => "retry")
(check (.get (vector-ref failure-retry-continuation-table 0) graph_template)
       => 1)
(check (.get (vector-ref failure-retry-continuation-table 0) max_attempts)
       => 3)
(check (.get failure-retry-loop-program program_id)
       => "failure-retry-typed-recovery")
(check (vector-length (.get failure-retry-loop-program transitions)) => 5)

(display "config-interface-loop-policy-pack-ok")
(newline)

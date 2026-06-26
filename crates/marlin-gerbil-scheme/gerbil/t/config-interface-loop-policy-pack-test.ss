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

(def real-repair-compiler-receipt
  (marlinRealRepair001LoopProgramCompilerReceipt))

(def real-repair-policy-pack
  (.get real-repair-compiler-receipt resolved-policy-pack))

(def real-repair-loop-program
  (.get real-repair-compiler-receipt loop-program))

(def policy-combination-compiler-receipt
  (marlinPolicyCombinationMatrixLoopProgramCompilerReceipt))

(def policy-combination-policy-pack
  (.get policy-combination-compiler-receipt resolved-policy-pack))

(def policy-combination-hot-pack
  (.get policy-combination-policy-pack hot))

(def policy-combination-audit-pack
  (.get policy-combination-policy-pack audit))

(def policy-combination-loop-program
  (.get policy-combination-compiler-receipt loop-program))

(check (.get failure-retry-compiler-receipt kind)
       => marlin-poo-loop-program-compiler-receipt-kind)
(check (.get failure-retry-compiler-receipt profile-id)
       => "marlin-failure-retry-profile/typed-recovery")
(check (.get failure-retry-compiler-receipt compiler-owner)
       => "gerbil-poo-flow")
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

(check (.get real-repair-compiler-receipt kind)
       => marlin-poo-loop-program-compiler-receipt-kind)
(check (.get real-repair-compiler-receipt profile-id)
       => "real-repair-001/reactive-tool-loop")
(check (.get real-repair-compiler-receipt compiler-owner)
       => "gerbil-poo-flow")
(check (.get real-repair-compiler-receipt scheme-boundary)
       => "scheme-types-to-rust-types")
(check (.get real-repair-compiler-receipt serialization-boundary)
       => "rust-owned-cli-trace-cross-process")
(check (.get real-repair-policy-pack policy_epoch) => 42)
(check (.get real-repair-loop-program program_id)
       => "real-repair-001-scripted-loop")
(check (vector-length (.get real-repair-loop-program transitions)) => 6)

(check (.get policy-combination-compiler-receipt kind)
       => marlin-poo-loop-program-compiler-receipt-kind)
(check (.get policy-combination-compiler-receipt profile-id)
       => "policy-combination/memory-rewrite-checker")
(check (.get policy-combination-compiler-receipt compiler-owner)
       => "gerbil-poo-flow")
(check (.get policy-combination-compiler-receipt scheme-boundary)
       => "scheme-types-to-rust-types")
(check (.get policy-combination-compiler-receipt serialization-boundary)
       => "rust-owned-cli-trace-cross-process")
(check (.get policy-combination-policy-pack policy_epoch) => 15)
(check (vector-length (.get policy-combination-hot-pack graph_nodes)) => 3)
(check (vector-length (.get policy-combination-audit-pack linearization)) => 5)
(check (.get policy-combination-loop-program program_id)
       => "policy-combination-memory-rewrite-checker")
(check (vector-length (.get policy-combination-loop-program mechanism_policies))
       => 3)
(check (vector-length (.get policy-combination-loop-program transitions))
       => 6)
(check (.get (vector-ref (.get policy-combination-loop-program transitions) 0)
             action)
       => "read_memory")
(check (.get (vector-ref (.get policy-combination-loop-program transitions) 5)
             action)
       => "stop")

(display "config-interface-loop-policy-pack-ok")
(newline)

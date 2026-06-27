;;; -*- Gerbil -*-
;;; Boundary: Smoke test for Scheme-owned case driver receipts.

(import (only-in "../src/config-interface/custom/marline-kernel/case-driver"
                 +marline-kernel-loop-case-driver-receipt-kind+
                 marline-kernel-loop-case-driver-receipts))

;;; Boundary: Local checks stay scalar around typed Scheme receipt alists.
;; MarlinResult <- MarlinInput
(defrules check ()
  ((_ expression => expected)
   (let ((actual-value expression)
         (expected-value expected))
     (unless (equal? actual-value expected-value)
       (error "check failed" actual-value expected-value)))))

(def (alist-ref values key)
  (let (value (assq key values))
    (and value (cdr value))))

(def case-driver-receipts
  (marline-kernel-loop-case-driver-receipts))

(def runtime-handoff-receipt
  (car case-driver-receipts))

(check (length case-driver-receipts) => 4)
(check (map (lambda (receipt) (alist-ref receipt 'kind))
            case-driver-receipts)
       => (list +marline-kernel-loop-case-driver-receipt-kind+
                +marline-kernel-loop-case-driver-receipt-kind+
                +marline-kernel-loop-case-driver-receipt-kind+
                +marline-kernel-loop-case-driver-receipt-kind+))
(check (map (lambda (receipt) (alist-ref receipt 'case-id))
            case-driver-receipts)
       => '(marlin-runtime-handoff-real-llm
            marlin-policy-receipt-gate-real-llm
            marlin-loop-contract-real-llm
            marlin-failure-retry-real-llm))
(check (map (lambda (receipt) (alist-ref receipt 'profile-ref))
            case-driver-receipts)
       => '(marlin-runtime-handoff-profile
            marlin-policy-receipt-gate-profile
            marlin-loop-contract-profile
            marlin-failure-retry-profile))
(check (map (lambda (receipt) (alist-ref receipt 'runtime-mode))
            case-driver-receipts)
       => '(real-llm-opt-in real-llm-opt-in real-llm-opt-in real-llm-opt-in))
(check (map (lambda (receipt) (alist-ref receipt 'live-gate-env))
            case-driver-receipts)
       => '("MARLIN_RUN_REAL_LLM_CASES"
            "MARLIN_RUN_REAL_LLM_CASES"
            "MARLIN_RUN_REAL_LLM_CASES"
            "MARLIN_RUN_REAL_LLM_CASES"))
(check (map (lambda (receipt) (alist-ref receipt 'live-enabled?))
            case-driver-receipts)
       => '(#f #f #f #f))
(check (map (lambda (receipt) (alist-ref receipt 'smoke-status))
            case-driver-receipts)
       => '(no-live-llm-denied
            no-live-llm-denied
            no-live-llm-denied
            no-live-llm-denied))
(check (map (lambda (receipt) (alist-ref receipt 'command-kind))
            case-driver-receipts)
       => '(loop-program-run loop-run loop-run loop-run))
(check (map (lambda (receipt) (alist-ref receipt 'stable-fixture?))
            case-driver-receipts)
       => '(#t #t #t #t))
(check (map (lambda (receipt) (alist-ref receipt 'module-kind))
            case-driver-receipts)
       => '("poo-flow.modules.user-selection.v1"
            "poo-flow.modules.user-selection.v1"
            "poo-flow.modules.user-selection.v1"
            "poo-flow.modules.user-selection.v1"))
(check (map (lambda (receipt) (alist-ref receipt 'module-user-module))
            case-driver-receipts)
       => '(funflow funflow funflow funflow))
(check (map (lambda (receipt) (alist-ref receipt 'module-selection-tags))
            case-driver-receipts)
       => '((+functional +dag +typed-receipts +runtime-manifest)
            (+functional +dag +typed-receipts +runtime-manifest)
            (+functional +dag +typed-receipts +runtime-manifest)
            (+functional +dag +typed-receipts +runtime-manifest)))
(check (map (lambda (receipt) (alist-ref receipt 'module-source-ref))
            case-driver-receipts)
       => '(none none none none))
(check (map (lambda (receipt) (alist-ref receipt 'module-entrypoint))
            case-driver-receipts)
       => '(none none none none))
(check (map (lambda (receipt) (alist-ref receipt 'module-enabled?))
            case-driver-receipts)
       => '(#t #t #t #t))
(check (alist-ref runtime-handoff-receipt 'input-path)
       => "t/fixtures/config-interface/loop-cases/runtime-handoff-llm.loop-program-run.json")
(check (alist-ref runtime-handoff-receipt 'runtime-execution-owner)
       => "marlin-agent-core")
(check (alist-ref runtime-handoff-receipt 'scheme-boundary)
       => 'scheme-types->rust-types)
(check (alist-ref runtime-handoff-receipt 'serialization-boundary)
       => 'rust-owned-cli-trace-cross-process)

(display "config-interface-case-driver-ok")
(newline)
(display "case-driver-receipts=")
(display (length case-driver-receipts))
(newline)

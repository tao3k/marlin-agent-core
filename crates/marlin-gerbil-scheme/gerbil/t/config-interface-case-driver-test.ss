;;; -*- Gerbil -*-
;;; Boundary: Smoke test for Scheme-owned case driver receipts.

(import (only-in "../src/config-interface/custom/marline-kernel/case-driver"
                 +marline-kernel-loop-case-driver-receipt-kind+
                 marline-kernel-loop-case-driver-module-receipts
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

(def (positive-number? value)
  (and (number? value) (> value 0)))

(def case-driver-receipts
  (marline-kernel-loop-case-driver-receipts))

(def module-case-driver-receipts
  (marline-kernel-loop-case-driver-module-receipts))

(def vertical-case-driver-receipts
  (filter (lambda (receipt)
            (alist-ref receipt 'vertical-mainline?))
          case-driver-receipts))

(def runtime-handoff-receipt
  (car module-case-driver-receipts))

(check (length case-driver-receipts) => 11)
(check (length module-case-driver-receipts) => 4)
(check (length vertical-case-driver-receipts) => 7)
(check (map (lambda (receipt) (alist-ref receipt 'kind))
            case-driver-receipts)
       => (make-list 11 +marline-kernel-loop-case-driver-receipt-kind+))
(check (map (lambda (receipt) (alist-ref receipt 'case-id))
            module-case-driver-receipts)
       => '(marlin-runtime-handoff-real-llm
            marlin-policy-receipt-gate-real-llm
            marlin-loop-contract-real-llm
            marlin-failure-retry-real-llm))
(check (map (lambda (receipt) (alist-ref receipt 'profile-ref))
            module-case-driver-receipts)
       => '(marlin-runtime-handoff-profile
            marlin-policy-receipt-gate-profile
            marlin-loop-contract-profile
            marlin-failure-retry-profile))
(check (map (lambda (receipt) (alist-ref receipt 'runtime-mode))
            module-case-driver-receipts)
       => '(real-llm-opt-in real-llm-opt-in real-llm-opt-in real-llm-opt-in))
(check (map (lambda (receipt) (alist-ref receipt 'live-gate-env))
            module-case-driver-receipts)
       => '("MARLIN_RUN_REAL_LLM_CASES"
            "MARLIN_RUN_REAL_LLM_CASES"
            "MARLIN_RUN_REAL_LLM_CASES"
            "MARLIN_RUN_REAL_LLM_CASES"))
(check (map (lambda (receipt) (alist-ref receipt 'live-enabled?))
            module-case-driver-receipts)
       => '(#f #f #f #f))
(check (map (lambda (receipt) (alist-ref receipt 'smoke-status))
            module-case-driver-receipts)
       => '(no-live-llm-denied
            no-live-llm-denied
            no-live-llm-denied
            no-live-llm-denied))
(check (map (lambda (receipt) (alist-ref receipt 'command-kind))
            module-case-driver-receipts)
       => '(loop-program-run loop-run loop-run loop-run))
(check (map (lambda (receipt) (alist-ref receipt 'stable-fixture?))
            module-case-driver-receipts)
       => '(#t #t #t #t))
(check (map (lambda (receipt) (alist-ref receipt 'module-kind))
            module-case-driver-receipts)
       => '("poo-flow.modules.user-selection.v1"
            "poo-flow.modules.user-selection.v1"
            "poo-flow.modules.user-selection.v1"
            "poo-flow.modules.user-selection.v1"))
(check (map (lambda (receipt) (alist-ref receipt 'module-user-module))
            module-case-driver-receipts)
       => '(funflow funflow funflow funflow))
(check (map (lambda (receipt) (alist-ref receipt 'module-selection-tags))
            module-case-driver-receipts)
       => '((+functional +dag +typed-receipts +runtime-manifest)
            (+functional +dag +typed-receipts +runtime-manifest)
            (+functional +dag +typed-receipts +runtime-manifest)
            (+functional +dag +typed-receipts +runtime-manifest)))
(check (map (lambda (receipt) (alist-ref receipt 'module-source-ref))
            module-case-driver-receipts)
       => '(none none none none))
(check (map (lambda (receipt) (alist-ref receipt 'module-entrypoint))
            module-case-driver-receipts)
       => '(none none none none))
(check (map (lambda (receipt) (alist-ref receipt 'module-enabled?))
            module-case-driver-receipts)
       => '(#t #t #t #t))
(check (alist-ref runtime-handoff-receipt 'input-path)
       => "t/fixtures/config-interface/loop-cases/runtime-handoff-llm.loop-program-run.json")
(check (alist-ref runtime-handoff-receipt 'runtime-execution-owner)
       => "marlin-agent-core")
(check (alist-ref runtime-handoff-receipt 'scheme-boundary)
       => 'scheme-types->rust-types)
(check (alist-ref runtime-handoff-receipt 'serialization-boundary)
       => 'rust-owned-cli-trace-cross-process)

(check (map (lambda (receipt) (alist-ref receipt 'case-id))
            vertical-case-driver-receipts)
       => '("real-repair-001"
            "real-policy-001/sandbox-denylist"
            "real-policy-002/retry-budget"
            "real-policy-003/maker-checker"
            "real-policy-004/dynamic-rewrite"
            "real-policy-005/memory-recall"
            "policy-combination/memory-rewrite-checker"))
(check (map (lambda (receipt) (alist-ref receipt 'profile-ref))
            vertical-case-driver-receipts)
       => '("real-repair-001/reactive-tool-loop"
            "real-policy-001/sandbox-denylist"
            "real-policy-002/retry-budget"
            "real-policy-003/maker-checker"
            "real-policy-004/dynamic-rewrite"
            "real-policy-005/memory-recall"
            "policy-combination/memory-rewrite-checker"))
(check (map (lambda (receipt) (alist-ref receipt 'runtime-mode))
            vertical-case-driver-receipts)
       => '(typed-loop-projection
            typed-loop-projection
            typed-loop-projection
            typed-loop-projection
            typed-loop-projection
            typed-loop-projection
            typed-loop-projection))
(check (map (lambda (receipt) (alist-ref receipt 'smoke-status))
            vertical-case-driver-receipts)
       => '(typed-loop-projection-ready
            typed-loop-projection-ready
            typed-loop-projection-ready
            typed-loop-projection-ready
            typed-loop-projection-ready
            typed-loop-projection-ready
            typed-loop-projection-ready))
(check (map (lambda (receipt) (alist-ref receipt 'loop-program-id))
            vertical-case-driver-receipts)
       => '("real-repair-001-scripted-loop"
            "real-policy-001-sandbox-denylist"
            "real-policy-002-retry-budget"
            "real-policy-003-maker-checker"
            "real-policy-004-dynamic-rewrite"
            "real-policy-005-memory-recall"
            "policy-combination-memory-rewrite-checker"))
(check (map (lambda (receipt) (alist-ref receipt 'capability-tags))
            vertical-case-driver-receipts)
       => '((+scripted-e2e +tool-repair +verification)
            (+sandbox +denylist)
            (+retry-budget +failure-policy)
            (+maker +checker)
            (+dynamic-rewrite +repair)
            (+memory-recall +tool-selection)
            (+policy-combination +memory +rewrite +checker)))
(check (map (lambda (receipt) (alist-ref receipt 'live-gate-env))
            vertical-case-driver-receipts)
       => '("MARLIN_LIVE_LLM" "none" "none" "none" "none" "none" "none"))
(check (map (lambda (receipt) (alist-ref receipt 'live-llm-required?))
            vertical-case-driver-receipts)
       => '(#t #f #f #f #f #f #f))
(check (map (lambda (receipt) (alist-ref receipt 'live-llm-allowed?))
            vertical-case-driver-receipts)
       => (make-list 7 #f))
(check (map (lambda (receipt) (alist-ref receipt 'live-llm-denial-receipt))
            vertical-case-driver-receipts)
       => '(deferred-no-live-llm
            not-required
            not-required
            not-required
            not-required
            not-required
            not-required))
(check (map (lambda (receipt) (alist-ref receipt 'llm-repair-intent))
            vertical-case-driver-receipts)
       => '(single-file-repair none none none none none none))
(check (map (lambda (receipt) (alist-ref receipt 'session-transform))
            vertical-case-driver-receipts)
       => (make-list 7 'loop-policy-profile->loop-program))
(check (map (lambda (receipt) (alist-ref receipt 'tool-intent-count))
            vertical-case-driver-receipts)
       => '(1 1 0 1 1 1 1))
(check (map (lambda (receipt) (alist-ref receipt 'memory-intent-count))
            vertical-case-driver-receipts)
       => '(0 0 0 0 0 1 1))
(check (map (lambda (receipt) (alist-ref receipt 'placement-intent-count))
            vertical-case-driver-receipts)
       => (make-list 7 1))
(check (map (lambda (receipt) (alist-ref receipt 'runtime-handoff-kind))
            vertical-case-driver-receipts)
       => (make-list 7 'loop-program-runtime-handoff))
(check (map (lambda (receipt) (alist-ref receipt 'runtime-receipt-kind))
            vertical-case-driver-receipts)
       => (make-list 7 'loop-program-runtime-receipt))
(check (map (lambda (receipt) (alist-ref receipt 'derived-session-kind))
            vertical-case-driver-receipts)
       => (make-list 7 'derived-session/from-loop-receipt))
(check (map (lambda (receipt) (alist-ref receipt 'module-kind))
            vertical-case-driver-receipts)
       => (make-list
           7
           "marlin.config-interface.loop-policy-profile-projection.v1"))
(check (map (lambda (receipt) (alist-ref receipt 'module-user-module))
            vertical-case-driver-receipts)
       => (make-list 7 'funflow))
(check (map (lambda (receipt) (alist-ref receipt 'module-selection-tags))
            vertical-case-driver-receipts)
       => '((+scripted-e2e +tool-repair +verification)
            (+sandbox +denylist)
            (+retry-budget +failure-policy)
            (+maker +checker)
            (+dynamic-rewrite +repair)
            (+memory-recall +tool-selection)
            (+policy-combination +memory +rewrite +checker)))
(check (map (lambda (receipt) (alist-ref receipt 'module-source-ref))
            vertical-case-driver-receipts)
       => '("real-repair-001/reactive-tool-loop"
            "real-policy-001/sandbox-denylist"
            "real-policy-002/retry-budget"
            "real-policy-003/maker-checker"
            "real-policy-004/dynamic-rewrite"
            "real-policy-005/memory-recall"
            "policy-combination/memory-rewrite-checker"))
(check (map (lambda (receipt) (alist-ref receipt 'module-entrypoint))
            vertical-case-driver-receipts)
       => (make-list 7 'marlinLoopPolicyProfileCompilerReceipts))
(check (map (lambda (receipt) (alist-ref receipt 'module-enabled?))
            vertical-case-driver-receipts)
       => (make-list 7 #t))
(check (map (lambda (receipt) (alist-ref receipt 'transition-count))
            vertical-case-driver-receipts)
       => '(6 2 3 3 4 3 6))
(check (alist-ref (car vertical-case-driver-receipts) 'transition-actions)
       => "invoke_model|dispatch_tools|continue|rewrite_graph|verify|stop")
(check (alist-ref (car vertical-case-driver-receipts) 'transition-events)
       => "start|tool_request|tool_receipt|model_event|runtime_receipt|verification_receipt")
(check (map (lambda (receipt) (alist-ref receipt 'mechanism-policy-count))
            vertical-case-driver-receipts)
       => '(3 2 2 1 2 2 3))
(check (map (lambda (receipt) (alist-ref receipt 'compiler-owner))
            vertical-case-driver-receipts)
       => (make-list 7 "gerbil-poo-flow"))
(check (map (lambda (receipt) (string? (alist-ref receipt 'mechanism-policy-ids)))
            vertical-case-driver-receipts)
       => (make-list 7 #t))
(check (not (string=? (alist-ref (car vertical-case-driver-receipts)
                                  'mechanism-policy-ids)
                       ""))
       => #t)
(check (map (lambda (receipt) (positive-number?
                               (alist-ref receipt 'capability-mask)))
            vertical-case-driver-receipts)
       => (make-list 7 #t))
(check (map (lambda (receipt) (alist-ref receipt 'policy-digest-length))
            vertical-case-driver-receipts)
       => '(32 32 32 32 32 32 32))
(check (string? (alist-ref (car vertical-case-driver-receipts)
                           'policy-digest-octets))
       => #t)
(check (map (lambda (receipt) (alist-ref receipt 'scheme-boundary))
            vertical-case-driver-receipts)
       => '(scheme-types->rust-types
            scheme-types->rust-types
            scheme-types->rust-types
            scheme-types->rust-types
            scheme-types->rust-types
            scheme-types->rust-types
            scheme-types->rust-types))
(check (map (lambda (receipt) (alist-ref receipt 'serialization-boundary))
            vertical-case-driver-receipts)
       => '(rust-owned-cli-trace-cross-process
            rust-owned-cli-trace-cross-process
            rust-owned-cli-trace-cross-process
            rust-owned-cli-trace-cross-process
            rust-owned-cli-trace-cross-process
            rust-owned-cli-trace-cross-process
            rust-owned-cli-trace-cross-process))

(def +vertical-case-trace-fields+
  '(case-id
    profile-ref
    compiler-owner
    compiler-profile-id
    loop-program-id
    capability-tags
    live-gate-env
    live-llm-required?
    live-llm-allowed?
    live-llm-denial-receipt
    llm-repair-intent
    session-transform
    tool-intent-count
    memory-intent-count
    placement-intent-count
    runtime-handoff-kind
    runtime-receipt-kind
    derived-session-kind
    module-kind
    module-user-module
    module-selection-tags
    module-source-ref
    module-entrypoint
    module-enabled?
    resolved-policy-pack-policy-epoch
    loop-program-policy-epoch
    transition-count
    transition-actions
    transition-events
    mechanism-policy-count
    mechanism-policy-ids
    policy-digest-length
    policy-digest-octets
    capability-mask
    budget-max-attempts
    budget-max-cost-units
    budget-max-wall-time-ms
    scheme-boundary
    serialization-boundary))

(def (display-vertical-case-field index receipt field)
  (display "vertical-case.")
  (display index)
  (display ".")
  (display field)
  (display "=")
  (display (alist-ref receipt field))
  (newline))

(def (display-vertical-case-lines receipts)
  (let loop ((index 0)
             (rest receipts))
    (unless (null? rest)
      (let (receipt (car rest))
        (for-each
         (lambda (field)
           (display-vertical-case-field index receipt field))
         +vertical-case-trace-fields+)
        (loop (+ index 1) (cdr rest))))))

(display "config-interface-case-driver-ok")
(newline)
(display "case-driver-receipts=")
(display (length case-driver-receipts))
(newline)
(display "vertical-case-receipts=")
(display (length vertical-case-driver-receipts))
(newline)
(display-vertical-case-lines vertical-case-driver-receipts)

;;; -*- Gerbil -*-
;;; Boundary: Smoke test for Marlin-owned kernel policy config interface.

(import :clan/poo/object
        (only-in :poo-flow/src/module-system/facade
                 poo-flow-user-module-selection-key)
        :config-interface/modules/lib
        :config-interface/modules/prefabs/user-interface
        :config-interface/modules/prefabs/user-interface-delivery
        (only-in "../src/config-interface/custom/marline-kernel/config"
                 poo-flow-custom-module-runtime-handoff-module
                 poo-flow-custom-module-policy-receipt-gate-module
                 poo-flow-custom-module-loop-contract-module
                 poo-flow-custom-module-failure-retry-module
                 poo-flow-custom-module-runtime-handoff-llm-case
                 poo-flow-custom-module-policy-receipt-gate-llm-case
                 poo-flow-custom-module-loop-contract-llm-case
                 poo-flow-custom-module-failure-retry-llm-case)
        (only-in "../src/config-interface/init"
                 poo-flow-user-module-bundles))

;;; Boundary: Local checks stay scalar around POO-heavy values.
;; MarlinResult <- MarlinInput
(defrules check ()
  ((_ expression => expected)
   (let ((actual-value expression)
         (expected-value expected))
     (unless (equal? actual-value expected-value)
       (error "check failed" actual-value expected-value)))))

(def (contains? values value)
  (if (member value values) #t #f))

(def marline-init-selection-keys
  (map poo-flow-user-module-selection-key
       (UserInterfaceRootSelections poo-flow-user-module-bundles)))

(def marline-kernel-loop-profile-modules
  (list poo-flow-custom-module-runtime-handoff-module
        poo-flow-custom-module-policy-receipt-gate-module
        poo-flow-custom-module-loop-contract-module
        poo-flow-custom-module-failure-retry-module))

(def marline-kernel-loop-llm-case-modules
  (list poo-flow-custom-module-runtime-handoff-llm-case
        poo-flow-custom-module-policy-receipt-gate-llm-case
        poo-flow-custom-module-loop-contract-llm-case
        poo-flow-custom-module-failure-retry-llm-case))

(def marline-kernel-loop-selections
  (map UserInterfaceModuleBundleSelection
       marline-kernel-loop-profile-modules))

(def marline-kernel-loop-llm-case-selections
  (map UserInterfaceModuleBundleSelection
       marline-kernel-loop-llm-case-modules))

(def marline-kernel-loop-profiles
  (map (lambda (module-bundle)
         (car (UserInterfaceModuleBundleConfig module-bundle)))
       marline-kernel-loop-profile-modules))

(def marline-kernel-loop-llm-cases
  (map (lambda (module-bundle)
         (car (UserInterfaceModuleBundleConfig module-bundle)))
       marline-kernel-loop-llm-case-modules))

(def marline-kernel-loop-profile
  (car marline-kernel-loop-profiles))

(def (marline-metadata-ref object key)
  (cdr (assq key (.get object metadata))))

(def marline-kernel-loop-runtime
  (.get marline-kernel-loop-profile runtime))

(def marline-kernel-loop-budget
  (.get marline-kernel-loop-profile budget))

(def marline-kernel-loop-capability-policy
  (.get marline-kernel-loop-profile capability-policy))

(def marline-kernel-workspace
  (UserInterfaceWorkspace
   (.o workspace-root: "marline-kernel-workspace"
       interface-file: "interface.org"
       state-file: "state/marline-kernel-state.org"
       model-profile: "kernel-policy")))

(def marline-kernel-delivery
  (UserInterfaceDeliveryReceipt marline-kernel-workspace))

(check (contains? marline-init-selection-keys '(flow . funflow)) => #t)
(check (contains? marline-init-selection-keys '(loop . governor)) => #t)
(check (contains? marline-init-selection-keys '(sandbox . nono-sandbox)) => #t)
(check (contains? marline-init-selection-keys '(sandbox . cubeSandbox)) => #t)
(check (contains? marline-init-selection-keys '(sandbox . docker-sandbox)) => #t)
(check (contains? marline-init-selection-keys '(flow . loop-engine)) => #t)
(check (contains? marline-init-selection-keys '(custom . marline-kernel)) => #t)
(check (map poo-flow-user-module-selection-key marline-kernel-loop-selections)
       => '((flow . loop-engine)
            (flow . loop-engine)
            (flow . loop-engine)
            (flow . loop-engine)))
(check (map poo-flow-user-module-selection-key
            marline-kernel-loop-llm-case-selections)
       => '((flow . funflow)
            (flow . funflow)
            (flow . funflow)
            (flow . funflow)))
(check (length marline-kernel-loop-profiles) => 4)
(check (length marline-kernel-loop-llm-cases) => 4)
(check (map (lambda (llm-case) (.get llm-case map-name))
            marline-kernel-loop-llm-cases)
       => '(marlin-runtime-handoff-real-llm
            marlin-policy-receipt-gate-real-llm
            marlin-loop-contract-real-llm
            marlin-failure-retry-real-llm))
(check (map (lambda (profile) (.get profile profile-id))
            marline-kernel-loop-profiles)
       => '(marlin-runtime-handoff-profile
            marlin-policy-receipt-gate-profile
            marlin-loop-contract-profile
            marlin-failure-retry-profile))
(check (map (lambda (profile)
              (.get (car (.get profile use-cases)) name))
            marline-kernel-loop-profiles)
       => '(marlin-runtime-handoff
            marlin-policy-receipt-gate
            marlin-loop-contract
            marlin-failure-retry))
(check (map (lambda (profile) (.get profile real-llm-case))
            marline-kernel-loop-profiles)
       => '(marlin-runtime-handoff-real-llm
            marlin-policy-receipt-gate-real-llm
            marlin-loop-contract-real-llm
            marlin-failure-retry-real-llm))
(check (map (lambda (llm-case)
              (marline-metadata-ref llm-case 'profile-ref))
            marline-kernel-loop-llm-cases)
       => '(marlin-runtime-handoff-profile
            marlin-policy-receipt-gate-profile
            marlin-loop-contract-profile
            marlin-failure-retry-profile))
(check (map (lambda (llm-case)
              (marline-metadata-ref llm-case 'runtime-mode))
            marline-kernel-loop-llm-cases)
       => '(real-llm-opt-in real-llm-opt-in real-llm-opt-in real-llm-opt-in))
(check (map (lambda (llm-case)
              (marline-metadata-ref llm-case 'case-id))
            marline-kernel-loop-llm-cases)
       => '(marlin-runtime-handoff-real-llm
            marlin-policy-receipt-gate-real-llm
            marlin-loop-contract-real-llm
            marlin-failure-retry-real-llm))
(check (map (lambda (llm-case)
              (marline-metadata-ref llm-case 'live-gate-env))
            marline-kernel-loop-llm-cases)
       => '("MARLIN_RUN_REAL_LLM_CASES"
            "MARLIN_RUN_REAL_LLM_CASES"
            "MARLIN_RUN_REAL_LLM_CASES"
            "MARLIN_RUN_REAL_LLM_CASES"))
(check (map (lambda (profile) (.get (.get profile budget) max-actionable))
            marline-kernel-loop-profiles)
       => '(1 1 1 1))
(check (.get marline-kernel-loop-runtime capabilities)
       => '(+manifest-handoff +l2-receipts +typed-receipts))
(check (.get marline-kernel-loop-runtime handoff)
       => 'loop-governor-marlin-runtime-manifest)
(check (.get marline-kernel-loop-runtime owner)
       => "marlin-agent-core")
(check (.get marline-kernel-loop-runtime runtime-executed)
       => #f)
(check (.get marline-kernel-loop-budget max-actionable) => 1)
(check (.get marline-kernel-loop-budget max-attempts) => 1)
(check (.get marline-kernel-loop-capability-policy backend)
       => 'marlin-runtime)
(check (.get marline-kernel-loop-capability-policy unsupported-behavior)
       => 'deny-and-receipt)
(check (.get marline-kernel-delivery marlin-loops-policy-owner)
       => "marlin")
(check (.get marline-kernel-delivery
             marlin-loops-policy-control-plane-owner)
       => "poo-flow")
(check (.get marline-kernel-delivery
             marlin-loops-policy-runtime-execution-owner)
       => "marlin-agent-core")
(check (.get marline-kernel-delivery
             marlin-loops-policy-receipt-contract-count)
       => 8)

(display "config-interface-ok")
(newline)
(display "loop-policy-owner=")
(display (.get marline-kernel-delivery marlin-loops-policy-owner))
(newline)
(display "loop-receipt-contract-count=")
(display (.get marline-kernel-delivery
               marlin-loops-policy-receipt-contract-count))
(newline)
(display "kernel-loop-use-cases=")
(display (length marline-kernel-loop-profiles))
(newline)
(display "kernel-loop-profiles=")
(display (length marline-kernel-loop-profiles))
(newline)
(display "kernel-loop-llm-cases=")
(display (length marline-kernel-loop-llm-cases))
(newline)

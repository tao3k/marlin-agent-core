;;; -*- Gerbil -*-
;;; Boundary: Policy receipt validation loop profile.
;;; Invariant: Scheme declares receipt gates; Rust validates typed receipts.

(use-module loop-engine
  :config
  (.def (marlin-policy-receipt-gate-loop @ loop-engine-use-case
                                          name level mode goal)
    name: 'marlin-policy-receipt-gate
    level: 'l2
    mode: 'typed-receipt-validation
    goal: 'validate-policy-receipt-contracts)

  (.def (marlin-policy-receipt-gate-governor @ loop-engine-governor
                                             capabilities)
    capabilities: '(+policy +manifest-handoff +typed-receipts))

  (.def (marlin-policy-receipt-gate-result @ loop-engine-result
                                           default format required-fields)
    default: 'marlin.loop-engine.policy-receipt-gate-result.v1
    format: 'structured-alist
    required-fields: '(decision summary evidence runtime-owner receipt-contracts))

  (.def (marlin-policy-receipt-gate-state @ loop-engine-state
                                          store path acting-on)
    store: 'file
    path: "state/marline-policy-receipt-gate-loop-state.org"
    acting-on: 'marlin-runtime-kernel)

  (.def (marlin-policy-receipt-gate-budget @ loop-engine-budget
                                           max-actionable max-attempts
                                           weekly-runs)
    max-actionable: 1
    max-attempts: 1
    weekly-runs: 10)

  (.def (marlin-policy-receipt-gate-runtime @ loop-engine-runtime capabilities)
    capabilities: '(+manifest-handoff +l2-receipts +typed-receipts))

  (.def (marlin-policy-receipt-gate-capability-policy
         @ loop-engine-capability-policy
         backend isolation required optional unsupported-behavior)
    backend: 'marlin-runtime
    isolation: 'runtime-owned
    required: '(typed-receipts runtime-manifest catalog-resolution)
    optional: '(memory-recall compression-handoff)
    unsupported-behavior: 'deny-and-receipt)

  (.def (marlin-policy-receipt-gate-profile @ loop-engine-profile
                                            profile-id use-cases governor result
                                            state budget runtime capability-policy
                                            real-llm-case)
    profile-id: 'marlin-policy-receipt-gate-profile
    use-cases: (list marlin-policy-receipt-gate-loop)
    governor: marlin-policy-receipt-gate-governor
    result: marlin-policy-receipt-gate-result
    state: marlin-policy-receipt-gate-state
    budget: marlin-policy-receipt-gate-budget
    runtime: marlin-policy-receipt-gate-runtime
    capability-policy: marlin-policy-receipt-gate-capability-policy
    real-llm-case: 'marlin-policy-receipt-gate-real-llm))

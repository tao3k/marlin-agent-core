;;; -*- Gerbil -*-
;;; Boundary: Runtime handoff loop profile.
;;; Invariant: Scheme declares policy intent; Rust owns runtime execution.

(use-module loop-engine
  :config
  (.def (marlin-runtime-handoff-loop @ loop-engine-use-case
                                     name level mode goal)
    name: 'marlin-runtime-handoff
    level: 'l2
    mode: 'handoff-only
    goal: 'produce-marlin-runtime-receipt)

  (.def (marlin-runtime-handoff-governor @ loop-engine-governor capabilities)
    capabilities: '(+policy +manifest-handoff +typed-receipts))

  (.def (marlin-runtime-handoff-result @ loop-engine-result
                                       default format required-fields)
    default: 'marlin.loop-engine.runtime-handoff-result.v1
    format: 'structured-alist
    required-fields: '(decision summary evidence runtime-owner receipt-contracts))

  (.def (marlin-runtime-handoff-state @ loop-engine-state store path acting-on)
    store: 'file
    path: "state/marline-runtime-handoff-loop-state.org"
    acting-on: 'marlin-runtime-kernel)

  (.def (marlin-runtime-handoff-budget @ loop-engine-budget
                                       max-actionable max-attempts weekly-runs)
    max-actionable: 1
    max-attempts: 1
    weekly-runs: 10)

  (.def (marlin-runtime-handoff-runtime @ loop-engine-runtime capabilities)
    capabilities: '(+manifest-handoff +l2-receipts +typed-receipts))

  (.def (marlin-runtime-handoff-capability-policy
         @ loop-engine-capability-policy
         backend isolation required optional unsupported-behavior)
    backend: 'marlin-runtime
    isolation: 'runtime-owned
    required: '(typed-receipts runtime-manifest catalog-resolution)
    optional: '(memory-recall compression-handoff)
    unsupported-behavior: 'deny-and-receipt)

  (.def (marlin-runtime-handoff-profile @ loop-engine-profile
                                        profile-id use-cases governor result
                                        state budget runtime capability-policy
                                        real-llm-case)
    profile-id: 'marlin-runtime-handoff-profile
    use-cases: (list marlin-runtime-handoff-loop)
    governor: marlin-runtime-handoff-governor
    result: marlin-runtime-handoff-result
    state: marlin-runtime-handoff-state
    budget: marlin-runtime-handoff-budget
    runtime: marlin-runtime-handoff-runtime
    capability-policy: marlin-runtime-handoff-capability-policy
    real-llm-case: 'marlin-runtime-handoff-real-llm))

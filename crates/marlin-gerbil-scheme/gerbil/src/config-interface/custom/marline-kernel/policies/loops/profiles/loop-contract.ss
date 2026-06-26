;;; -*- Gerbil -*-
;;; Boundary: Loop contract publication profile.
;;; Invariant: this profile is report-only until Rust consumes its receipts.

package: config-interface/custom/marline-kernel/policies/loops/profiles

(import :poo-flow/src/module-system/init-syntax)

(export poo-flow-custom-module-loop-contract-module)

(def poo-flow-custom-module-loop-contract-module
  (use-module loop-engine
  :config
  (.def (marlin-loop-contract-loop @ loop-engine-use-case
                                   name level mode goal)
    name: 'marlin-loop-contract
    level: 'l1
    mode: 'report-only
    goal: 'publish-loop-engine-contracts)

  (.def (marlin-loop-contract-governor @ loop-engine-governor capabilities)
    capabilities: '(+policy +manifest-handoff +typed-receipts))

  (.def (marlin-loop-contract-result @ loop-engine-result
                                     default format required-fields)
    default: 'marlin.loop-engine.loop-contract-result.v1
    format: 'structured-alist
    required-fields: '(decision summary evidence runtime-owner receipt-contracts))

  (.def (marlin-loop-contract-state @ loop-engine-state store path acting-on)
    store: 'file
    path: "state/marline-loop-contract-loop-state.org"
    acting-on: 'marlin-runtime-kernel)

  (.def (marlin-loop-contract-budget @ loop-engine-budget
                                     max-actionable max-attempts weekly-runs)
    max-actionable: 1
    max-attempts: 1
    weekly-runs: 10)

  (.def (marlin-loop-contract-runtime @ loop-engine-runtime capabilities)
    capabilities: '(+manifest-handoff +l1-receipts +typed-receipts))

  (.def (marlin-loop-contract-capability-policy
         @ loop-engine-capability-policy
         backend isolation required optional unsupported-behavior)
    backend: 'marlin-runtime
    isolation: 'runtime-owned
    required: '(typed-receipts runtime-manifest catalog-resolution)
    optional: '(memory-recall compression-handoff)
    unsupported-behavior: 'deny-and-receipt)

  (.def (marlin-loop-contract-profile @ loop-engine-profile
                                      profile-id use-cases governor result
                                      state budget runtime capability-policy
                                      real-llm-case)
    profile-id: 'marlin-loop-contract-profile
    use-cases: (list marlin-loop-contract-loop)
    governor: marlin-loop-contract-governor
    result: marlin-loop-contract-result
    state: marlin-loop-contract-state
    budget: marlin-loop-contract-budget
    runtime: marlin-loop-contract-runtime
    capability-policy: marlin-loop-contract-capability-policy
    real-llm-case: 'marlin-loop-contract-real-llm)))

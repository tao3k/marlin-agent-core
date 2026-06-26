;;; -*- Gerbil -*-
;;; Boundary: Failure retry loop profile for kernel policy experiments.
;;; Invariant: Scheme declares retry intent; Rust owns failure classification and execution.

package: config-interface/custom/marline-kernel/policies/loops/profiles

(import :poo-flow/src/module-system/init-syntax)

(export poo-flow-custom-module-failure-retry-module)

(def poo-flow-custom-module-failure-retry-module
  (use-module loop-engine
  :config
  (.def (marlin-failure-retry-loop @ loop-engine-use-case
                                   name level mode goal)
    name: 'marlin-failure-retry
    level: 'l2
    mode: 'typed-failure-retry
    goal: 'retry-failed-loop-with-typed-recovery)

  (.def (marlin-failure-retry-governor @ loop-engine-governor capabilities)
    capabilities: '(+failure-observation +retry +typed-recovery))

  (.def (marlin-failure-retry-result @ loop-engine-result
                                     default format required-fields)
    default: 'marlin.loop-engine.failure-retry-result.v1
    format: 'structured-alist
    required-fields: '(decision summary failure-classification retry-plan evidence))

  (.def (marlin-failure-retry-state @ loop-engine-state store path acting-on)
    store: 'file
    path: "state/marline-failure-retry-loop-state.org"
    acting-on: 'marlin-runtime-kernel)

  (.def (marlin-failure-retry-sandbox @ loop-engine-sandbox
                                      profile isolation profile-refs entries)
    profile: 'nono-profile
    isolation: 'runtime-owned
    profile-refs: '(nono-profile)
    entries: '((backend . nono)
               (filesystem-scope . runtime)
               (network-access . #f)
               (session-kind . sub-agent)
               (visible-namespaces . (system workspace tools))
               (verifier-pass-statuses . (Completed))
               (verifier-retry-statuses . (Failed))
               (verifier-human-audit-statuses . (Cancelled))))

  (.def (marlin-failure-retry-budget @ loop-engine-budget
                                     max-actionable max-attempts weekly-runs)
    max-actionable: 1
    max-attempts: 3
    weekly-runs: 10)

  (.def (marlin-failure-retry-runtime @ loop-engine-runtime capabilities)
    capabilities: '(+failure-classification +retry-continuation +typed-receipts))

  (.def (marlin-failure-retry-capability-policy
         @ loop-engine-capability-policy
         backend isolation required optional unsupported-behavior)
    backend: 'marlin-runtime
    isolation: 'runtime-owned
    required: '(failure-classification continuation-receipt retry-budget)
    optional: '(repair-graph human-review)
    unsupported-behavior: 'deny-and-receipt)

  (.def (marlin-failure-retry-profile @ loop-engine-profile
                                      profile-id use-cases governor result
                                      state sandbox budget runtime
                                      capability-policy real-llm-case)
    profile-id: 'marlin-failure-retry-profile
    use-cases: (list marlin-failure-retry-loop)
    governor: marlin-failure-retry-governor
    result: marlin-failure-retry-result
    state: marlin-failure-retry-state
    sandbox: marlin-failure-retry-sandbox
    budget: marlin-failure-retry-budget
    runtime: marlin-failure-retry-runtime
    capability-policy: marlin-failure-retry-capability-policy
    real-llm-case: 'marlin-failure-retry-real-llm)))

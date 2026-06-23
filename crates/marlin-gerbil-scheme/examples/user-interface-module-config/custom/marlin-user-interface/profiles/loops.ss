;;; -*- Gerbil -*-
;;; Boundary: designer-authored loop policy objects.
;;; Invariant: this fragment only composes POO loop profiles; Marlin owns runtime execution.

(use-module loop-engine
  :config
  (.def (marlin-ui-loop @ loop-engine-use-case name level mode goal)
    name: 'marlin-user-interface
    level: 'l1
    mode: 'report-only
    goal: 'produce-policy-delivery-receipt)

  (.def (marlin-ui-governor @ loop-engine-governor capabilities)
    capabilities: '(+policy +manifest-handoff))

  (.def (marlin-ui-state @ loop-engine-state store path acting-on)
    store: 'file
    path: "state/worker-state.org"
    acting-on: 'user-interface-workspace)

  (.def (marlin-ui-budget @ loop-engine-budget max-actionable max-attempts)
    max-actionable: 1
    max-attempts: 1)

  (.def (marlin-ui-runtime @ loop-engine-runtime capabilities)
    capabilities: '(+manifest-handoff +l1-receipts))

  (.def (marlin-ui-profile @ loop-engine-profile
                            use-case governor state budget runtime)
    use-case: marlin-ui-loop
    governor: marlin-ui-governor
    state: marlin-ui-state
    budget: marlin-ui-budget
    runtime: marlin-ui-runtime))

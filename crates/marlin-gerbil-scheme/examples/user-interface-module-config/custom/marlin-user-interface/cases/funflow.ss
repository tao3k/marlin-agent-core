;;; -*- Gerbil -*-
;;; Boundary: downstream verification flow for the Marlin user-interface case.
;;; Invariant: this is report-only control-plane data.

(use-module funflow
  :config
  (.def (marlin-ui/delivery-smoke @ funflow-check
                                  check-name profile-ref command-vector
                                  artifact-outputs result-protocol runtime-mode
                                  observability observes)
    check-name: 'marlin-ui-delivery-smoke
    profile-ref: 'marlin-user-interface/session
    command-vector: '("gxi" "t/user-interface-module-config-test.ss")
    artifact-outputs: '(policy-delivery-receipt)
    result-protocol: '(read :lines)
    runtime-mode: 'manifest-handoff
    observability: 'marlin-user-interface-summary
    observes: '(policy-projection receipt-owners loop-governor))

  (.def (marlin-ui/policy-handoff @ funflow-pipeline
                                  pipeline-name checks metadata)
    pipeline-name: 'marlin-ui-policy-handoff
    checks: (list marlin-ui/delivery-smoke)
    metadata: '((surface . marlin-user-interface)
                (control-plane . poo-flow)
                (execution-owner . marlin-agent-core))))

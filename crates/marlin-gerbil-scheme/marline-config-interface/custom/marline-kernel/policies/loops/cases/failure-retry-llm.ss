;;; -*- Gerbil -*-
;;; Boundary: Real LLM case manifest for failure retry loop policy.
;;; Invariant: the debug CLI consumes the typed loop request and enforces retry budget.

(use-module funflow
  :config
  (.def (marlin-failure-retry-real-llm @ funflow-check
                                       check-name profile-ref command-vector
                                       artifact-outputs result-protocol
                                       runtime-mode observability observes)
    check-name: 'marlin-failure-retry-real-llm
    profile-ref: 'marlin-failure-retry-profile
    command-vector: '("marlin" "loop" "run"
                      "--input" "../../../.cache/marlin/loop-cases/failure-retry-llm.loop.json"
                      "--catalog" "custom/marline-kernel/policies/loops/cases/real-llm-catalog.toml"
                      "--continuation-planner" "retry-on-failure"
                      "--no-store")
    artifact-outputs: '(llm-transcript failure-classification continuation-receipt)
    result-protocol: '(read :typed-failure-retry)
    runtime-mode: 'real-llm-opt-in
    observability: 'marlin-failure-retry-real-llm
    observes: '(failure-observation retry-continuation typed-recovery))

  (.def (marlin-failure-retry-real-llm-pipeline @ funflow-pipeline
                                                pipeline-name checks metadata)
    pipeline-name: 'marlin-failure-retry-real-llm
    checks: (list marlin-failure-retry-real-llm)
    metadata: '((profile-ref . marlin-failure-retry-profile)
                (runtime-mode . real-llm-opt-in)
                (case-id . marlin-failure-retry-real-llm)
                (goal . "recover a failed loop iteration by observing the failure and retrying under typed budget")
                (max-rounds . 3)
                (live-gate-env . "MARLIN_RUN_REAL_LLM_CASES"))))

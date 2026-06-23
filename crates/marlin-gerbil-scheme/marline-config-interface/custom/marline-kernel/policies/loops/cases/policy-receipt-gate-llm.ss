;;; -*- Gerbil -*-
;;; Boundary: Real LLM case manifest for policy receipt validation.
;;; Invariant: default smoke validates the manifest; live execution is opt-in.

(use-module funflow
  :config
  (.def (marlin-policy-receipt-gate-real-llm @ funflow-check
                                             check-name profile-ref
                                             command-vector artifact-outputs
                                             result-protocol runtime-mode
                                             observability observes)
    check-name: 'marlin-policy-receipt-gate-real-llm
    profile-ref: 'marlin-policy-receipt-gate-profile
    command-vector: '("marlin" "loop" "run"
                      "--input" "../../../.cache/marlin/loop-cases/policy-receipt-gate-llm.loop.json"
                      "--catalog" "custom/marline-kernel/policies/loops/cases/real-llm-catalog.toml"
                      "--continuation-planner" "repeat-graph"
                      "--no-store")
    artifact-outputs: '(llm-transcript policy-projection typed-receipts)
    result-protocol: '(read :typed-receipt)
    runtime-mode: 'real-llm-opt-in
    observability: 'marlin-policy-receipt-gate-real-llm
    observes: '(policy-receipt-contracts validation typed-receipts))

  (.def (marlin-policy-receipt-gate-real-llm-pipeline @ funflow-pipeline
                                                      pipeline-name checks
                                                      metadata)
    pipeline-name: 'marlin-policy-receipt-gate-real-llm
    checks: (list marlin-policy-receipt-gate-real-llm)
    metadata: '((profile-ref . marlin-policy-receipt-gate-profile)
                (runtime-mode . real-llm-opt-in)
                (case-id . marlin-policy-receipt-gate-real-llm)
                (goal . "recover a policy receipt validation failure by identifying missing typed evidence")
                (max-rounds . 3)
                (live-gate-env . "MARLIN_RUN_REAL_LLM_CASES"))))

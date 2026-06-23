;;; -*- Gerbil -*-
;;; Boundary: Real LLM case manifest for runtime handoff.
;;; Invariant: default smoke validates the manifest; live execution is opt-in.

(use-module funflow
  :config
  (.def (marlin-runtime-handoff-real-llm @ funflow-check
                                         check-name profile-ref command-vector
                                         artifact-outputs result-protocol
                                         runtime-mode observability observes)
    check-name: 'marlin-runtime-handoff-real-llm
    profile-ref: 'marlin-runtime-handoff-profile
    command-vector: '("marlin" "loop" "run"
                      "--input" "../../../.cache/marlin/loop-cases/runtime-handoff-llm.loop.json"
                      "--catalog" "custom/marline-kernel/policies/loops/cases/real-llm-catalog.toml"
                      "--continuation-planner" "repeat-graph"
                      "--no-store")
    artifact-outputs: '(llm-transcript runtime-manifest typed-receipts)
    result-protocol: '(read :typed-receipt)
    runtime-mode: 'real-llm-opt-in
    observability: 'marlin-runtime-handoff-real-llm
    observes: '(runtime-handoff catalog-resolution typed-receipts))

  (.def (marlin-runtime-handoff-real-llm-pipeline @ funflow-pipeline
                                                  pipeline-name checks metadata)
    pipeline-name: 'marlin-runtime-handoff-real-llm
    checks: (list marlin-runtime-handoff-real-llm)
    metadata: '((profile-ref . marlin-runtime-handoff-profile)
                (runtime-mode . real-llm-opt-in)
                (case-id . marlin-runtime-handoff-real-llm)
                (goal . "recover a failed runtime handoff by producing a typed receipt repair plan")
                (max-rounds . 3)
                (live-gate-env . "MARLIN_RUN_REAL_LLM_CASES"))))

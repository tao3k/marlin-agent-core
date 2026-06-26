;;; -*- Gerbil -*-
;;; Boundary: Real LLM case manifest for loop contract publication.
;;; Invariant: default smoke validates the manifest; live execution is opt-in.

package: config-interface/custom/marline-kernel/policies/loops/cases

(import :poo-flow/src/module-system/init-syntax)

(export poo-flow-custom-module-loop-contract-llm-case)

(def poo-flow-custom-module-loop-contract-llm-case
  (use-module funflow
  :config
  (.def (marlin-loop-contract-real-llm @ funflow-check
                                       check-name profile-ref command-vector
                                       artifact-outputs result-protocol
                                       runtime-mode observability observes)
    check-name: 'marlin-loop-contract-real-llm
    profile-ref: 'marlin-loop-contract-profile
    command-vector: '("marlin" "loop" "run"
                      "--input" "../../../.cache/marlin/loop-cases/loop-contract-llm.loop.json"
                      "--catalog" "custom/marline-kernel/policies/loops/cases/real-llm-catalog.toml"
                      "--continuation-planner" "repeat-graph"
                      "--no-store")
    artifact-outputs: '(llm-transcript loop-contract typed-receipts)
    result-protocol: '(read :typed-receipt)
    runtime-mode: 'real-llm-opt-in
    observability: 'marlin-loop-contract-real-llm
    observes: '(loop-contract publication typed-receipts))

  (.def (marlin-loop-contract-real-llm-pipeline @ funflow-pipeline
                                                pipeline-name checks metadata)
    pipeline-name: 'marlin-loop-contract-real-llm
    checks: (list marlin-loop-contract-real-llm)
    metadata: '((profile-ref . marlin-loop-contract-profile)
                (runtime-mode . real-llm-opt-in)
                (case-id . marlin-loop-contract-real-llm)
                (goal . "recover a loop contract publication failure by producing the missing contract summary")
                (max-rounds . 3)
                (live-gate-env . "MARLIN_RUN_REAL_LLM_CASES")))))

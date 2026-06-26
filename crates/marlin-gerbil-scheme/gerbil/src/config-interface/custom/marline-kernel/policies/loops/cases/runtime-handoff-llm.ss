;;; -*- Gerbil -*-
;;; Boundary: Real LLM case manifest for runtime handoff.
;;; Invariant: default smoke validates the manifest; live execution is opt-in.

package: config-interface/custom/marline-kernel/policies/loops/cases

(import :poo-flow/src/module-system/init-syntax)

(export poo-flow-custom-module-runtime-handoff-llm-case)

(def poo-flow-custom-module-runtime-handoff-llm-case
  (use-module funflow
  :config
  (.def (marlin-runtime-handoff-real-llm @ funflow-check
                                         check-name profile-ref command-vector
                                         artifact-outputs result-protocol
                                         runtime-mode observability observes)
    check-name: 'marlin-runtime-handoff-real-llm
    profile-ref: 'marlin-runtime-handoff-profile
    command-vector: '("marlin" "loop" "program" "run"
                      "--input" "t/fixtures/config-interface/loop-cases/runtime-handoff-llm.loop-program-run.json")
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
                (live-gate-env . "MARLIN_RUN_REAL_LLM_CASES")))))

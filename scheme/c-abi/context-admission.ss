;;; Build-owned entry for the upstream rooted-object C ABI.

(import :gerbil-scheme-rust/scheme/runtime
        "../context/batch.ss")

(begin-foreign
  (c-define (context-admit-batch root) (int64) int64
    "marlin_context_admit_batch" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call root
      marlin-agent-core/context/batch#marlin-context-admit-batch)))

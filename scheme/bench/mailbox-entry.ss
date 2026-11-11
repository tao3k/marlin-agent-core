;;; Qualification image for the production mailbox transport.
package: marlin-benchmark
(import :gerbil-scheme-rust/scheme/runtime
        (prefix-in :marlin-agent-core/c-abi/deck-runtime production.)
        (only-in :marlin-benchmark/mailbox run-mailbox-benchmark)
        (only-in :marlin-benchmark/context-profile run-context-profile run-context-retention-profile))
(export main)
(def (main . _) (void))
(begin-foreign
  (c-define (mailbox-run root) (int64) int64 "marlin_deck_mailbox_benchmark" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call
      root marlin-benchmark/mailbox#run-mailbox-benchmark)))

(begin-foreign
  (c-define (context-profile root) (int64) int64 "marlin_deck_context_profile" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call
      root marlin-benchmark/context-profile#run-context-profile)))

(begin-foreign
  (c-define (context-retention root) (int64) int64 "marlin_deck_context_retention" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call
      root marlin-benchmark/context-profile#run-context-retention-profile)))

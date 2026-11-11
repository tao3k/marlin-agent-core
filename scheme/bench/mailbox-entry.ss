;;; Qualification image for the production mailbox transport.
package: marlin-benchmark
(import :gerbil-scheme-rust/scheme/runtime
        (prefix-in :marlin-agent-core/c-abi/deck-runtime production.)
        (only-in :marlin-benchmark/mailbox run-mailbox-benchmark))
(export main)
(def (main . _) (void))
(begin-foreign
  (c-define (mailbox-run root) (int64) int64 "marlin_deck_mailbox_benchmark" "extern"
    (gerbil-scheme-rust/scheme/runtime#gerbil-rs-rooted-call
      root marlin-benchmark/mailbox#run-mailbox-benchmark)))

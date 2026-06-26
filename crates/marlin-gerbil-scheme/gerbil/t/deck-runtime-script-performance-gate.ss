;;; -*- Gerbil -*-
;;; Boundary: gxi entrypoint for the Scenario benchmark performance gate.

(import (only-in :std/test check)
        :clan/poo/object
        (only-in :gslph/src/benchmark/gate
                 benchmark-fixture-contract-pass?
                 benchmark-receipt-pass?
                 benchmark-run)
        (only-in :marlin/deck-runtime-script
                 marlin-deck-runtime-script-batch-metrics-kind)
        (only-in :marlin-deck-runtime/src/marlin/deck-runtime-script-performance
                 deck-runtime-script-performance-context
                 deck-runtime-script-performance-run-batch))

(def fixture-path
  "t/scenarios/performance/deck-runtime-script-batch/benchmark.ss")

(def fixture
  (call-with-input-file fixture-path read))

(def benchmark-iterations
  1024)

(def benchmark-context
  (deck-runtime-script-performance-context))

(def (deck-runtime-script-performance-assert-metrics metrics iterations)
  (check (.get metrics kind)
         => marlin-deck-runtime-script-batch-metrics-kind)
  (check (.get metrics script-id) => "performance-script")
  (check (.get metrics iterations) => iterations)
  (check (.get metrics runs) => iterations)
  (check (>= (.get metrics elapsed-us) 0) => #t))

(check (benchmark-fixture-contract-pass? fixture) => #t)

(deck-runtime-script-performance-assert-metrics
 (deck-runtime-script-performance-run-batch
  benchmark-iterations
  benchmark-context)
 benchmark-iterations)

(def receipt
  (benchmark-run
   fixture
   (lambda ()
     (deck-runtime-script-performance-run-batch
      benchmark-iterations
      benchmark-context))))

(check (benchmark-receipt-pass? receipt) => #t)

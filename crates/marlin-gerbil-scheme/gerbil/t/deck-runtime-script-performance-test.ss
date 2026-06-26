;;; -*- Gerbil -*-
;;; Boundary: Test owns quick script performance and batch-run contracts.

(import (only-in :std/test
                 check
                 test-case
                 test-suite)
        :clan/poo/object
        (only-in :marlin/deck-runtime-script
                 marlin-deck-runtime-script-batch-metrics-kind)
        (only-in :marlin/deck-runtime-script-performance
                 deck-runtime-script-performance-count-runs
                 deck-runtime-script-performance-context
                 deck-runtime-script-performance-run-batch))

(export deck-runtime-script-performance-assert-metrics
        deck-runtime-script-performance-measure
        deck-runtime-script-performance-test)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (deck-runtime-script-performance-assert-metrics metrics iterations)
  (check (.get metrics kind)
         => marlin-deck-runtime-script-batch-metrics-kind)
  (check (.get metrics script-id) => "performance-script")
  (check (.get metrics iterations) => iterations)
  (check (.get metrics runs) => iterations)
  (check (>= (.get metrics elapsed-us) 0) => #t))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (deck-runtime-script-performance-measure iterations context)
  (let* ((runs
          (deck-runtime-script-performance-count-runs
           iterations
           context))
         (metrics
          (deck-runtime-script-performance-run-batch
           iterations
           context)))
    (check runs => iterations)
    (deck-runtime-script-performance-assert-metrics
     metrics
     iterations)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
;;; Boundary: Test suite keeps quick shape checks under gxtest.
;; MarlinResult <- MarlinInput
(def deck-runtime-script-performance-test
  (test-suite "deck runtime script performance"
    (test-case "keeps batch script execution shape valid"
      (deck-runtime-script-performance-measure
       128
       (deck-runtime-script-performance-context)))))

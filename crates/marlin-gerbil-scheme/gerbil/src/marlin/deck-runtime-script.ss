;;; -*- Gerbil -*-
;;; Boundary: Module owns quick Gerbil script interfaces for downstream POO extensions.

package: marlin

(import (only-in :clan/poo/object .get .o)
        :marlin/deck-runtime-extension
        :marlin/deck-runtime-native-projection)

(export marlin-deck-runtime-script-kind
        marlin-deck-runtime-script-interface-kind
        marlin-deck-runtime-script-interface-receipt-kind
        marlin-deck-runtime-script-batch-metrics-kind
        make-marlin-deck-runtime-script
        marlin-deck-runtime-script-extension
        marlin-deck-runtime-script-native-projection
        marlin-deck-runtime-script-interface-receipt
        marlin-deck-runtime-script-run
        count-marlin-deck-runtime-script-runs
        marlin-deck-runtime-script-batch-metrics
        defmarlin-deck-runtime-script)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-script-kind
  "marlin-deck-runtime.script.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-script-interface-kind
  "poo-native-api-or-gxi-script")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-script-interface-receipt-kind
  "marlin-deck-runtime.script-interface-receipt.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-deck-runtime-script-batch-metrics-kind
  "marlin-deck-runtime.script-batch-metrics.v1")

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (make-marlin-deck-runtime-script
      script-id-value
      extension-value
      action-value
      runner-value
      metadata-value)
  (.o kind: marlin-deck-runtime-script-kind
      interface: marlin-deck-runtime-script-interface-kind
      id: script-id-value
      extension: extension-value
      action: action-value
      runner: runner-value
      metadata: metadata-value
      native-projection:
      (make-marlin-deck-runtime-poo-policy-projection
       script-id-value
       action-value)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-script-extension script)
  (.get script extension))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-script-native-projection script)
  (.get script native-projection))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-script-interface-receipt script)
  (.o kind: marlin-deck-runtime-script-interface-receipt-kind
      script-id: (.get script id)
      interface: (.get script interface)
      action: (.get script action)
      extension-id: (.get (marlin-deck-runtime-script-extension script) id)
      metadata: (.get script metadata)
      native-projection:
      (marlin-deck-runtime-script-native-projection script)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-script-run script context)
  ((.get script runner) context))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (count-marlin-deck-runtime-script-runs script iterations context)
  (foldl (lambda (_ runs)
           (marlin-deck-runtime-script-run script context)
           (+ runs 1))
         0
         (list-tabulate iterations identity)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-script-elapsed-us start-jiffy end-jiffy)
  (quotient (* (- end-jiffy start-jiffy) 1000000) (jiffies-per-second)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-deck-runtime-script-batch-metrics script iteration-count context)
  (let ((start-jiffy (current-jiffy)))
    (let ((run-count
           (count-marlin-deck-runtime-script-runs
            script
            iteration-count
            context)))
      (let ((end-jiffy (current-jiffy)))
        (.o kind: marlin-deck-runtime-script-batch-metrics-kind
            script-id: (.get script id)
            interface: (.get script interface)
            iterations: iteration-count
            runs: run-count
            elapsed-us:
            (marlin-deck-runtime-script-elapsed-us
             start-jiffy
             end-jiffy))))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(defrules defmarlin-deck-runtime-script ()
  ((_ binding
      script-id
      extension
      action
      metadata
      (context-var)
      body ...)
   (def binding
     (make-marlin-deck-runtime-script
      script-id
      extension
      action
      (lambda (context-var) body ...)
      metadata))))

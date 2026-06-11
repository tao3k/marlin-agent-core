;;; -*- Gerbil -*-
;;; Library-module entry point for the Marlin Gerbil command adapter.

package: marlin

(import ./request ./parser ./protocol)

(export run-fixed-loop-graph-adapter main)

(def (run-fixed-loop-graph-adapter)
  (let* ((request (read-gerbil-compile-request))
         (expected (gerbil-compile-request-expected-kind request))
         (_ (ensure-marlin-supported-artifact-kind expected))
         (source-text (gerbil-compile-request-source-text request))
         (graph (compile-loop-graph source-text)))
    (display-gerbil-compile-response graph)
    (newline)))

(def (main . _args)
  (run-fixed-loop-graph-adapter))

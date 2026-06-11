;;; -*- Gerbil -*-
;;; Library-module entry point for the Marlin Gerbil command adapter.

package: marlin

(import ./request ./parser ./protocol)

(export run-fixed-loop-graph-adapter main)

(def (compile-requested-artifact expected source-text)
  (cond
    ((equal? expected marlin-loop-graph-artifact-kind)
     (compile-loop-graph source-text))
    ((equal? expected marlin-workspace-schema-artifact-kind)
     (compile-workspace-schema source-text))
    (else (error "marlin gerbil adapter cannot compile artifact kind" expected))))

(def (run-fixed-loop-graph-adapter)
  (let* ((request (read-gerbil-compile-request))
         (expected (gerbil-compile-request-expected-kind request))
         (_ (ensure-marlin-supported-artifact-kind expected))
         (source-text (gerbil-compile-request-source-text request))
         (artifact (compile-requested-artifact expected source-text)))
    (display-gerbil-artifact-response expected artifact)
    (newline)))

(def (main . _args)
  (run-fixed-loop-graph-adapter))

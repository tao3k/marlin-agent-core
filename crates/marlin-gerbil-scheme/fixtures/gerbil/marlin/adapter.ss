;;; -*- Gerbil -*-
;;; Library-module entry point for the Marlin Gerbil command adapter.

package: marlin

(import ./request ./parser ./protocol)

(export run-fixed-loop-graph-adapter main)

(def marlin-artifact-compilers
  (list (list marlin-loop-graph-artifact-kind compile-loop-graph)
        (list marlin-workspace-schema-artifact-kind compile-workspace-schema)))

(def (find-marlin-artifact-compiler expected)
  (let loop ((remaining marlin-artifact-compilers))
    (cond
      ((null? remaining) #f)
      ((equal? expected (car (car remaining))) (cadr (car remaining)))
      (else (loop (cdr remaining))))))

(def (compile-requested-artifact expected source-text)
  (let ((compiler (find-marlin-artifact-compiler expected)))
    (if compiler
      (compiler source-text)
      (error "marlin gerbil adapter cannot compile artifact kind" expected))))

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

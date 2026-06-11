;;; -*- Gerbil -*-
;;; Library-module entry point for the Marlin Gerbil command adapter.

package: marlin

(import ./request ./parser ./protocol)

(export run-marlin-command-adapter main)

(def marlin-artifact-compilers
  (list (list marlin-loop-graph-artifact-kind compile-loop-graph)
        (list marlin-workspace-schema-artifact-kind compile-workspace-schema)
        (list marlin-workspace-patch-intent-artifact-kind
              compile-workspace-patch-intent)))

(def (marlin-artifact-compiler-kind entry)
  (car entry))

(def (marlin-artifact-compiler-procedure entry)
  (cadr entry))

(def (find-marlin-artifact-compiler expected)
  (let loop ((remaining marlin-artifact-compilers))
    (cond
      ((null? remaining) #f)
      ((equal? expected (marlin-artifact-compiler-kind (car remaining)))
       (marlin-artifact-compiler-procedure (car remaining)))
      (else (loop (cdr remaining))))))

(def (ensure-marlin-supported-kind-has-compiler artifact-kind)
  (unless (find-marlin-artifact-compiler artifact-kind)
    (error "marlin gerbil adapter missing compiler for supported artifact kind"
           artifact-kind
           marlin-supported-artifact-kinds)))

(def (ensure-marlin-compiler-kind-is-supported artifact-kind)
  (unless (marlin-supported-artifact-kind? artifact-kind)
    (error "marlin gerbil adapter compiler table contains unsupported artifact kind"
           artifact-kind
           marlin-supported-artifact-kinds)))

(def (ensure-marlin-artifact-compiler-table)
  (let loop-supported ((remaining marlin-supported-artifact-kinds))
    (unless (null? remaining)
      (ensure-marlin-supported-kind-has-compiler (car remaining))
      (loop-supported (cdr remaining))))
  (let loop-compilers ((remaining marlin-artifact-compilers))
    (unless (null? remaining)
      (ensure-marlin-compiler-kind-is-supported
       (marlin-artifact-compiler-kind (car remaining)))
      (loop-compilers (cdr remaining)))))

(def (compile-requested-artifact expected source-text)
  (let ((compiler (find-marlin-artifact-compiler expected)))
    (if compiler
      (compiler source-text)
      (error "marlin gerbil adapter cannot compile artifact kind" expected))))

(def (run-marlin-command-adapter)
  (let* ((request (read-gerbil-compile-request))
         (expected (gerbil-compile-request-expected-kind request))
         (_ (ensure-marlin-supported-artifact-kind expected))
         (_ (ensure-marlin-artifact-compiler-table))
         (source-text (gerbil-compile-request-source-text request))
         (artifact (compile-requested-artifact expected source-text)))
    (display-gerbil-artifact-response expected artifact)
    (newline)))

(def (main . _args)
  (run-marlin-command-adapter))

;;; -*- Gerbil -*-
;;; Library-module entry point for the Marlin Gerbil command adapter.

package: marlin

(import ./request ./parser ./protocol)

(export run-marlin-command-adapter run-marlin-command-adapter-batch main)

(def marlin-artifact-compilers
  (list (list marlin-loop-graph-artifact-kind compile-loop-graph)
        (list marlin-workspace-schema-artifact-kind compile-workspace-schema)
        (list marlin-workspace-patch-intent-artifact-kind
              compile-workspace-patch-intent)
        (list marlin-agent-scenario-contract-artifact-kind
              compile-agent-scenario-contract)
        (list marlin-release-topology-artifact-kind
              compile-release-topology)))

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

(def (compile-requested-marlin-artifact expected source-text)
  (make-marlin-artifact expected (compile-requested-artifact expected source-text)))

(def (ensure-marlin-contract-facts-shape contract-facts)
  (when contract-facts
    (unless (hash-ref contract-facts "registry" #f)
      (error "marlin gerbil adapter contract_facts missing registry"))
    (unless (hash-ref contract-facts "resolutions" #f)
      (error "marlin gerbil adapter contract_facts missing resolutions"))
    (unless (hash-ref contract-facts "validations" #f)
      (error "marlin gerbil adapter contract_facts missing validations"))))

(def (compile-gerbil-compile-request request)
  (let* ((expected (gerbil-compile-request-expected-kind request))
         (_ (ensure-marlin-supported-artifact-kind expected))
         (_ (ensure-marlin-artifact-compiler-table))
         (contract-facts (gerbil-compile-request-contract-facts request))
         (_ (ensure-marlin-contract-facts-shape contract-facts))
         (source-text (gerbil-compile-request-source-text request))
         (artifact (compile-requested-marlin-artifact expected source-text)))
    artifact))

(def (marlin-error->string error)
  (call-with-output-string "" (lambda (port) (write error port))))

(def (display-marlin-compile-request-result request)
  (with-catch
   (lambda (error)
     (display-marlin-compile-error-response (marlin-error->string error)))
   (lambda ()
     (display-marlin-compile-response
      (compile-gerbil-compile-request request)))))

(def (run-marlin-command-adapter)
  (let ((artifact (compile-gerbil-compile-request (read-gerbil-compile-request))))
    (display-marlin-compile-response artifact)
    (newline)))

(def (run-marlin-command-adapter-batch)
  (let loop ((requests (read-gerbil-compile-request-lines)))
    (unless (null? requests)
      (display-marlin-compile-request-result (car requests))
      (newline)
      (loop (cdr requests)))))

(def (main . _args)
  (run-marlin-command-adapter))

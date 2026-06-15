;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; Typed Gerbil artifact compiler used behind Rust-owned runtime boundaries.

package: marlin

(import ./request ./parser ./protocol)

(export compile-requested-marlin-artifact
        compile-gerbil-compile-request
        compile-marlin-command-request-result)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def marlin-artifact-compilers
  (list (list marlin-loop-graph-artifact-kind compile-loop-graph)
        (list marlin-workspace-schema-artifact-kind compile-workspace-schema)
        (list marlin-workspace-patch-intent-artifact-kind
              compile-workspace-patch-intent)
        (list marlin-agent-scenario-contract-artifact-kind
              compile-agent-scenario-contract)
        (list marlin-release-topology-artifact-kind
              compile-release-topology)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-artifact-compiler-kind entry)
  (car entry))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (marlin-artifact-compiler-procedure entry)
  (cadr entry))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (find-marlin-artifact-compiler expected)
  (let (entry
        (find (lambda (candidate)
                (equal? expected (marlin-artifact-compiler-kind candidate)))
              marlin-artifact-compilers))
    (and entry (marlin-artifact-compiler-procedure entry))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (ensure-marlin-supported-kind-has-compiler artifact-kind)
  (unless (find-marlin-artifact-compiler artifact-kind)
    (error "marlin gerbil adapter missing compiler for supported artifact kind"
           artifact-kind
           marlin-supported-artifact-kinds)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (ensure-marlin-compiler-kind-is-supported artifact-kind)
  (unless (marlin-supported-artifact-kind? artifact-kind)
    (error "marlin gerbil adapter compiler table contains unsupported artifact kind"
           artifact-kind
           marlin-supported-artifact-kinds)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (ensure-marlin-artifact-compiler-table)
  (for-each ensure-marlin-supported-kind-has-compiler
            marlin-supported-artifact-kinds)
  (for-each (lambda (compiler)
              (ensure-marlin-compiler-kind-is-supported
               (marlin-artifact-compiler-kind compiler)))
            marlin-artifact-compilers))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (compile-requested-artifact expected source-text)
  (let ((compiler (find-marlin-artifact-compiler expected)))
    (if compiler
      (compiler source-text)
      (error "marlin gerbil adapter cannot compile artifact kind" expected))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (compile-requested-marlin-artifact expected source-text)
  (make-marlin-artifact expected (compile-requested-artifact expected source-text)))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (ensure-marlin-contract-facts-shape contract-facts)
  (when contract-facts
    (unless (hash-ref contract-facts "registry" #f)
      (error "marlin gerbil adapter contract_facts missing registry"))
    (unless (hash-ref contract-facts "resolutions" #f)
      (error "marlin gerbil adapter contract_facts missing resolutions"))
    (unless (hash-ref contract-facts "validations" #f)
      (error "marlin gerbil adapter contract_facts missing validations"))))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (compile-gerbil-compile-request request)
  (let* ((expected (gerbil-compile-request-expected-kind request))
         (_ (ensure-marlin-supported-artifact-kind expected))
         (_ (ensure-marlin-artifact-compiler-table))
         (contract-facts (gerbil-compile-request-contract-facts request))
         (_ (ensure-marlin-contract-facts-shape contract-facts))
         (source-text (gerbil-compile-request-source-text request))
         (artifact (compile-requested-marlin-artifact expected source-text)))
    artifact))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(def (compile-marlin-command-request-result request)
  (compile-gerbil-compile-request request))

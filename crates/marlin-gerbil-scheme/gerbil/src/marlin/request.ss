;;; -*- Gerbil -*-
;;; Boundary: Module owns Marlin Gerbil policy and runtime contracts for agent edits.
;;; GerbilCompileRequest typed value accessors used by Gerbil-side adapters.

package: marlin

(export make-gerbil-compile-request
        gerbil-compile-request-source-text
        gerbil-compile-request-expected-kind
        gerbil-compile-request-contract-facts)

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (make-gerbil-compile-request source-text expected-kind contract-facts)
  (list source-text expected-kind contract-facts))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (gerbil-compile-request-source-text request)
  (car request))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (gerbil-compile-request-expected-kind request)
  (cadr request))

;;; Boundary: Definition keeps a parser-owned edit boundary for policy repair.
;; MarlinResult <- MarlinInput
(define (gerbil-compile-request-contract-facts request)
  (caddr request))

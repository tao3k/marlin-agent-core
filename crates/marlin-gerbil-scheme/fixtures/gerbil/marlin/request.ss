;;; -*- Gerbil -*-
;;; GerbilCompileRequest JSON reader used by Gerbil-side adapters.

(import :std/text/json)

(export read-gerbil-compile-request
        gerbil-compile-request-source-text
        gerbil-compile-request-expected-kind)

(define (read-all input)
  (let loop ((acc ""))
    (let ((ch (read-char input)))
      (if (eof-object? ch)
        acc
        (loop (string-append acc (string ch)))))))

(define (read-gerbil-compile-request)
  (string->json-object (read-all (current-input-port))))

(define (gerbil-compile-request-source-text request)
  (let ((source (hash-ref request "source")))
    (hash-ref source "text")))

(define (gerbil-compile-request-expected-kind request)
  (hash-ref request "expected"))

;;; -*- Gerbil -*-
;;; GerbilCompileRequest JSON reader used by Gerbil-side adapters.

package: marlin

(import :std/text/json)

(export read-gerbil-compile-request
        read-gerbil-compile-request-lines
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

(define (read-line-or-eof input)
  (let loop ((chars '()))
    (let ((ch (read-char input)))
      (cond
       ((eof-object? ch)
        (if (null? chars) ch (list->string (reverse chars))))
       ((char=? ch #\newline)
        (list->string (reverse chars)))
       (else (loop (cons ch chars)))))))

(define (read-gerbil-compile-request-lines)
  (let loop ((requests '()))
    (let ((line (read-line-or-eof (current-input-port))))
      (cond
       ((eof-object? line) (reverse requests))
       ((equal? line "") (loop requests))
       (else (loop (cons (string->json-object line) requests)))))))

(define (gerbil-compile-request-source-text request)
  (let ((source (hash-ref request "source")))
    (hash-ref source "text")))

(define (gerbil-compile-request-expected-kind request)
  (hash-ref request "expected"))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Uniform native C-ABI record vector constructor and shared pure string
;;; helpers for `scheme/ui-bridge` modules.
;;; Record layout decoded by Rust `SchemeBridgeActor`:
;;;   #(record-id kind title summary source-path start-line end-line #(#(k v) ...))
(import (only-in :std/string/misc string-trim))
(export make-bridge-record-vec
        str-trim
        str-join
        str-suffix?
        str-downcase)

(def (datum->str x)
  (cond
    ((string? x) x)
    ((symbol? x) (symbol->string x))
    ((number? x) (number->string x))
    ((boolean? x) (if x "true" "false"))
    (else "")))

(def (str-trim s)
  (string-trim s))

(def (str-join xs sep)
  (cond
    ((null? xs) "")
    ((null? (cdr xs)) (car xs))
    (else
     (let loop ((rem (cdr xs)) (acc (list (car xs))))
       (if (null? rem)
         (apply string-append (reverse acc))
         (loop (cdr rem) (cons (car rem) (cons sep acc))))))))

(def (str-suffix? suf s)
  (let ((slen (string-length s))
        (flen (string-length suf)))
    (and (>= slen flen)
         (string=? (substring s (- slen flen) slen) suf))))

(def (str-downcase s)
  (let* ((len (string-length s))
         (out (make-string len)))
    (let loop ((i 0))
      (if (>= i len)
        out
        (begin
          (string-set! out i (char-downcase (string-ref s i)))
          (loop (+ i 1)))))))

(def (make-bridge-record-vec id kind title summary source-path start-line end-line facets-alist)
  (vector
    (if (string? id) id "")
    (if (string? kind) kind "")
    (if (string? title) title "")
    (if (string? summary) summary "")
    (if (string? source-path) source-path "")
    (if (exact-integer? start-line) start-line 0)
    (if (exact-integer? end-line) end-line 0)
    (list->vector
      (map (lambda (kv)
             (vector (datum->str (car kv))
                     (datum->str (cdr kv))))
           facets-alist))))

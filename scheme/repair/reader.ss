;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Stage A: Bounded Lexical Preprocessor & Safe Non-Eval S-Expression Reader
;;; (`scheme/repair/reader.ss`).
;;;
;;; Deterministically heals surface formatting slips without ever running eval:
;;;   - `strip-outer-code-fence`: ```scheme ... ``` or inline `(...)` fences
;;;   - `balance-sexp-parens`: string-, char-, and comment-aware paren balancing
;;;   - `read-bounded-search-sexp`: fail-closed readtable (no `#.(...)` eval,
;;;     no shared datum cycles, bounded bytes/nodes/depth)
(import (only-in :gerbil/runtime/gambit
                 current-readtable
                 readtable-eval-allowed?-set
                 readtable-sharing-allowed?-set)
        (only-in :std/string/misc string-trim)
        "./objects.ss")
(export add-repair-tag
        starts-with?
        ends-with?
        strip-outer-code-fence
        balance-sexp-parens
        bare-admitted-search-head?
        read-bounded-search-sexp)

(def (add-repair-tag tags tag)
  (if (memq tag tags) tags (append tags (list tag))))

(def (starts-with? s prefix)
  (let ((plen (string-length prefix)))
    (and (>= (string-length s) plen)
         (string=? (substring s 0 plen) prefix))))

(def (ends-with? s suffix)
  (let ((slen (string-length s))
        (xlen (string-length suffix)))
    (and (>= slen xlen)
         (string=? (substring s (- slen xlen) slen) suffix))))

(def (string-index-char s ch start)
  (let ((len (string-length s)))
    (let loop ((i start))
      (cond
        ((>= i len) #f)
        ((char=? (string-ref s i) ch) i)
        (else (loop (+ i 1)))))))

;;; Safely strip a single outer markdown code fence (```scheme ... ``` or
;;; single backticks `(...)`). Never coerces non-S-expression prose.
(def (strip-outer-code-fence text tags)
  (let ((s (string-trim text)))
    (cond
      ((starts-with? s "```")
       (let ((nl (string-index-char s #\newline 3)))
         (if (not nl)
           (values s tags)
           (let* ((after-open (substring s (+ nl 1) (string-length s)))
                  (trimmed-after (string-trim after-open))
                  (unwrapped (if (ends-with? trimmed-after "```")
                               (string-trim
                                 (substring trimmed-after 0 (- (string-length trimmed-after) 3)))
                               trimmed-after)))
             (values unwrapped (add-repair-tag tags 'strip-code-fence))))))
      ((and (>= (string-length s) 3)
            (char=? (string-ref s 0) #\`)
            (char=? (string-ref s (- (string-length s) 1)) #\`))
       (let ((inner (string-trim (substring s 1 (- (string-length s) 1)))))
         (if (and (> (string-length inner) 0) (char=? (string-ref inner 0) #\())
           (values inner (add-repair-tag tags 'strip-code-fence))
           (values s tags))))
      (else (values s tags)))))

;;; String-, char-literal-, and comment-aware parenthesis balancer.
(def (balance-sexp-parens text tags)
  (let ((len (string-length text)))
    (let loop ((i 0)
               (depth 0)
               (in-string? #f)
               (escape? #f)
               (in-comment? #f)
               (first-zero-end #f)
               (extra-closes 0)
               (non-close-after-zero? #f))
      (if (>= i len)
        (cond
          (in-string?
           (error "unterminated string literal in :gerbil-ascent query"))
          ((> depth 0)
           (if (<= depth +max-close-parens-repair+)
             (values (string-append text (make-string depth #\)))
                     (add-repair-tag tags 'close-missing-parens))
             (error "unbalanced parentheses exceed safe auto-repair bound" depth)))
          ((and first-zero-end
                (> extra-closes 0)
                (<= extra-closes +max-trim-parens-repair+)
                (not non-close-after-zero?))
           (values (substring text 0 first-zero-end)
                   (add-repair-tag tags 'trim-extra-closing-parens)))
          (else (values text tags)))
        (let ((ch (string-ref text i)))
          (cond
            (in-comment?
             (loop (+ i 1) depth #f #f (not (char=? ch #\newline))
                   first-zero-end extra-closes non-close-after-zero?))
            (in-string?
             (cond
               (escape?
                (loop (+ i 1) depth #t #f #f
                      first-zero-end extra-closes non-close-after-zero?))
               ((char=? ch #\\)
                (loop (+ i 1) depth #t #t #f
                      first-zero-end extra-closes non-close-after-zero?))
               ((char=? ch #\")
                (loop (+ i 1) depth #f #f #f
                      first-zero-end extra-closes non-close-after-zero?))
               (else
                (loop (+ i 1) depth #t #f #f
                      first-zero-end extra-closes non-close-after-zero?))))
            ((char=? ch #\;)
             (loop (+ i 1) depth #f #f #t
                   first-zero-end extra-closes non-close-after-zero?))
            ((char=? ch #\")
             (loop (+ i 1) depth #t #f #f
                   first-zero-end extra-closes
                   (or non-close-after-zero? (and first-zero-end #t))))
            ((and (char=? ch #\#)
                  (< (+ i 2) len)
                  (char=? (string-ref text (+ i 1)) #\\))
             (loop (+ i 3) depth #f #f #f
                   first-zero-end extra-closes
                   (or non-close-after-zero? (and first-zero-end #t))))
            ((char=? ch #\()
             (loop (+ i 1) (+ depth 1) #f #f #f
                   first-zero-end extra-closes
                   (or non-close-after-zero? (and first-zero-end #t))))
            ((char=? ch #\))
             (if (> depth 0)
               (let ((nd (- depth 1)))
                 (loop (+ i 1) nd #f #f #f
                       (if (and (= nd 0) (not first-zero-end)) (+ i 1) first-zero-end)
                       extra-closes non-close-after-zero?))
               (loop (+ i 1) 0 #f #f #f
                     first-zero-end (+ extra-closes 1) non-close-after-zero?)))
            ((char-whitespace? ch)
             (loop (+ i 1) depth #f #f #f
                   first-zero-end extra-closes non-close-after-zero?))
            (else
             (loop (+ i 1) depth #f #f #f
                   first-zero-end extra-closes
                   (or non-close-after-zero? (and first-zero-end #t))))))))))

(def (bare-admitted-search-head? text)
  (ormap (lambda (head)
           (let ((h (symbol->string head)))
             (and (starts-with? text h)
                  (or (= (string-length text) (string-length h))
                      (char-whitespace? (string-ref text (string-length h)))))))
         +admitted-top-heads+))

;;; Read one bounded S-expression with eval and datum-sharing disabled.
;;; Returns `(values form lexical-repair-tags)`.
(def (read-bounded-search-sexp text)
  (unless (string? text)
    (error "search query must be a string"))
  (let ((raw-trimmed (string-trim text)))
    (when (or (string=? raw-trimmed "")
              (> (u8vector-length (string->utf8 raw-trimmed)) +max-dsl-bytes+))
      (error "search query empty or exceeds byte bound"))
    (let-values (((unwrapped0 t1) (strip-outer-code-fence raw-trimmed '())))
      (when (string=? unwrapped0 "")
        (error "search query empty after stripping code fence"))
      (let ((unwrapped
             (if (and (not (char=? (string-ref unwrapped0 0) #\())
                      (bare-admitted-search-head? unwrapped0))
               (string-append "(" unwrapped0 ")")
               unwrapped0)))
        (unless (char=? (string-ref unwrapped 0) #\()
          (error "search DSL requires a native S-expression: (find-impact ...), (find-collisions ...), (find-callers ...), (ascent-fragment ...), (solve-placement ...), or (search-choice-program ...); legacy @lang:/@sym: syntax is retired" unwrapped))
        (let-values (((balanced t2) (balance-sexp-parens unwrapped t1)))
          (parameterize ((current-readtable
                          (readtable-sharing-allowed?-set
                           (readtable-eval-allowed?-set (current-readtable) #f) #f)))
            (call-with-input-string balanced
              (lambda (port)
                (let ((form (read port))
                      (trailing (read port)))
                  (unless (and (pair? form) (list? form) (eof-object? trailing))
                    (error "expected a single well-formed :gerbil-ascent S-expression" form))
                  (let ((nodes 0))
                    (def (visit value depth)
                      (set! nodes (+ nodes 1))
                      (when (or (> nodes +max-dsl-nodes+) (> depth +max-dsl-depth+))
                        (error "ASCENT query AST bound exceeded"))
                      (when (pair? value)
                        (visit (car value) (+ depth 1))
                        (visit (cdr value) depth)))
                    (visit form 0))
                  (values form t2))))))))))

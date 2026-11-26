;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Safe Pre-Tool-Call Model AST Repair Hook (`scheme/search-dsl/repair.ss`).
;;;
;;; LLMs write valid native `:gerbil-ascent` S-expressions ~90-95% of the time,
;;; occasionally slipping on deterministic syntax details (markdown code fences,
;;; 1-6 omitted trailing `)`, `<-` / `:-` arrow typos, omitting column predicate
;;; annotations, omitting head `(relation answer ...)` declarations, or omitting
;;; `(limits ...)` in Dusa choice programs).
;;;
;;; This pure-functional, zero-`eval`, AOT `linked-ui`-safe hook heals ONLY
;;; unambiguous deterministic syntax slips using the POO `SearchLanguageRegistry.`
;;; relation type signatures, records every applied repair as an auditable symbol
;;; tag, and ALWAYS fails closed on:
;;;   - legacy `@lang:` / `@sym:` / `@graph:` strings or non-S-expressions,
;;;   - read-time evaluation `#.(...)`,
;;;   - unknown top-level forms (e.g. `(search ...)`, `(eval ...)`),
;;;   - unterminated strings or structurally ambiguous parentheses,
;;;   - arbitrary Scheme host callbacks (rejected downstream by `ascent-closed-dsl-parts`).
(import (only-in :clan/poo/object .get)
        (only-in :gerbil/runtime/gambit
                 current-readtable
                 readtable-eval-allowed?-set
                 readtable-sharing-allowed?-set)
        (only-in :std/string/misc string-trim)
        "./objects.ss")
(export repair-and-read-ascent-query
        repair-ascent-ast)

(def +max-dsl-bytes+ 65536)
(def +max-dsl-nodes+ 4096)
(def +max-dsl-depth+ 64)
(def +max-close-parens-repair+ 6)
(def +max-trim-parens-repair+ 4)
(def +admitted-top-heads+ '(ascent ascent-fragment relational-choice-program search-selector))
(def +closed-column-predicates+ '(symbol? integer? exact-integer? boolean? string?))
(def +numeric-guard-ops+ '(< <= > >= = + - * /))

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

;;; Stage A.1: Safely strip a single outer markdown code fence (```scheme ... ```
;;; or single backticks `(...)`). Never coerces non-S-expression prose.
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

;;; Stage A.2: String-, char-literal-, and comment-aware parenthesis balancer.
;;; Safely closes up to `+max-close-parens-repair+` missing trailing `)` or trims
;;; up to `+max-trim-parens-repair+` stray extra trailing `)`. Fails closed on
;;; unterminated strings or multiple/ambiguous top-level forms.
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

;;; ============================================================================
;;; Stage B: Pure Scheme AST Structural Repair Guided by POO Registry Schemas
;;; ============================================================================

(def (build-registry-signature-table registry)
  (let ((table (make-hash-table-eq)))
    (hash-put! table 'file '(integer? string? string?))
    (hash-put! table 'entity '(string? integer? string? string? string? string? integer?))
    (hash-put! table 'edge '(string? string? string? string? integer?))
    (hash-put! table 'attr '(string? string? string?))
    (for-each
      (lambda (meta)
        (let ((rel-name (car meta))
              (lang (cadr meta)))
          (unless (string=? lang "edb")
            (hash-put! table rel-name '(string? integer? string? integer?)))))
      (.get registry relation-meta))
    table))

(def (typed-column-spec? item)
  (and (pair? item)
       (list? item)
       (= (length item) 2)
       (symbol? (car item))
       (symbol? (cadr item))
       (memq (cadr item) +closed-column-predicates+)
       #t))

(def (rule-clause? clause)
  (and (pair? clause)
       (pair? (car clause))
       (symbol? (caar clause))
       (pair? (cdr clause))
       (memq (cadr clause) '(<-- <- :-))
       #t))

(def (normalize-rule-arrows clauses tags)
  (let loop ((rest clauses) (acc '()) (t tags))
    (if (null? rest)
      (values (reverse acc) t)
      (let ((c (car rest)))
        (if (and (rule-clause? c) (not (eq? (cadr c) '<--)))
          (loop (cdr rest)
                (cons (cons (car c) (cons '<-- (cddr c))) acc)
                (add-repair-tag t 'normalize-rule-arrow))
          (loop (cdr rest) (cons c acc) t))))))

(def (record-numeric-guard-vars! expr var-types)
  (when (and (pair? expr) (symbol? (car expr)))
    (when (memq (car expr) +numeric-guard-ops+)
      (for-each
        (lambda (arg)
          (cond
            ((and (symbol? arg) (not (eq? arg '_)))
             (hash-put! var-types arg 'integer?))
            ((pair? arg)
             (record-numeric-guard-vars! arg var-types))))
        (cdr expr)))))

(def (infer-arg-predicate arg var-types)
  (cond
    ((exact-integer? arg) 'integer?)
    ((string? arg) 'string?)
    ((boolean? arg) 'boolean?)
    ((and (symbol? arg) (hash-get var-types arg)) => (lambda (p) p))
    (else 'string?)))

(def (infer-rule-head-signature rule sig-table)
  (let ((var-types (make-hash-table-eq))
        (head (car rule))
        (body (cddr rule)))
    (for-each
      (lambda (atom)
        (cond
          ((and (pair? atom) (eq? (car atom) 'if) (>= (length atom) 3))
           (record-numeric-guard-vars! (caddr atom) var-types))
          ((and (pair? atom) (symbol? (car atom)))
           (let ((sig (hash-get sig-table (car atom))))
             (when sig
               (let loop ((args (cdr atom)) (preds sig))
                 (when (and (pair? args) (pair? preds))
                   (let ((a (car args))
                         (p (car preds)))
                     (when (and (symbol? a) (not (eq? a '_)) (not (hash-get var-types a)))
                       (hash-put! var-types a p)))
                   (loop (cdr args) (cdr preds)))))))))
      body)
    (map (lambda (h-arg) (infer-arg-predicate h-arg var-types)) (cdr head))))

(def (make-column-name arg idx used)
  (let ((candidate
          (if (and (symbol? arg)
                   (not (eq? arg '_))
                   (not (hash-get used arg)))
            arg
            (string->symbol
              (call-with-output-string
                (lambda (p) (display "c" p) (display idx p)))))))
    (hash-put! used candidate #t)
    candidate))

(def (synthesize-relation-decl rel-name head-args preds)
  (let ((used (make-hash-table-eq)))
    (let loop ((args head-args) (ps preds) (idx 0) (acc '()))
      (if (or (null? args) (null? ps))
        (list 'relation rel-name (reverse acc))
        (let ((col (make-column-name (car args) idx used)))
          (loop (cdr args) (cdr ps) (+ idx 1) (cons (list col (car ps)) acc)))))))

(def (repair-ascent-clauses clauses registry tags)
  (let* ((sig-table (build-registry-signature-table registry))
         (registry-rels (make-hash-table-eq)))
    (hash-for-each (lambda (k _v) (hash-put! registry-rels k #t)) sig-table)
    ;; Seed signatures from any already-typed `(relation ...)` declarations.
    (for-each
      (lambda (c)
        (when (and (pair? c) (eq? (car c) 'relation) (>= (length c) 3)
                   (symbol? (cadr c)) (list? (caddr c))
                   (andmap typed-column-spec? (caddr c)))
          (hash-put! sig-table (cadr c) (map cadr (caddr c)))))
      clauses)
    ;; Two fixed-point inference sweeps over rules handle chained/recursive derived relations.
    (let ((inferred-heads (make-hash-table-eq))
          (head-order '()))
      (for-each
        (lambda (_pass)
          (for-each
            (lambda (c)
              (when (and (rule-clause? c) (eq? (cadr c) '<--))
                (let* ((head (car c))
                       (h-name (car head))
                       (h-args (cdr head))
                       (preds (infer-rule-head-signature c sig-table)))
                  (unless (hash-get sig-table h-name)
                    (hash-put! sig-table h-name preds))
                  (unless (hash-get inferred-heads h-name)
                    (set! head-order (cons h-name head-order)))
                  (hash-put! inferred-heads h-name (cons h-args preds)))))
            clauses))
        '(1 2))
      ;; 1. Upgrade any untyped `(relation name (col ...))` declarations.
      (let ((declared (make-hash-table-eq)))
        (let loop ((rest clauses) (acc '()) (t tags))
          (if (null? rest)
            ;; 2. Synthesize missing `(relation head ...)` declarations for undeclared rule heads.
            (let ((synth-decls '())
                  (t-final t))
              (for-each
                (lambda (h-name)
                  (when (and (not (hash-get declared h-name))
                             (not (hash-get registry-rels h-name)))
                    (let* ((info (hash-ref inferred-heads h-name))
                           (decl (synthesize-relation-decl h-name (car info) (cdr info))))
                      (set! synth-decls (append synth-decls (list decl)))
                      (set! t-final (add-repair-tag t-final 'synthesize-head-relation)))))
                (reverse head-order))
              (values (append synth-decls (reverse acc)) t-final))
            (let ((c (car rest)))
              (if (and (pair? c) (eq? (car c) 'relation) (= (length c) 3)
                       (symbol? (cadr c)) (list? (caddr c)))
                (let* ((rel-name (cadr c))
                       (cols (caddr c))
                       (inferred-sig (hash-get sig-table rel-name)))
                  (hash-put! declared rel-name #t)
                  (if (andmap typed-column-spec? cols)
                    (loop (cdr rest) (cons c acc) t)
                    (let ((upgraded
                            (let col-loop ((cs cols) (ps (or inferred-sig '())) (out '()))
                              (if (null? cs)
                                (reverse out)
                                (let* ((col (car cs))
                                       (pred (if (pair? ps) (car ps) 'string?))
                                       (next-ps (if (pair? ps) (cdr ps) '())))
                                  (cond
                                    ((typed-column-spec? col)
                                     (col-loop (cdr cs) next-ps (cons col out)))
                                    ((symbol? col)
                                     (col-loop (cdr cs) next-ps (cons (list col pred) out)))
                                    (else
                                     (error "invalid column declaration in (relation ...)" col))))))))
                      (loop (cdr rest)
                            (cons (list 'relation rel-name upgraded) acc)
                            (add-repair-tag t 'infer-column-predicates)))))
                (loop (cdr rest) (cons c acc) t)))))))))

(def (repair-choice-clauses clauses tags)
  (let ((has-limits? (ormap (lambda (c) (and (pair? c) (eq? (car c) 'limits))) clauses)))
    (if has-limits?
      (values clauses tags)
      (values (append clauses '((limits 4096 4096 4096)))
              (add-repair-tag tags 'append-default-limits)))))

;;; Pure AST-to-AST safe repair pass over an already-read Scheme datum.
;;; Returns `(values repaired-form repair-tags)`.
(def (repair-ascent-ast form (registry SearchLanguageRegistryValue) (initial-tags '()))
  (let-values (((f0 t0)
                (if (rule-clause? form)
                  (values (list 'ascent-fragment form)
                          (add-repair-tag initial-tags 'wrap-bare-rule))
                  (values form initial-tags))))
    (unless (and (pair? f0)
                 (list? f0)
                 (memq (car f0) +admitted-top-heads+))
      (error "expected one native :gerbil-ascent form: (ascent-fragment ...), (ascent ...), or (relational-choice-program ...)" f0))
    (case (car f0)
      ((ascent ascent-fragment)
       (let*-values (((c1 t1) (normalize-rule-arrows (cdr f0) t0))
                     ((c2 t2) (repair-ascent-clauses c1 registry t1)))
         (values (cons (car f0) c2) t2)))
      ((relational-choice-program)
       (let-values (((c1 t1) (repair-choice-clauses (cdr f0) t0)))
         (values (cons 'relational-choice-program c1) t1)))
      ((search-selector)
       (values f0 t0))
      (else (error "unsupported top-level search form" (car f0))))))

;;; Safe bounded pre-tool-call read + repair hook.
;;; Returns `(values repaired-form repair-tags)`.
(def (repair-and-read-ascent-query text (registry SearchLanguageRegistryValue))
  (unless (string? text)
    (error "search query must be a string"))
  (let ((raw-trimmed (string-trim text)))
    (when (or (string=? raw-trimmed "")
              (> (u8vector-length (string->utf8 raw-trimmed)) +max-dsl-bytes+))
      (error "search query empty or exceeds byte bound"))
    (let-values (((unwrapped t1) (strip-outer-code-fence raw-trimmed '())))
      (when (string=? unwrapped "")
        (error "search query empty after stripping code fence"))
      (unless (char=? (string-ref unwrapped 0) #\()
        (error "workspace_search requires a native :gerbil-ascent S-expression: (ascent-fragment ...), (ascent ...), or (relational-choice-program ...); legacy @lang:/@sym: syntax is retired" unwrapped))
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
                (repair-ascent-ast form registry t2)))))))))

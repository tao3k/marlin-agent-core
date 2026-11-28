;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Dusa POPL 2025 Finite-Choice AST Repair, Validation & Canonical Formatter
;;; (`scheme/repair/choice.ss`).
(import "./objects.ss"
        "./reader.ss"
        "./diagnostics.ss")
(export repair-choice-clauses
        format-canonical-search-form)

(def (choice-var-like? sym)
  (and (symbol? sym)
       (let ((s (symbol->string sym)))
         (and (> (string-length s) 0) (char=? (string-ref s 0) #\?)))))

;;; Normalize bare `'sym` literals (and bare non-`?` symbols when `allow-bare-sym?`)
;;; in Dusa choice rules into `(value 'sym)` as silent first-class surface syntax.
(def (normalize-choice-term term tags (allow-bare-sym? #f))
  (cond
    ((and (pair? term) (eq? (car term) 'quote) (= (length term) 2))
     (values (list 'value term) tags))
    ((and allow-bare-sym? (symbol? term) (not (choice-var-like? term)))
     (values (list 'value (list 'quote term)) tags))
    (else (values term tags))))

(def (normalize-choice-terms terms tags (allow-bare-sym? #f))
  (let loop ((rest terms) (acc '()) (t tags))
    (if (null? rest)
      (values (reverse acc) t)
      (let-values (((nt t1) (normalize-choice-term (car rest) t allow-bare-sym?)))
        (loop (cdr rest) (cons nt acc) t1)))))

(def (normalize-choice-atom atom sig-table tags)
  (if (not (and (pair? atom) (symbol? (car atom))))
    (values atom tags)
    (let*-values (((norm-args t1) (normalize-choice-terms (cdr atom) tags #f))
                  ((padded t2) (pad-atom-wildcards (cons (car atom) norm-args) sig-table '?_ t1)))
      (values padded t2))))

(def (make-choice-col-names count)
  (let loop ((i 0) (acc '()))
    (if (= i count)
      (reverse acc)
      (loop (+ i 1)
            (cons (string->symbol (string-append "c" (number->string i))) acc)))))

(def (repair-choice-clauses clauses registry tags)
  (let ((sig-table (build-registry-signature-table registry))
        (choice-rule-arities (make-hash-table-eq))
        (plain-rule-arities (make-hash-table-eq))
        (declared-choices (make-hash-table-eq))
        (declared-rels (make-hash-table-eq)))
    (for-each
      (lambda (c)
        (when (and (pair? c) (eq? (car c) 'rule) (>= (length c) 2))
          (let ((head (cadr c)))
            (cond
              ((and (pair? head) (memq (car head) '(open closed)))
               (unless (and (= (length head) 3)
                            (pair? (cadr head))
                            (list? (cadr head))
                            (>= (length (cadr head)) 2)
                            (symbol? (caadr head)))
                 (raise-repair-diagnostic
                   'invalid_dusa_rule_head
                   "malformed Dusa choice rule head"
                   "(rule (closed (attr ?k) (v1 v2)) body...) or (rule (open (attr ?k) ?v) body...)"
                   "(search-choice-program (use rust) (scope path-prefix: \"crates/marlin-agent-search-engine/src/\" limit: 6) (choice tier (m lvl)) (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m)) (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m provider)) (solve tier state-budget: 64 provenance?: #t))"))
               (when (and (eq? (car head) 'open)
                          (pair? (caddr head))
                          (not (memq (caaddr head) '(value quote))))
                 (raise-repair-diagnostic
                   'invalid_dusa_open_value
                   "Dusa (open (attr ?k) ?v) requires a single scalar/symbol value, not a tuple"
                   "(rule (open (attr ?k) ?v) body...)"
                   "Pass one value variable ?v (or use L1 (ascent-fragment ...) for multi-column tuples)."))
               (let* ((attr (caadr head))
                      (arity (+ 1 (length (cdadr head)))))
                 (unless (hash-get choice-rule-arities attr)
                   (hash-put! choice-rule-arities attr arity))))
              ((and (pair? head) (list? head) (symbol? (car head)))
               (unless (hash-get plain-rule-arities (car head))
                 (hash-put! plain-rule-arities (car head) (length (cdr head)))))))))
      clauses)
    (let-values (((fixed-decls t-decl)
                  (let loop ((rest clauses) (acc '()) (t tags))
                    (if (null? rest)
                      (values (reverse acc) t)
                      (let ((c (car rest)))
                        (cond
                          ((and (pair? c) (eq? (car c) 'choice) (>= (length c) 3)
                                (symbol? (cadr c)) (list? (caddr c)))
                           (let* ((name (cadr c))
                                  (cols (caddr c))
                                  (inferred (hash-get choice-rule-arities name))
                                  (expected (or inferred (length cols))))
                             (hash-put! declared-choices name #t)
                             (cond
                               ((and inferred (not (= (length cols) inferred)) (>= inferred 2))
                                (hash-put! sig-table name (make-list inferred 'symbol?))
                                (loop (cdr rest)
                                      (cons (list 'choice name (make-choice-col-names inferred)) acc)
                                      (add-repair-tag t 'infer-choice-arity)))
                               ((< expected 2)
                                (raise-repair-diagnostic
                                  'invalid_dusa_choice
                                  "Dusa (choice ...) requires at least 2 columns: (key-col ... val-col)"
                                  "(choice attr (key val)) or (choice attr (k1 k2 val))"
                                  "(choice tier (m lvl))"))
                               (else
                                (hash-put! sig-table name (make-list (length cols) 'symbol?))
                                (loop (cdr rest) (cons c acc) t)))))
                          ((and (pair? c) (eq? (car c) 'relation) (>= (length c) 3)
                                (symbol? (cadr c)) (list? (caddr c)))
                           (hash-put! declared-rels (cadr c) #t)
                           (hash-put! sig-table (cadr c) (make-list (length (caddr c)) 'symbol?))
                           (loop (cdr rest) (cons c acc) t))
                          ((scope-clause? c)
                           (let-values (((nc nt) (normalize-scope-clause c t)))
                             (loop (cdr rest) (cons nc acc) nt)))
                          (else
                           (loop (cdr rest) (cons c acc) t))))))))
      (let ((synth-decls '())
            (t-synth t-decl))
        (hash-for-each
          (lambda (attr arity)
            (unless (hash-get declared-choices attr)
              (hash-put! sig-table attr (make-list arity 'symbol?))
              (set! synth-decls (append synth-decls (list (list 'choice attr (make-choice-col-names arity)))))
              (set! t-synth (add-repair-tag t-synth 'synthesize-choice-decl))))
          choice-rule-arities)
        (hash-for-each
          (lambda (rel arity)
            (unless (or (hash-get declared-rels rel) (hash-get sig-table rel))
              (hash-put! sig-table rel (make-list arity 'symbol?))
              (set! synth-decls (append synth-decls (list (list 'relation rel (make-choice-col-names arity)))))
              (set! t-synth (add-repair-tag t-synth 'synthesize-choice-helper-relation))))
          plain-rule-arities)
        (let loop ((rest fixed-decls) (acc '()) (t t-synth) (has-limits? #f))
          (if (null? rest)
            (let ((out (append synth-decls (reverse acc))))
              (if has-limits?
                (values out t)
                (values (append out '((limits 4096 4096 4096))) t)))
            (let ((c (car rest)))
              (cond
                ((and (pair? c) (eq? (car c) 'limits))
                 (loop (cdr rest) (cons c acc) t #t))
                ((and (pair? c) (memq (car c) '(use scope solve)))
                 (loop (cdr rest) (cons c acc) t has-limits?))
                ((and (pair? c) (eq? (car c) 'rule) (>= (length c) 2))
                 (let* ((head (cadr c))
                        (body (cddr c)))
                   (let-values (((new-head t1)
                                 (cond
                                   ((and (pair? head) (eq? (car head) 'closed) (= (length head) 3)
                                         (list? (caddr head)))
                                    (let-values (((nvals tv) (normalize-choice-terms (caddr head) t #t)))
                                      (values (list 'closed (cadr head) nvals) tv)))
                                   ((and (pair? head) (eq? (car head) 'open) (= (length head) 3))
                                    (let-values (((nval tv) (normalize-choice-term (caddr head) t #t)))
                                      (values (list 'open (cadr head) nval) tv)))
                                   (else (values head t)))))
                     (let body-loop ((bs body) (b-acc '()) (bt t1))
                       (if (null? bs)
                         (let ((repaired-body (reverse b-acc)))
                           (for-each
                             (lambda (atom)
                               (when (and (pair? atom) (symbol? (car atom)))
                                 (cond
                                   ((memq (car atom) +comparison-atom-ops+)
                                    (raise-repair-diagnostic
                                      'unsupported_dusa_guard
                                      (string-append "comparison guard '" (symbol->string (car atom)) "' is not supported in Dusa (search-choice-program ...)")
                                      "Dusa choice rule bodies accept positive EDB/built-in/choice atoms only (no != / < / = guards)."
                                      "Use L1 (ascent-fragment ...) for != / < relational joins, and L2 (search-choice-program ...) for finite-choice disambiguation (closed/open/forbid)."))
                                   ((not (hash-get sig-table (car atom)))
                                    (raise-unknown-relation-error (car atom) sig-table))
                                   ((> (length (cdr atom)) (length (hash-ref sig-table (car atom))))
                                    (raise-arity-mismatch-error
                                      (car atom) (hash-ref sig-table (car atom))
                                      (length (cdr atom)) sig-table)))))
                             repaired-body)
                           (loop (cdr rest)
                                 (cons (cons 'rule (cons new-head repaired-body)) acc)
                                 bt has-limits?))
                         (let-values (((na nt) (normalize-choice-atom (car bs) sig-table bt)))
                           (body-loop (cdr bs) (cons na b-acc) nt)))))))
                (else
                 (loop (cdr rest) (cons c acc) t has-limits?))))))))))

(def (strip-internal-choice-boilerplate form)
  (if (and (pair? form) (memq (car form) '(search-choice-program relational-choice-program)))
    (cons (car form)
          (filter (lambda (c) (not (equal? c '(limits 4096 4096 4096))))
                  (cdr form)))
    form))

(def (format-canonical-search-form form)
  (call-with-output-string
    (lambda (p) (write (strip-internal-choice-boilerplate form) p))))

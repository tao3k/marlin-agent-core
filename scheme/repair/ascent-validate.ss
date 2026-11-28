;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Closed `:gerbil-ascent` Validation, Safety Checks & Relation Signature
;;; Synthesis (`scheme/repair/ascent-validate.ss`).
(import "./objects.ss"
        "./reader.ss"
        "./diagnostics.ss")
(export rule-clause?
        literal-datum?
        collect-expr-vars
        repair-ascent-clauses)

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

(def (literal-datum? x)
  (or (string? x) (exact-integer? x) (boolean? x)))

(def (collect-expr-vars expr acc)
  (cond
    ((symbol? expr)
     (if (or (eq? expr '_) (memq expr +guard-expr-ops+) (memq expr acc))
       acc
       (append acc (list expr))))
    ((and (pair? expr) (eq? (car expr) 'quote)) acc)
    ((pair? expr)
     (let loop ((rest (cdr expr)) (vars acc))
       (if (null? rest)
         vars
         (loop (cdr rest) (collect-expr-vars (car rest) vars)))))
    (else acc)))

(def (pad-ascent-body-wildcards clauses sig-table tags)
  (let loop ((rest clauses) (acc '()) (t tags))
    (if (null? rest)
      (values (reverse acc) t)
      (let ((c (car rest)))
        (if (and (rule-clause? c) (eq? (cadr c) '<--))
          (let body-loop ((bs (cddr c)) (b-acc '()) (bt t))
            (if (null? bs)
              (loop (cdr rest) (cons (cons (car c) (cons '<-- (reverse b-acc))) acc) bt)
              (let-values (((nb nt) (pad-atom-wildcards (car bs) sig-table '_ bt)))
                (body-loop (cdr bs) (cons nb b-acc) nt))))
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
                   (let ((a (car args)) (p (car preds)))
                     (when (and (symbol? a) (not (eq? a '_)) (not (hash-get var-types a)))
                       (hash-put! var-types a p)))
                   (loop (cdr args) (cdr preds)))))))))
      body)
    (map (lambda (h-arg) (infer-arg-predicate h-arg var-types)) (cdr head))))

(def (make-column-name arg idx used)
  (let ((candidate
          (if (and (symbol? arg) (not (eq? arg '_)) (not (hash-get used arg)))
            arg
            (string->symbol
              (call-with-output-string (lambda (p) (display "c" p) (display idx p)))))))
    (hash-put! used candidate #t)
    candidate))

(def (synthesize-relation-decl rel-name head-args preds)
  (let ((used (make-hash-table-eq)))
    (let loop ((args head-args) (ps preds) (idx 0) (acc '()))
      (if (or (null? args) (null? ps))
        (list 'relation rel-name (reverse acc))
        (let ((col (make-column-name (car args) idx used)))
          (loop (cdr args) (cdr ps) (+ idx 1) (cons (list col (car ps)) acc)))))))

(def (check-atom-relation-and-arity! rel-sym args known-table)
  (let ((sig (hash-get known-table rel-sym)))
    (cond
      ((not sig) (raise-unknown-relation-error rel-sym known-table))
      ((> (length args) (length sig))
       (raise-arity-mismatch-error rel-sym sig (length args) known-table))
      (else #t))))

(def (validate-closed-guard-expr! expr inputs)
  (cond
    ((symbol? expr)
     (unless (memq expr inputs)
       (raise-repair-diagnostic
         'unbound_guard_variable
         (string-append "guard variable '" (symbol->string expr) "' is not listed in (if (vars...) expr) input list")
         "(if (var1 var2) (not (equal? var1 var2)))"
         "Include every variable referenced in the guard expression inside the (if (vars...) ...) input list.")))
    ((or (literal-datum? expr) (and (pair? expr) (eq? (car expr) 'quote))) #t)
    ((pair? expr)
     (let ((op (car expr)) (operands (cdr expr)))
       (unless (and (symbol? op) (memq op +closed-guard-ops+))
         (raise-repair-diagnostic
           'unsupported_guard_operator
           (string-append "operator '" (if (symbol? op) (symbol->string op) "?")
                          "' is not allowed in closed :gerbil-ascent guards")
           "Allowed closed guard operators: equal?, not, and, or, <, <=, >, >=, =, +, -, *, /, even?, odd?, zero?, positive?, negative?"
           "For path filtering use path_has_prefix(p, \"dir/\"), path_contains(p, \"sub\"), is_test_path(p), or prod_path(p); for regex matching use L0 (brute-search \"re:...\" path-prefix: \"...\")."))
       (for-each (lambda (sub) (validate-closed-guard-expr! sub inputs)) operands)))
    (else #t)))

(def (validate-ascent-body-relations! clauses known-table)
  (for-each
    (lambda (c)
      (when (and (rule-clause? c) (eq? (cadr c) '<--))
        (for-each
          (lambda (atom)
            (when (and (pair? atom) (symbol? (car atom)))
              (let ((head (car atom)))
                (cond
                  ((and (eq? head 'not) (= (length atom) 2)
                        (pair? (cadr atom)) (symbol? (caadr atom)))
                   (let ((inner (cadr atom)))
                     (unless (memq (car inner) +ascent-body-builtins+)
                       (check-atom-relation-and-arity! (car inner) (cdr inner) known-table))))
                  ((and (eq? head 'if) (= (length atom) 3) (list? (cadr atom)))
                   (validate-closed-guard-expr! (caddr atom) (cadr atom)))
                  ((memq head +ascent-body-builtins+) #t)
                  (else (check-atom-relation-and-arity! head (cdr atom) known-table))))))
          (cddr c))))
    clauses))

(def (inline-unbound-head-constants-from-body clauses sig-table tags)
  (let loop ((rest clauses) (acc '()) (t tags))
    (if (null? rest)
      (values (reverse acc) t)
      (let ((c (car rest)))
        (if (not (and (rule-clause? c) (eq? (cadr c) '<--)))
          (loop (cdr rest) (cons c acc) t)
          (let ((bound (make-hash-table-eq))
                (const-by-alias (make-hash-table-eq))
                (head (car c))
                (body (cddr c)))
            (for-each
              (lambda (atom)
                (when (and (pair? atom) (symbol? (car atom))
                           (not (memq (car atom) +ascent-body-builtins+)))
                  (let ((aliases-list (column-aliases-for-relation (car atom) sig-table)))
                    (let col-loop ((args (cdr atom)) (al aliases-list))
                      (when (pair? args)
                        (let ((a (car args))
                              (names (if (pair? al) (car al) '())))
                          (cond
                            ((and (symbol? a) (not (eq? a '_))) (hash-put! bound a #t))
                            ((literal-datum? a)
                             (for-each (lambda (nm)
                                         (unless (hash-get const-by-alias nm)
                                           (hash-put! const-by-alias nm a)))
                                       names)))
                          (col-loop (cdr args) (if (pair? al) (cdr al) '()))))))))
              body)
            (let arg-loop ((h-args (cdr head)) (h-acc '()) (ht t))
              (if (null? h-args)
                (loop (cdr rest) (cons (cons (cons (car head) (reverse h-acc)) (cons '<-- body)) acc) ht)
                (let ((h (car h-args)))
                  (if (and (symbol? h) (not (eq? h '_)) (not (hash-get bound h))
                           (hash-get const-by-alias h))
                    (arg-loop (cdr h-args)
                              (cons (hash-ref const-by-alias h) h-acc)
                              (add-repair-tag ht 'inline-head-constant-from-body))
                    (arg-loop (cdr h-args) (cons h h-acc) ht)))))))))))

(def (sanitize-ascent-negation-vars clauses tags)
  (let loop ((rest clauses) (acc '()) (t tags))
    (if (null? rest)
      (values (reverse acc) t)
      (let ((c (car rest)))
        (if (not (and (rule-clause? c) (eq? (cadr c) '<--)))
          (loop (cdr rest) (cons c acc) t)
          (let ((pos-bound (make-hash-table-eq))
                (outside-used (make-hash-table-eq))
                (neg-counts (make-hash-table-eq))
                (head (car c))
                (body (cddr c)))
            (for-each (lambda (h) (when (and (symbol? h) (not (eq? h '_))) (hash-put! outside-used h #t)))
                      (cdr head))
            (for-each
              (lambda (atom)
                (cond
                  ((and (pair? atom) (eq? (car atom) 'not) (= (length atom) 2)
                        (pair? (cadr atom)) (symbol? (caadr atom)))
                   (for-each (lambda (arg)
                               (when (and (symbol? arg) (not (eq? arg '_)))
                                 (hash-put! neg-counts arg (+ 1 (hash-ref neg-counts arg 0)))))
                             (cdadr atom)))
                  ((and (pair? atom) (eq? (car atom) 'if) (>= (length atom) 3))
                   (for-each (lambda (v) (hash-put! outside-used v #t)) (collect-expr-vars (caddr atom) '())))
                  ((and (pair? atom) (symbol? (car atom)) (not (memq (car atom) +ascent-body-builtins+)))
                   (for-each (lambda (arg) (when (and (symbol? arg) (not (eq? arg '_))) (hash-put! pos-bound arg #t)))
                             (cdr atom)))))
              body)
            (let ((clean-body
                    (map (lambda (atom)
                           (if (and (pair? atom) (eq? (car atom) 'not) (= (length atom) 2)
                                    (pair? (cadr atom)) (symbol? (caadr atom)))
                             (let* ((inner (cadr atom))
                                    (rel (car inner))
                                    (new-args
                                      (map (lambda (arg)
                                             (if (and (symbol? arg) (not (eq? arg '_)) (not (hash-get pos-bound arg)))
                                               (if (and (not (hash-get outside-used arg)) (= (hash-ref neg-counts arg 0) 1))
                                                 '_
                                                 (raise-repair-diagnostic
                                                   'unsafe_negation_variable
                                                   (string-append "unsafe ASCENT negation variable '" (symbol->string arg)
                                                                  "' inside (not (" (symbol->string rel) " ...)) is not bound in any positive body atom")
                                                   "Variables inside (not (...)) must either be bound in a positive body atom or written as _."
                                                   (string-append "Bind '" (symbol->string arg) "' in a positive body atom before (not ...), or replace unused columns inside (not ...) with _.")))
                                               arg))
                                           (cdr inner))))
                               (list 'not (cons rel new-args)))
                             atom))
                         body)))
              (loop (cdr rest) (cons (cons head (cons '<-- clean-body)) acc) t))))))))

(def (validate-ascent-head-safety! clauses)
  (for-each
    (lambda (c)
      (when (and (rule-clause? c) (eq? (cadr c) '<--))
        (let ((bound (make-hash-table-eq))
              (head (car c))
              (body (cddr c)))
          (for-each
            (lambda (atom)
              (when (and (pair? atom) (symbol? (car atom))
                         (not (memq (car atom) +ascent-body-builtins+)))
                (for-each (lambda (arg) (when (and (symbol? arg) (not (eq? arg '_))) (hash-put! bound arg #t)))
                          (cdr atom))))
            body)
          (for-each
            (lambda (h-arg)
              (when (and (symbol? h-arg) (not (eq? h-arg '_)) (not (hash-get bound h-arg)))
                (raise-repair-diagnostic
                  'unsafe_head_variable
                  (string-append "unsafe ASCENT head variable '" (symbol->string h-arg) "' is not bound in rule body")
                  "Every symbol variable in ((head v1 v2 ...) <-- body...) must appear in at least one positive body atom."
                  (string-append "Bind '" (symbol->string h-arg) "' in a body atom, remove it from the head, or replace it with a string literal like \"struct\"."))))
            (cdr head)))))
    clauses))

(def (repair-ascent-clauses clauses registry tags)
  (let* ((sig-table (build-registry-signature-table registry))
         (registry-rels (make-hash-table-eq)))
    (hash-for-each (lambda (k _v) (hash-put! registry-rels k #t)) sig-table)
    (for-each
      (lambda (c)
        (when (and (pair? c) (eq? (car c) 'relation) (>= (length c) 3)
                   (symbol? (cadr c)) (list? (caddr c)))
          (if (andmap typed-column-spec? (caddr c))
            (hash-put! sig-table (cadr c) (map cadr (caddr c)))
            (unless (hash-get sig-table (cadr c))
              (hash-put! sig-table (cadr c) (make-list (length (caddr c)) 'string?))))))
      clauses)
    (let*-values (((padded-0 t0) (pad-ascent-body-wildcards clauses sig-table tags))
                  ((padded-1 t1) (inline-unbound-head-constants-from-body padded-0 sig-table t0))
                  ((padded t-pad) (sanitize-ascent-negation-vars padded-1 t1)))
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
                    (hash-put! sig-table h-name preds)
                    (unless (hash-get inferred-heads h-name)
                      (set! head-order (cons h-name head-order)))
                    (hash-put! inferred-heads h-name (cons h-args preds)))))
              padded))
          '(1 2))
        (validate-ascent-body-relations! padded sig-table)
        (validate-ascent-head-safety! padded)
        (let ((declared (make-hash-table-eq)))
          (let loop ((rest padded) (acc '()) (t t-pad))
            (if (null? rest)
              (let ((synth-decls '()))
                (for-each
                  (lambda (h-name)
                    (when (and (not (hash-get declared h-name))
                               (not (hash-get registry-rels h-name)))
                      (let* ((info (hash-ref inferred-heads h-name))
                             (decl (synthesize-relation-decl h-name (car info) (cdr info))))
                        (set! synth-decls (append synth-decls (list decl))))))
                  (reverse head-order))
                (values (append synth-decls (reverse acc)) t))
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
                  (loop (cdr rest) (cons c acc) t))))))))))

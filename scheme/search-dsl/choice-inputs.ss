;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Dusa POPL 2025 Term Lowering, Provenance Projection, Closed-Forbid Static
;;; Deduction Propagation & Scenario B Semantic Input Materialization
;;; (`scheme/search-dsl/choice-inputs.ss`).
(import (only-in :gerbil-ascent/program/api
                 gerbil-ascent-variable
                 gerbil-ascent-wildcard
                 gerbil-ascent-literal
                 gerbil-ascent-atom
                 relational-program-query)
        (only-in :gerbil-ascent/program/scheme-checked
                 relational-source
                 relational-with-role)
        "./prefilter.ss"
        "./ascent-eval.ss")
(export normalize-choice-scalar
        normalize-choice-rows
        lower-choice-term
        lower-choice-atom
        make-choice-rule-origin
        project-choice-solutions
        bounded-pow
        max-safe-free-vars
        propagate-closed-forbid-clauses
        parse-solve-options
        parse-scope-options
        take-up-to
        collect-choice-referenced-rels
        build-scenario-b-choice-inputs)

(def (normalize-choice-scalar val)
  (cond
    ((string? val) (string->symbol val))
    ((or (exact-integer? val) (boolean? val) (symbol? val) (char? val)) val)
    (else (error "unsupported relational-choice scalar" val))))

(def (normalize-choice-rows rows)
  (map (lambda (row) (map normalize-choice-scalar row)) rows))

(def (choice-var-symbol? x)
  (and (symbol? x)
       (let ((s (symbol->string x)))
         (and (> (string-length s) 1) (char=? (string-ref s 0) #\?)))))

(def (lower-choice-term term)
  (cond
    ((eq? term '?_) (gerbil-ascent-wildcard))
    ((choice-var-symbol? term) (gerbil-ascent-variable term))
    ((and (pair? term) (eq? (car term) 'value) (= (length term) 2))
     (let ((inner (cadr term)))
       (cond
         ((and (pair? inner) (eq? (car inner) 'quote) (= (length inner) 2))
          (gerbil-ascent-literal (normalize-choice-scalar (cadr inner))))
         (else (gerbil-ascent-literal (normalize-choice-scalar inner))))))
    ((and (pair? term) (eq? (car term) 'quote) (= (length term) 2))
     (gerbil-ascent-literal (normalize-choice-scalar (cadr term))))
    ((or (exact-integer? term) (boolean? term) (char? term) (string? term))
     (gerbil-ascent-literal (normalize-choice-scalar term)))
    ((symbol? term)
     (gerbil-ascent-literal term))
    (else (error "invalid relational-choice term (expected ?var, ?_, scalar, or (value 'sym))" term))))

(def (lower-choice-atom atom)
  (unless (and (pair? atom) (symbol? (car atom)))
    (error "invalid relational-choice body atom" atom))
  (gerbil-ascent-atom (car atom) (map lower-choice-term (cdr atom))))

(def (make-choice-rule-origin body-atoms)
  (vector #f (make-list (length body-atoms) #f)))

(def (project-provenance-witness entry)
  (let* ((kind (if (and (pair? entry) (symbol? (car entry))) (car entry) 'derived))
         (fact (if (and (pair? entry) (pair? (cdr entry)) (list? (cadr entry))) (cadr entry) '()))
         (justs (if (and (pair? entry) (pair? (cdr entry)) (pair? (cddr entry)) (list? (caddr entry)))
                  (caddr entry) '()))
         (first-just (if (pair? justs) (car justs) #f))
         (rule-kind (if (and (list? first-just) (>= (length first-just) 2) (symbol? (cadr first-just)))
                      (cadr first-just) 'rule))
         (premises (if (and (list? first-just) (>= (length first-just) 4) (list? (list-ref first-just 3)))
                     (list-ref first-just 3) '()))
         (alts (if (and (list? first-just) (>= (length first-just) 5) (list? (list-ref first-just 4)))
                 (list-ref first-just 4) '())))
    (vector kind fact rule-kind premises alts)))

(def (project-choice-solutions solutions target-rel provenance?)
  (let loop ((rest solutions) (idx 1) (acc '()))
    (if (null? rest)
      (reverse acc)
      (let* ((sol (car rest))
             (bindings (relational-program-query sol target-rel))
             (raw-prov (if provenance?
                         (relational-program-query sol target-rel provenance?: #t)
                         '()))
             (witnesses (map project-provenance-witness raw-prov)))
        (loop (cdr rest)
              (+ idx 1)
              (cons (vector 'choice-world idx bindings witnesses) acc))))))

(def (bounded-pow base exp cap)
  (let loop ((i 0) (acc 1))
    (cond
      ((>= acc cap) cap)
      ((>= i exp) acc)
      (else (loop (+ i 1) (min cap (* acc (max 1 base))))))))

(def (branch-state-tree-size base free-k cap)
  (let ((b (max 2 base)))
    (let loop ((i 0) (term 1) (sum 0))
      (let ((next-sum (min cap (+ sum term))))
        (if (or (>= i free-k) (>= next-sum cap))
          next-sum
          (let ((factor (* (max 1 (- free-k i)) b)))
            (loop (+ i 1) (min cap (* term factor)) next-sum)))))))

(def (max-safe-free-vars base constrained budget)
  (let loop ((k 0) (best 0))
    (let ((needed (+ constrained (branch-state-tree-size base k (+ budget 1)))))
      (if (and (<= needed budget) (< k 12))
        (loop (+ k 1) k)
        best))))

(def (take-up-to lst n)
  (if (or (not n) (null? lst) (<= n 0))
    (if (not n) lst '())
    (cons (car lst) (take-up-to (cdr lst) (- n 1)))))

(def (subst-choice-var tree old-var new-var)
  (cond
    ((eq? tree old-var) new-var)
    ((and (pair? tree) (eq? (car tree) 'quote)) tree)
    ((pair? tree)
     (cons (subst-choice-var (car tree) old-var new-var)
           (subst-choice-var (cdr tree) old-var new-var)))
    (else tree)))

(def (choice-atom-uses-rel? atom choice-names)
  (and (pair? atom) (symbol? (car atom)) (memq (car atom) choice-names) #t))

(def (dedup-sexp-list items)
  (let loop ((rest items) (acc '()))
    (cond
      ((null? rest) (reverse acc))
      ((member (car rest) acc) (loop (cdr rest) acc))
      (else (loop (cdr rest) (cons (car rest) acc))))))

(def (choice-value-term-key term)
  (cond
    ((and (pair? term) (eq? (car term) 'value) (= (length term) 2))
     (let ((inner (cadr term)))
       (if (and (pair? inner) (eq? (car inner) 'quote) (= (length inner) 2))
         (normalize-choice-scalar (cadr inner))
         (normalize-choice-scalar inner))))
    ((and (pair? term) (eq? (car term) 'quote) (= (length term) 2))
     (normalize-choice-scalar (cadr term)))
    ((or (exact-integer? term) (boolean? term) (char? term) (string? term))
     (normalize-choice-scalar term))
    (else term)))

;;; Sound static deduction propagation for Dusa closed-world forbid rules.
(def (propagate-closed-forbid-clauses clauses choice-names)
  (let ((offers '())
        (forbids '()))
    (for-each
      (lambda (c)
        (when (and (pair? c) (eq? (car c) 'rule) (>= (length c) 3))
          (let ((head (cadr c))
                (body (cddr c)))
            (when (and (pair? head) (eq? (car head) 'closed) (= (length head) 3)
                       (pair? (cadr head)) (symbol? (caadr head)) (list? (caddr head)))
              (let ((attr (caadr head)) (keys (cdadr head)) (vals (caddr head)))
                (cond
                  ((and (pair? vals)
                        (not (ormap (lambda (a) (choice-atom-uses-rel? a choice-names)) body)))
                   (set! offers (cons (list attr keys vals body) offers)))
                  ((null? vals)
                   (let* ((choice-atoms (filter (lambda (a) (choice-atom-uses-rel? a choice-names)) body))
                          (prem-atoms (filter (lambda (a) (not (choice-atom-uses-rel? a choice-names))) body)))
                     (when (and (= (length choice-atoms) 1)
                                (eq? (caar choice-atoms) attr)
                                (= (length (cdar choice-atoms)) (+ (length keys) 1)))
                       (let* ((catom-args (cdar choice-atoms))
                              (catom-keys (take-up-to catom-args (length keys)))
                              (bad-val (list-ref catom-args (length keys))))
                         (when (and (equal? catom-keys keys) (not (choice-var-symbol? bad-val)))
                           (set! forbids (cons (list attr keys (choice-value-term-key bad-val) prem-atoms) forbids)))))))))))))
      clauses)
    (let ((entailed '()))
      (for-each
        (lambda (f)
          (let ((f-attr (list-ref f 0)) (f-keys (list-ref f 1))
                (f-bad-key (list-ref f 2)) (f-prems (list-ref f 3)))
            (for-each
              (lambda (o)
                (let ((o-attr (list-ref o 0)) (o-keys (list-ref o 1))
                      (o-vals (list-ref o 2)) (o-body (list-ref o 3)))
                  (when (and (eq? o-attr f-attr)
                             (= (length o-keys) (length f-keys))
                             (ormap (lambda (v) (equal? (choice-value-term-key v) f-bad-key)) o-vals))
                    (let* ((narrowed (filter (lambda (v) (not (equal? (choice-value-term-key v) f-bad-key))) o-vals))
                           (renamed-prems
                             (let rloop ((fk f-keys) (ok o-keys) (ps f-prems))
                               (if (or (null? fk) (null? ok))
                                 ps
                                 (rloop (cdr fk) (cdr ok)
                                        (if (and (symbol? (car fk)) (symbol? (car ok)) (not (eq? (car fk) (car ok))))
                                          (subst-choice-var ps (car fk) (car ok))
                                          ps)))))
                           (joined-body (dedup-sexp-list (append o-body renamed-prems))))
                      (set! entailed
                        (cons (cons 'rule (cons (list 'closed (cons o-attr o-keys) narrowed) joined-body))
                              entailed))))))
              offers)))
        forbids)
      (append clauses (reverse entailed)))))

(def (parse-solve-options opts)
  (let loop ((rest opts) (budget +default-choice-state-budget+) (prov? #t))
    (cond
      ((not (and (pair? rest) (pair? (cdr rest)))) (values budget prov?))
      ((or (equal? (car rest) state-budget:) (eq? (car rest) 'state-budget))
       (let ((v (cadr rest)))
         (loop (cddr rest) (if (exact-integer? v) v budget) prov?)))
      ((or (equal? (car rest) provenance?:) (eq? (car rest) 'provenance?))
       (loop (cddr rest) budget (if (cadr rest) #t #f)))
      (else (loop (cdr rest) budget prov?)))))

(def (parse-scope-options opts)
  (let loop ((rest opts) (prefix #f) (limit #f))
    (cond
      ((not (and (pair? rest) (pair? (cdr rest)))) (values prefix limit))
      ((or (equal? (car rest) path-prefix:) (eq? (car rest) 'path-prefix))
       (loop (cddr rest) (datum->hint-string (cadr rest)) limit))
      ((or (equal? (car rest) limit:) (eq? (car rest) 'limit))
       (let ((v (cadr rest)))
         (loop (cddr rest) prefix (if (and (exact-integer? v) (> v 0)) v limit))))
      (else (loop (cdr rest) prefix limit)))))

(def (collect-choice-referenced-rels clauses target-rel)
  (let ((seen (make-hash-table-eq)))
    (when target-rel (hash-put! seen target-rel #t))
    (for-each
      (lambda (c)
        (when (and (pair? c) (eq? (car c) 'rule))
          (let ((head (cadr c)) (body (cddr c)))
            (cond
              ((and (pair? head) (memq (car head) '(closed open)) (pair? (cadr head)))
               (hash-put! seen (caadr head) #t))
              ((and (pair? head) (symbol? (car head)))
               (hash-put! seen (car head) #t)))
            (for-each
              (lambda (atom)
                (when (and (pair? atom) (symbol? (car atom)))
                  (hash-put! seen (car atom) #t)))
              body))))
      clauses)
    seen))

(def (build-scenario-b-choice-inputs filtered-files filtered-entities filtered-edges attrs needed-table)
  (let* ((file-id->path (make-hash-table))
         (hot-attr-ids (make-hash-table))
         (defs-by-file+name (make-hash-table))
         (out-rels '()))
    (for-each (lambda (f) (hash-put! file-id->path (car f) (caddr f))) filtered-files)
    (for-each
      (lambda (a)
        (when (and (string? (cadr a)) (string=? (cadr a) "hot_path")
                   (string? (caddr a)) (string=? (caddr a) "true"))
          (hash-put! hot-attr-ids (car a) #t)))
      attrs)
    (let ((workspace-def-rows '())
          (in-test-rows '())
          (seen-test (make-hash-table)))
      (for-each
        (lambda (ent)
          (let* ((id (list-ref ent 0)) (fid (list-ref ent 1))
                 (kind (list-ref ent 3)) (name (list-ref ent 5))
                 (path (or (hash-get file-id->path fid) "")))
            (unless (and (string? kind) (string=? kind "import"))
              (set! workspace-def-rows (cons (list name id) workspace-def-rows))
              (let* ((key (cons path name))
                     (prev (or (hash-get defs-by-file+name key) '())))
                (hash-put! defs-by-file+name key (cons id prev)))
              (when (and (path-is-test-file? path) (not (hash-get seen-test id)))
                (hash-put! seen-test id #t)
                (set! in-test-rows (cons (list id) in-test-rows))))))
        filtered-entities)
      (when (hash-get needed-table 'workspace-def)
        (set! out-rels
          (cons (relational-with-role (relational-source 'workspace-def 2 (normalize-choice-rows (reverse workspace-def-rows))) 'input) out-rels)))
      (when (hash-get needed-table 'in-test-file)
        (set! out-rels
          (cons (relational-with-role (relational-source 'in-test-file 1 (normalize-choice-rows (reverse in-test-rows))) 'input) out-rels))))
    (when (or (hash-get needed-table 'ref-at) (hash-get needed-table 'same-module-def))
      (let ((ref-rows '()) (same-rows '()) (seen-ref (make-hash-table)))
        (for-each
          (lambda (e)
            (when (and (string? (caddr e)) (string=? (caddr e) "calls"))
              (let* ((caller (list-ref e 0)) (callee (list-ref e 1))
                     (path (list-ref e 3)) (line (list-ref e 4))
                     (callsite (string-append caller "@L" (number->string line) ":" callee)))
                (unless (hash-get seen-ref callsite)
                  (hash-put! seen-ref callsite #t)
                  (set! ref-rows (cons (list callsite callee) ref-rows))
                  (for-each (lambda (def-id) (set! same-rows (cons (list callsite callee def-id) same-rows)))
                            (or (hash-get defs-by-file+name (cons path callee)) '()))))))
          filtered-edges)
        (when (hash-get needed-table 'ref-at)
          (set! out-rels (cons (relational-with-role (relational-source 'ref-at 2 (normalize-choice-rows (reverse ref-rows))) 'input) out-rels)))
        (when (hash-get needed-table 'same-module-def)
          (set! out-rels (cons (relational-with-role (relational-source 'same-module-def 3 (normalize-choice-rows (reverse same-rows))) 'input) out-rels)))))
    (when (hash-get needed-table 'module-in-scope)
      (set! out-rels
        (cons (relational-with-role (relational-source 'module-in-scope 1 (normalize-choice-rows (map (lambda (f) (list (caddr f))) filtered-files))) 'input) out-rels)))
    (when (hash-get needed-table 'hot-path-module)
      (let ((hot-mods (filter-map (lambda (f) (let ((p (caddr f))) (and (or (path-is-hot-path-module? p) (hash-get hot-attr-ids p)) (list p)))) filtered-files)))
        (set! out-rels (cons (relational-with-role (relational-source 'hot-path-module 1 (normalize-choice-rows hot-mods)) 'input) out-rels))))
    (when (hash-get needed-table 'module-depends)
      (let ((dep-rows (filter-map (lambda (e) (and (string? (caddr e)) (or (string=? (caddr e) "imports") (string=? (caddr e) "calls") (string=? (caddr e) "implements")) (list (list-ref e 3) (cadr e)))) filtered-edges)))
        (set! out-rels (cons (relational-with-role (relational-source 'module-depends 2 (normalize-choice-rows dep-rows)) 'input) out-rels))))
    (when (or (hash-get needed-table 'calls_symbol) (hash-get needed-table 'calls-symbol))
      (let ((id->name (make-hash-table)) (call-sym-rows '()))
        (for-each (lambda (ent) (hash-put! id->name (list-ref ent 0) (list-ref ent 5))) filtered-entities)
        (for-each
          (lambda (e)
            (when (and (string? (caddr e)) (string=? (caddr e) "calls"))
              (let ((caller (or (hash-get id->name (list-ref e 0)) (list-ref e 0)))
                    (callee (or (hash-get id->name (list-ref e 1)) (list-ref e 1))))
                (set! call-sym-rows (cons (list caller callee (list-ref e 3) (list-ref e 4)) call-sym-rows)))))
          filtered-edges)
        (let ((rel-sym (if (hash-get needed-table 'calls-symbol) 'calls-symbol 'calls_symbol)))
          (set! out-rels (cons (relational-with-role (relational-source rel-sym 4 (normalize-choice-rows (reverse call-sym-rows))) 'input) out-rels)))))
    (for-each
      (lambda (spec)
        (let ((dash-sym (car spec)) (under-sym (cadr spec)) (attr-key (caddr spec)))
          (when (or (hash-get needed-table dash-sym) (hash-get needed-table under-sym))
            (let ((rows (filter-map (lambda (a) (and (string? (cadr a)) (string=? (cadr a) attr-key) (list (car a) (caddr a)))) attrs))
                  (rel-sym (if (hash-get needed-table dash-sym) dash-sym under-sym)))
              (set! out-rels (cons (relational-with-role (relational-source rel-sym 2 (normalize-choice-rows rows)) 'input) out-rels))))))
      '((verified-tier verified_tier "verified_tier")
        (verified-resolution verified_resolution "verified_resolution")
        (verified-evidence verified_evidence "verified_evidence")))
    out-rels))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Zero-Eval Upstream Dusa POPL 2025 Finite-Choice Constraint Evaluator
;;; (`scheme/search-dsl/choice-eval.ss`).
(import (only-in :clan/poo/object .get .ref)
        (only-in :std/string/misc string-contains)
        (only-in :gerbil-ascent/program/api
                 gerbil-ascent-variable
                 gerbil-ascent-wildcard
                 gerbil-ascent-literal
                 gerbil-ascent-atom
                 gerbil-ascent-program
                 relational-choice-solve)
        (only-in :gerbil-ascent/program/scheme-checked
                 relational-source
                 relational-with-role
                 relational-rule/reader-origin)
        (only-in :gerbil-ascent/program/scheme-choice
                 make-relational-choice-head
                 lower-relational-choice-program)
        (only-in :marlin-agent-core/repair/funcs format-canonical-search-form)
        "./prefilter.ss"
        "./ascent-eval.ss"
        "./choice-inputs.ss")
(export evaluate-choice-program-over-edb
        evaluate-closed-choice-over-edb)

(def (evaluate-choice-program-over-edb
       choice-prog
       target-rel
       (repairs '())
       (state-budget +default-choice-state-budget+)
       (provenance? #t)
       (canonical-query #f))
  (let ((clamped-budget (max 1 (min 4096 state-budget))))
    (with-catch
      (lambda (err)
        (let ((msg (call-with-output-string (lambda (p) (display-exception err p)))))
          (if (string-contains (string-downcase msg) "budget")
            (vector 'search-result 'choice target-rel
                    (vector 'choice-budget-exhausted clamped-budget msg)
                    repairs
                    canonical-query)
            (raise err))))
      (lambda ()
        (let* ((solutions (relational-choice-solve choice-prog state-budget: clamped-budget))
               (worlds (project-choice-solutions solutions target-rel provenance?)))
          (vector 'search-result 'choice target-rel
                  (vector 'choice-report (length worlds) clamped-budget provenance? worlds)
                  repairs
                  canonical-query))))))

(def (evaluate-closed-choice-over-edb form edb registry repairs)
  (let* ((declared-choice-names
           (filter-map (lambda (c)
                         (and (pair? c) (eq? (car c) 'choice) (>= (length c) 2) (symbol? (cadr c))
                              (cadr c)))
                       (cdr form)))
         (propagated-clauses
           (propagate-closed-forbid-clauses (cdr form) declared-choice-names)))
    (let ((extra-rels '()) (choice-rels '()) (choice-names '())
          (plain-rules '()) (choice-heads '()) (max-branch-factor 2)
          (use-langs '()) (scope-prefix #f) (scope-limit #f)
          (solve-target #f) (solve-budget +default-choice-state-budget+)
          (solve-provenance? #t) (input-limit 65536) (derived-limit 65536) (output-limit 65536))
      (for-each
        (lambda (clause)
          (unless (and (pair? clause) (symbol? (car clause)))
            (error "invalid choice program clause" clause))
          (case (car clause)
            ((use)
             (for-each (lambda (ld) (let ((s (datum->hint-string ld))) (when s (set! use-langs (append-unique use-langs s))))) (cdr clause)))
            ((scope)
             (let-values (((p lim) (parse-scope-options (cdr clause))))
               (when p (set! scope-prefix p))
               (when lim (set! scope-limit lim))))
            ((solve)
             (when (pair? (cdr clause))
               (set! solve-target (cadr clause))
               (let-values (((b prov?) (parse-solve-options (cddr clause))))
                 (set! solve-budget b)
                 (set! solve-provenance? prov?))))
            ((relation)
             (let* ((name (cadr clause)) (cols (caddr clause))
                    (rows (if (and (>= (length clause) 4) (pair? (cadddr clause)) (eq? (car (cadddr clause)) 'quote))
                            (normalize-choice-rows (cadr (cadddr clause))) '())))
               (unless (memq name +edb-relation-names+)
                 (set! extra-rels
                   (append extra-rels
                           (list (relational-with-role (relational-source name (length cols) rows)
                                                       (if (null? rows) 'derived 'input))))))))
            ((choice)
             (let ((name (cadr clause)) (cols (caddr clause)))
               (set! choice-names (append-unique choice-names name))
               (set! choice-rels (append choice-rels (list (relational-with-role (relational-source name (length cols) '()) 'input))))))
            ((rule)
             (let* ((head (cadr clause))
                    (body (map lower-choice-atom (cddr clause)))
                    (origin (make-choice-rule-origin body)))
               (cond
                 ((and (pair? head) (eq? (car head) 'closed))
                  (let* ((target (cadr head)) (vals (map lower-choice-term (caddr head))))
                    (when (> (length vals) max-branch-factor) (set! max-branch-factor (length vals)))
                    (set! choice-heads
                      (append choice-heads
                              (list (make-relational-choice-head 'closed (car target) (map lower-choice-term (cdr target)) vals body origin))))))
                 ((and (pair? head) (eq? (car head) 'open))
                  (let ((target (cadr head)) (val (lower-choice-term (caddr head))))
                    (set! choice-heads
                      (append choice-heads
                              (list (make-relational-choice-head 'open (car target) (map lower-choice-term (cdr target)) (list val) body origin))))))
                 (else
                  (set! plain-rules
                    (append plain-rules (list (relational-rule/reader-origin (list (lower-choice-atom head)) body origin))))))))
            ((limits)
             (when (= (length clause) 4)
               (set! input-limit (min 100000 (cadr clause)))
               (set! derived-limit (min 100000 (caddr clause)))
               (set! output-limit (min 100000 (cadddr clause)))))
            (else (error "unsupported choice program clause" clause))))
        propagated-clauses)
      (unless (pair? choice-names)
        (error "choice program requires at least one (choice ...) declaration"))
      (let* ((target-rel (or solve-target (car (reverse choice-names))))
             (needed (collect-choice-referenced-rels propagated-clauses target-rel))
             (raw-attrs (edb-table-ref edb 'attr))
             (hot-attr-ids (let ((tbl (make-hash-table)))
                             (for-each (lambda (a)
                                         (when (and (string? (cadr a)) (string=? (cadr a) "hot_path")
                                                    (string? (caddr a)) (string=? (caddr a) "true"))
                                           (hash-put! tbl (car a) #t)))
                                       raw-attrs)
                             tbl))
             (is-hot? (lambda (f) (let ((p (caddr f))) (or (path-is-hot-path-module? p) (hash-get hot-attr-ids p)))))
             (raw-files (edb-table-ref edb 'file))
             (filtered-files-0
               (filter (lambda (f)
                         (and (or (null? use-langs) (member (cadr f) use-langs))
                              (or (not scope-prefix) (string-prefix? scope-prefix (caddr f)))))
                       raw-files))
             (filtered-files
               (if (and scope-limit (hash-get needed 'hot-path-module))
                 (let ((hot-files (filter is-hot? filtered-files-0))
                       (free-files (filter (lambda (f) (not (is-hot? f))) filtered-files-0)))
                   (if (null? hot-files)
                     (take-up-to filtered-files-0 scope-limit)
                     (let* ((target-free (if (>= scope-limit 8) 3 2))
                            (free-reserve (min target-free (length free-files) (max 0 (- scope-limit 1))))
                            (chosen-hot (take-up-to hot-files (max 1 (- scope-limit free-reserve))))
                            (rem (max 0 (- scope-limit (length chosen-hot))))
                            (chosen-free (take-up-to free-files rem)))
                       (append chosen-hot chosen-free))))
                 (take-up-to filtered-files-0 scope-limit)))
             (allowed-fids (let ((tbl (make-hash-table))) (for-each (lambda (f) (hash-put! tbl (car f) #t)) filtered-files) tbl))
             (allowed-paths (let ((tbl (make-hash-table))) (for-each (lambda (f) (hash-put! tbl (caddr f) #t)) filtered-files) tbl))
             (filtered-entities
               (filter (lambda (e)
                         (and (or (null? use-langs) (member (list-ref e 2) use-langs))
                              (or (not (or scope-prefix scope-limit)) (hash-get allowed-fids (list-ref e 1)))))
                       (edb-table-ref edb 'entity)))
             (filtered-edges
               (filter (lambda (e)
                         (or (and (null? use-langs) (not scope-prefix) (not scope-limit))
                             (hash-get allowed-paths (list-ref e 3))))
                       (edb-table-ref edb 'edge)))
             (general-ontology-specs
               '((callable . ("function" "method"))
                 (type_decl . ("struct" "class" "enum" "trait" "interface" "type"))
                 (module_decl . ("module")) (import_decl . ("import"))
                 (binding_decl . ("binding" "constant")) (macro_decl . ("macro"))
                 (comment_node . ("doc")) (test_decl . ("test"))))
             (active-general-specs (filter (lambda (s) (hash-get needed (car s))) general-ontology-specs))
             (general-helper-rels (map (lambda (s) (relational-with-role (relational-source (car s) 5 '()) 'derived)) active-general-specs))
             (general-helper-rules
               (apply append
                 (map (lambda (spec)
                        (map (lambda (kind-str)
                               (let ((head (gerbil-ascent-atom (car spec) (list (gerbil-ascent-variable '?id) (gerbil-ascent-variable '?f) (gerbil-ascent-variable '?lang) (gerbil-ascent-variable '?name) (gerbil-ascent-variable '?line))))
                                     (body (list (gerbil-ascent-atom 'entity (list (gerbil-ascent-variable '?id) (gerbil-ascent-variable '?f) (gerbil-ascent-variable '?lang) (gerbil-ascent-literal (string->symbol kind-str)) (gerbil-ascent-wildcard) (gerbil-ascent-variable '?name) (gerbil-ascent-variable '?line))))))
                                 (relational-rule/reader-origin (list head) body (make-choice-rule-origin body))))
                             (cdr spec)))
                      active-general-specs)))
             (graph-edge-specs
               '((contains_edge . contains) (contains-edge . contains)
                 (imports_edge . imports) (imports-edge . imports)
                 (calls_edge . calls) (calls-edge . calls)
                 (implements_edge . implements) (implements-edge . implements)))
             (active-graph-specs (filter (lambda (s) (hash-get needed (car s))) graph-edge-specs))
             (graph-helper-rels
               (append
                 (map (lambda (s) (relational-with-role (relational-source (car s) 4 '()) 'derived)) active-graph-specs)
                 (if (or (hash-get needed 'depends_on) (hash-get needed 'depends-on))
                   (list (relational-with-role (relational-source (if (hash-get needed 'depends-on) 'depends-on 'depends_on) 2 '()) 'derived)) '())
                 (if (or (hash-get needed 'entity_locator) (hash-get needed 'entity-locator))
                   (list (relational-with-role (relational-source (if (hash-get needed 'entity-locator) 'entity-locator 'entity_locator) 2 '()) 'derived)) '())))
             (graph-helper-rules
               (append
                 (map (lambda (s)
                        (let ((head (gerbil-ascent-atom (car s) (list (gerbil-ascent-variable '?src) (gerbil-ascent-variable '?dst) (gerbil-ascent-variable '?path) (gerbil-ascent-variable '?line))))
                              (body (list (gerbil-ascent-atom 'edge (list (gerbil-ascent-variable '?src) (gerbil-ascent-variable '?dst) (gerbil-ascent-literal (cdr s)) (gerbil-ascent-variable '?path) (gerbil-ascent-variable '?line))))))
                          (relational-rule/reader-origin (list head) body (make-choice-rule-origin body))))
                      active-graph-specs)
                 (if (or (hash-get needed 'depends_on) (hash-get needed 'depends-on))
                   (let ((dep-sym (if (hash-get needed 'depends-on) 'depends-on 'depends_on)))
                     (append
                       (map (lambda (rs)
                              (let ((head (gerbil-ascent-atom dep-sym (list (gerbil-ascent-variable '?src) (gerbil-ascent-variable '?dst))))
                                    (body (list (gerbil-ascent-atom 'edge (list (gerbil-ascent-variable '?src) (gerbil-ascent-variable '?dst) (gerbil-ascent-literal rs) (gerbil-ascent-wildcard) (gerbil-ascent-wildcard))))))
                                (relational-rule/reader-origin (list head) body (make-choice-rule-origin body))))
                            '(imports calls implements))
                       (list (let ((head (gerbil-ascent-atom dep-sym (list (gerbil-ascent-variable '?src) (gerbil-ascent-variable '?dst))))
                                   (body (list (gerbil-ascent-atom dep-sym (list (gerbil-ascent-variable '?src) (gerbil-ascent-variable '?mid)))
                                               (gerbil-ascent-atom dep-sym (list (gerbil-ascent-variable '?mid) (gerbil-ascent-variable '?dst))))))
                               (relational-rule/reader-origin (list head) body (make-choice-rule-origin body))))))
                   '())
                 (if (or (hash-get needed 'entity_locator) (hash-get needed 'entity-locator))
                   (let ((loc-sym (if (hash-get needed 'entity-locator) 'entity-locator 'entity_locator)))
                     (list (let ((head (gerbil-ascent-atom loc-sym (list (gerbil-ascent-variable '?id) (gerbil-ascent-variable '?loc))))
                                 (body (list (gerbil-ascent-atom 'attr (list (gerbil-ascent-variable '?id) (gerbil-ascent-literal 'locator) (gerbil-ascent-variable '?loc))))))
                             (relational-rule/reader-origin (list head) body (make-choice-rule-origin body)))))
                   '())))
             (poo-helper-entries
               (filter (lambda (m)
                         (and (hash-get needed (car m))
                              (not (member (cadr m) '("edb" "general" "graph" "graph-pair")))))
                       (.get registry relation-meta)))
             (poo-helper-rels (map (lambda (m) (relational-with-role (relational-source (car m) 4 '()) 'derived)) poo-helper-entries))
             (poo-helper-rules
               (map (lambda (m)
                      (let* ((kind-pat (caddr m)) (lk-pat (cadddr m))
                             (head (gerbil-ascent-atom (car m) (list (gerbil-ascent-variable '?id) (gerbil-ascent-variable '?f) (gerbil-ascent-variable '?name) (gerbil-ascent-variable '?line))))
                             (body (list (gerbil-ascent-atom 'entity
                                           (list (gerbil-ascent-variable '?id) (gerbil-ascent-variable '?f)
                                                 (gerbil-ascent-literal (string->symbol (cadr m)))
                                                 (if (string? kind-pat) (gerbil-ascent-literal (string->symbol kind-pat)) (gerbil-ascent-wildcard))
                                                 (if (string? lk-pat) (gerbil-ascent-literal (string->symbol lk-pat)) (gerbil-ascent-wildcard))
                                                 (gerbil-ascent-variable '?name) (gerbil-ascent-variable '?line))))))
                        (relational-rule/reader-origin (list head) body (make-choice-rule-origin body))))
                    poo-helper-entries))
             (canonical-query (and (pair? repairs) (format-canonical-search-form form)))
             (clamped-budget (max 1 (min 4096 solve-budget)))
             (need-entity? (or (hash-get needed 'entity) (pair? active-general-specs) (pair? poo-helper-entries)))
             (need-edge? (or (hash-get needed 'edge) (pair? active-graph-specs) (hash-get needed 'depends_on) (hash-get needed 'depends-on)))
             (need-attr? (or (hash-get needed 'attr) (hash-get needed 'entity_locator) (hash-get needed 'entity-locator)))
             (need-file? (hash-get needed 'file))
             (assemble-choice-program
               (lambda (cur-files cur-entities cur-edges)
                 (let* ((scenario-b-rels (build-scenario-b-choice-inputs cur-files cur-entities cur-edges raw-attrs needed))
                        (edb-rels
                          (append
                            (if need-file? (list (relational-with-role (relational-source 'file 3 (normalize-choice-rows cur-files)) 'input)) '())
                            (if need-entity? (list (relational-with-role (relational-source 'entity 7 (normalize-choice-rows cur-entities)) 'input)) '())
                            (if need-edge? (list (relational-with-role (relational-source 'edge 5 (normalize-choice-rows cur-edges)) 'input)) '())
                            (if need-attr? (list (relational-with-role (relational-source 'attr 3 (normalize-choice-rows raw-attrs)) 'input)) '())))
                        (all-rels (append edb-rels scenario-b-rels general-helper-rels graph-helper-rels poo-helper-rels choice-rels extra-rels))
                        (base-program
                          (gerbil-ascent-program
                            all-rels (append general-helper-rules graph-helper-rules poo-helper-rules plain-rules)
                            input-limit derived-limit output-limit
                            (append (map (lambda (r) (.ref r 'name)) edb-rels)
                                    (map (lambda (r) (.ref r 'name)) scenario-b-rels)
                                    choice-names))))
                   (lower-relational-choice-program base-program choice-names choice-heads))))
             (choice-program (assemble-choice-program filtered-files filtered-entities filtered-edges)))
        (with-catch
          (lambda (err)
            (let ((msg (call-with-output-string (lambda (p) (display-exception err p)))))
              (if (not (string-contains (string-downcase msg) "budget"))
                (raise err)
                (let* ((hot-files (if (hash-get needed 'hot-path-module) (filter is-hot? filtered-files) '()))
                       (free-files (if (hash-get needed 'hot-path-module) (filter (lambda (f) (not (is-hot? f))) filtered-files) filtered-files))
                       (scoped-count (length filtered-files))
                       (constrained-count (length hot-files))
                       (free-vars (length free-files))
                       (est-worlds (bounded-pow max-branch-factor free-vars 1000000))
                       (safe-free (max-safe-free-vars max-branch-factor constrained-count clamped-budget))
                       (rec-limit (max 1 (min scoped-count (+ constrained-count safe-free))))
                       (sample-files (append (take-up-to hot-files 6) (take-up-to free-files 2)))
                       (sample-fids (let ((tbl (make-hash-table))) (for-each (lambda (f) (hash-put! tbl (car f) #t)) sample-files) tbl))
                       (sample-paths (let ((tbl (make-hash-table))) (for-each (lambda (f) (hash-put! tbl (caddr f) #t)) sample-files) tbl))
                       (sample-entities (filter (lambda (e) (hash-get sample-fids (list-ref e 1))) filtered-entities))
                       (sample-edges (filter (lambda (e) (hash-get sample-paths (list-ref e 3))) filtered-edges))
                       (sample-worlds
                         (if (null? sample-files)
                           '()
                           (with-catch
                             (lambda (sub-err)
                               (let ((sub-msg (call-with-output-string (lambda (p) (display-exception sub-err p)))))
                                 (if (string-contains (string-downcase sub-msg) "budget") '() (raise sub-err))))
                             (lambda ()
                               (let* ((sub-prog (assemble-choice-program sample-files sample-entities sample-edges))
                                      (sub-sols (relational-choice-solve sub-prog state-budget: (max 32 (min 64 clamped-budget)))))
                                 (project-choice-solutions sub-sols target-rel solve-provenance?)))))))
                  (vector 'search-result 'choice target-rel
                          (vector 'choice-budget-exhausted clamped-budget msg scoped-count constrained-count free-vars
                                  max-branch-factor est-worlds rec-limit (length sample-files) sample-worlds)
                          repairs canonical-query)))))
          (lambda ()
            (let* ((solutions (relational-choice-solve choice-program state-budget: clamped-budget))
                   (worlds (project-choice-solutions solutions target-rel solve-provenance?)))
              (vector 'search-result 'choice target-rel
                      (vector 'choice-report (length worlds) clamped-budget solve-provenance? worlds
                              (length filtered-files) (length filtered-files-0))
                      repairs canonical-query))))))))

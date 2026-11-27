;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Zero-Regex Rust Open EDB Prefilter Hint Extraction & Dependency-Cone Active
;;; EDB Analysis (`scheme/search-dsl/prefilter.ss`).
(import (only-in :clan/poo/object .get .ref .slot?)
        (only-in :std/string/misc string-contains)
        (only-in :gerbil-ascent/rules/closed-source ascent-closed-dsl-parts)
        (only-in :marlin-agent-core/repair/funcs repair-and-read-ascent-query)
        "./objects.ss")
(export +default-max-files+
        +default-choice-state-budget+
        +edb-relation-names+
        +derived-path-edb-names+
        +derived-name-edb-names+
        append-unique
        datum->hint-string
        edb-table-ref
        rule-head-relations
        rule-body-relations
        reachable-relation-names
        strip-ascent-directives
        extract-form-prefilter-hints
        compile-search-dsl-prefilter)

(def +default-max-files+ 4096)
(def +default-choice-state-budget+ 1024)
(def +edb-relation-names+
  '(file entity edge attr is_test_path prod_path path_has_prefix path_contains
    name_contains name_has_prefix symbol_ref))
(def +derived-path-edb-names+
  '(is_test_path prod_path path_has_prefix path_contains))
(def +derived-name-edb-names+
  '(name_contains name_has_prefix))

(def (append-unique items value)
  (if (member value items) items (append items (list value))))

(def (datum->hint-string x)
  (cond
    ((string? x) x)
    ((and (pair? x) (eq? (car x) 'value) (pair? (cdr x))
          (pair? (cadr x)) (eq? (caadr x) 'quote) (symbol? (cadadr x)))
     (symbol->string (cadadr x)))
    (else #f)))

(def (edb-table-ref edb key)
  (let ((entry (assq key edb)))
    (if entry (cdr entry) '())))

(def (rule-head-relations rule)
  (filter-map (lambda (h) (and (.slot? h 'relation) (.ref h 'relation)))
              (.ref rule 'heads)))

(def (rule-body-relations rule)
  (filter-map (lambda (b) (and (.slot? b 'relation) (.ref b 'relation)))
              (.ref rule 'body)))

;;; Compute the exact dependency cone of relation names reachable from
;;; `query-part` across `all-rules`.
(def (reachable-relation-names query-part all-rules)
  (let ((deps-by-head (make-hash-table-eq))
        (needed (make-hash-table-eq)))
    (for-each
      (lambda (rule)
        (let ((body-rels (rule-body-relations rule)))
          (for-each
            (lambda (head-rel)
              (hash-put! deps-by-head head-rel
                         (append body-rels (or (hash-get deps-by-head head-rel) '()))))
            (rule-head-relations rule))))
      all-rules)
    (let ((seeds
            (append
              (map (lambda (r) (.ref r 'name)) (.ref query-part 'relations))
              (apply append (map rule-head-relations (.ref query-part 'rules)))
              (apply append (map rule-body-relations (.ref query-part 'rules))))))
      (let loop ((queue seeds))
        (when (pair? queue)
          (let ((rel (car queue)))
            (if (hash-get needed rel)
              (loop (cdr queue))
              (begin
                (hash-put! needed rel #t)
                (loop (append (or (hash-get deps-by-head rel) '()) (cdr queue))))))))
      needed)))

(def (relation->language-entry rel-name registry)
  (let ((entry (assq rel-name (.get registry relation-meta))))
    (and entry (cadr entry))))

(def (choose-path-prefix current candidate)
  (if (and (string? candidate) (> (string-length candidate) 0))
    (or current candidate)
    current))

(def (safe-content-token? text)
  (and (string? text)
       (>= (string-length text) 3)
       (not (string-contains text "/"))
       (not (string-contains text "#"))))

(def (extract-atom-hints atom registry langs path-prefix tokens)
  (if (not (and (pair? atom) (symbol? (car atom))))
    (values langs path-prefix tokens)
    (let* ((head (car atom))
           (args (cdr atom))
           (rel-role (relation->language-entry head registry)))
      (cond
        ((and rel-role (string=? rel-role "general"))
         (let* ((lang (and (>= (length args) 3) (datum->hint-string (list-ref args 2))))
                (name (and (>= (length args) 4) (datum->hint-string (list-ref args 3))))
                (l1 (if lang (append-unique langs lang) langs))
                (t1 (if (safe-content-token? name) (append-unique tokens name) tokens)))
           (values l1 path-prefix t1)))
        ((and rel-role (string=? rel-role "graph"))
         (let* ((col0 (and (>= (length args) 1) (datum->hint-string (list-ref args 0))))
                (col1 (and (>= (length args) 2) (datum->hint-string (list-ref args 1))))
                (path (and (>= (length args) 3) (datum->hint-string (list-ref args 2))))
                (p1 (choose-path-prefix path-prefix path))
                (t1 (if (safe-content-token? col0) (append-unique tokens col0) tokens))
                (t2 (if (safe-content-token? col1) (append-unique t1 col1) t1)))
           (values langs p1 t2)))
        ((and rel-role (string=? rel-role "graph-pair"))
         (values langs path-prefix tokens))
        ((and rel-role (not (string=? rel-role "edb")))
         (let* ((l1 (append-unique langs rel-role))
                (name (and (>= (length args) 3) (datum->hint-string (caddr args))))
                (t1 (if (safe-content-token? name) (append-unique tokens name) tokens)))
           (values l1 path-prefix t1)))
        ((eq? head 'use)
         (let ((l1 (let loop ((rest args) (acc langs))
                     (if (null? rest)
                       acc
                       (let ((s (datum->hint-string (car rest))))
                         (loop (cdr rest) (if s (append-unique acc s) acc)))))))
           (values l1 path-prefix tokens)))
        ((eq? head 'scope)
         (let loop ((rest args) (p path-prefix))
           (cond
             ((not (and (pair? rest) (pair? (cdr rest)))) (values langs p tokens))
             ((or (equal? (car rest) path-prefix:) (eq? (car rest) 'path-prefix))
              (loop (cddr rest) (choose-path-prefix p (datum->hint-string (cadr rest)))))
             (else (loop (cdr rest) p)))))
        ((eq? head 'file)
         (let* ((lang (and (>= (length args) 2) (datum->hint-string (cadr args))))
                (path (and (>= (length args) 3) (datum->hint-string (caddr args))))
                (l1 (if lang (append-unique langs lang) langs))
                (p1 (choose-path-prefix path-prefix path)))
           (values l1 p1 tokens)))
        ((eq? head 'string-prefix?)
         (let* ((prefix (and (>= (length args) 2) (datum->hint-string (car args))))
                (p1 (choose-path-prefix path-prefix prefix)))
           (values langs p1 tokens)))
        ((eq? head 'path_has_prefix)
         (let* ((prefix (and (>= (length args) 2) (datum->hint-string (cadr args))))
                (p1 (choose-path-prefix path-prefix prefix)))
           (values langs p1 tokens)))
        ((eq? head 'symbol_ref)
         (let* ((name (and (>= (length args) 1) (datum->hint-string (car args))))
                (t1 (if (safe-content-token? name) (append-unique tokens name) tokens)))
           (values langs path-prefix t1)))
        ((eq? head 'entity)
         (let* ((lang (and (>= (length args) 3) (datum->hint-string (list-ref args 2))))
                (name (and (>= (length args) 6) (datum->hint-string (list-ref args 5))))
                (l1 (if lang (append-unique langs lang) langs))
                (t1 (if (safe-content-token? name) (append-unique tokens name) tokens)))
           (values l1 path-prefix t1)))
        ((eq? head 'edge)
         (let* ((src (and (>= (length args) 1) (datum->hint-string (car args))))
                (dst (and (>= (length args) 2) (datum->hint-string (cadr args))))
                (path (and (>= (length args) 4) (datum->hint-string (list-ref args 3))))
                (p1 (choose-path-prefix path-prefix path))
                (t1 (if (safe-content-token? src) (append-unique tokens src) tokens))
                (t2 (if (safe-content-token? dst) (append-unique t1 dst) t1)))
           (values langs p1 t2)))
        (else (values langs path-prefix tokens))))))

(def (count-positive-file-atoms tree)
  (cond
    ((not (pair? tree)) 0)
    ((eq? (car tree) 'not) 0)
    ((eq? (car tree) 'file) 1)
    (else (+ (count-positive-file-atoms (car tree))
             (count-positive-file-atoms (cdr tree))))))

(def (walk-form-hints form registry langs path-prefix tokens (allow-path-pred? #t))
  (cond
    ((not (pair? form)) (values langs path-prefix tokens))
    ((eq? (car form) 'not) (values langs path-prefix tokens))
    ((symbol? (car form))
     (let-values (((l1 p1 t1)
                   (if (and (not allow-path-pred?) (eq? (car form) 'path_has_prefix))
                     (values langs path-prefix tokens)
                     (extract-atom-hints form registry langs path-prefix tokens))))
       (let loop ((rest (cdr form)) (l l1) (p p1) (t t1))
         (if (not (pair? rest))
           (values l p t)
           (let-values (((nl np nt) (walk-form-hints (car rest) registry l p t allow-path-pred?)))
             (loop (cdr rest) nl np nt))))))
    (else
     (let loop ((rest form) (l langs) (p path-prefix) (t tokens))
       (if (not (pair? rest))
         (values l p t)
         (let-values (((nl np nt) (walk-form-hints (car rest) registry l p t allow-path-pred?)))
           (loop (cdr rest) nl np nt)))))))

(def (strip-ascent-directives form)
  (let loop ((rest (cdr form)) (acc '()) (row-limit #f) (context-lines #f))
    (if (null? rest)
      (values (cons (car form) (reverse acc)) row-limit context-lines)
      (let ((c (car rest)))
        (cond
          ((and (pair? c) (eq? (car c) 'limit) (= (length c) 2)
                (exact-integer? (cadr c)) (> (cadr c) 0))
           (loop (cdr rest) acc (cadr c) context-lines))
          ((and (pair? c) (eq? (car c) 'context) (= (length c) 2)
                (exact-integer? (cadr c)) (>= (cadr c) 0))
           (loop (cdr rest) acc row-limit (min 8 (cadr c))))
          ((and (pair? c) (eq? (car c) 'scope))
           (loop (cdr rest) acc row-limit context-lines))
          (else (loop (cdr rest) (cons c acc) row-limit context-lines)))))))

(def (compute-active-edb-for-form form registry)
  (if (and (pair? form) (memq (car form) '(ascent ascent-fragment)))
    (let-values (((pure-form _row-limit _ctx) (strip-ascent-directives form)))
      (let ((query-parts (with-catch (lambda (_) #f)
                           (lambda () (ascent-closed-dsl-parts (list pure-form))))))
        (if (and (pair? query-parts) (null? (cdr query-parts)))
          (let* ((all-parts (append (.get registry ascent-fragments) query-parts))
                 (all-rules (apply append (map (lambda (p) (.ref p 'rules)) all-parts)))
                 (needed (reachable-relation-names (car query-parts) all-rules)))
            (filter (lambda (n) (hash-get needed n)) +edb-relation-names+))
          +edb-relation-names+)))
    +edb-relation-names+))

(def (ascent-rule-form? c)
  (and (pair? c) (pair? (car c)) (pair? (cdr c)) (eq? (cadr c) '<--)))

(def (intersect-string-lists a b)
  (filter (lambda (x) (member x b)) a))

(def (extract-form-prefilter-hints form registry)
  (if (not (and (pair? form) (memq (car form) '(ascent ascent-fragment))))
    (walk-form-hints form registry '() #f '() #t)
    (let* ((clauses (cdr form))
           (rules (filter ascent-rule-form? clauses))
           (non-rules (filter (lambda (c) (not (ascent-rule-form? c))) clauses)))
      (if (<= (length rules) 1)
        (let ((single-file? (<= (count-positive-file-atoms form) 1)))
          (walk-form-hints form registry '() #f '() single-file?))
        (let-values (((top-langs top-prefix top-tokens)
                      (walk-form-hints non-rules registry '() #f '() #t)))
          (let loop ((rs rules)
                     (union-langs top-langs)
                     (all-have-lang? #t)
                     (shared-prefix 'unset)
                     (common-tokens 'unset))
            (if (null? rs)
              (let ((final-langs (if all-have-lang? union-langs '()))
                    (final-prefix (or top-prefix (and (string? shared-prefix) shared-prefix)))
                    (final-tokens (append top-tokens
                                          (if (list? common-tokens) common-tokens '()))))
                (values final-langs final-prefix final-tokens))
              (let-values (((rl rp rt)
                            (walk-form-hints (car rs) registry '() #f '()
                                             (<= (count-positive-file-atoms (car rs)) 1))))
                (let* ((next-all-lang? (and all-have-lang? (pair? rl)))
                       (next-langs (if next-all-lang?
                                     (foldl (lambda (x acc) (append-unique acc x)) union-langs rl)
                                     '()))
                       (next-prefix
                         (cond
                           ((eq? shared-prefix 'unset) rp)
                           ((and (string? shared-prefix) (string? rp) (string=? shared-prefix rp))
                            shared-prefix)
                           (else #f)))
                       (next-tokens
                         (if (eq? common-tokens 'unset)
                           rt
                           (intersect-string-lists common-tokens rt))))
                  (loop (cdr rs) next-langs next-all-lang? next-prefix next-tokens))))))))))

;;; Compile any native `:gerbil-ascent` query (after automatic safe AST repair)
;;; into a zero-regex rooted Rust Open EDB prefilter 6-vector:
;;;   #(search-edb-prefilter languages-vec path-prefix tokens-vec max-files active-edb-vec)
(def (compile-search-dsl-prefilter text (registry SearchLanguageRegistryValue))
  (let-values (((form _repairs) (repair-and-read-ascent-query text registry)))
    (let-values (((langs path-prefix tokens) (extract-form-prefilter-hints form registry)))
      (let ((active-edb (compute-active-edb-for-form form registry)))
        (vector 'search-edb-prefilter
                (list->vector langs)
                path-prefix
                (list->vector tokens)
                +default-max-files+
                (list->vector active-edb))))))

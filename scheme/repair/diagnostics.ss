;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO-Registry Schema Introspection, Similarity Ranking, Wildcard Padding,
;;; and Structured `DSL_REPAIR_DIAGNOSTIC` Emitters (`scheme/repair/diagnostics.ss`).
(import (only-in :clan/poo/object .get)
        (only-in :std/string/misc string-contains)
        "./objects.ss"
        "./reader.ss")
(export suggest-similar-relations
        raise-repair-diagnostic
        named-schema-for-relation
        column-aliases-for-relation
        build-registry-signature-table
        raise-unknown-relation-error
        raise-arity-mismatch-error
        scope-clause?
        normalize-scope-clause
        pad-atom-wildcards)

(def (common-prefix-len a b)
  (let ((limit (min (string-length a) (string-length b))))
    (let loop ((i 0))
      (if (and (< i limit) (char=? (string-ref a i) (string-ref b i)))
        (loop (+ i 1))
        i))))

(def (shared-bigram-count a b)
  (let ((la (string-length a))
        (lb (string-length b)))
    (if (or (< la 2) (< lb 2))
      0
      (let loop ((i 0) (hits 0))
        (if (>= (+ i 1) la)
          hits
          (let ((bg (substring a i (+ i 2))))
            (loop (+ i 1) (if (string-contains b bg) (+ hits 1) hits))))))))

(def (relation-similarity-score needle cand)
  (let* ((n (string-downcase needle))
         (c (string-downcase cand))
         (pref (common-prefix-len n c))
         (bg (shared-bigram-count n c))
         (sub (if (or (string-contains c n) (string-contains n c)) 6 0))
         (len-pen (abs (- (string-length n) (string-length c)))))
    (- (+ (* pref 4) (* bg 2) sub) len-pen)))

(def (insert-ranked-candidate pair ranked)
  (cond
    ((null? ranked) (list pair))
    ((> (car pair) (caar ranked)) (cons pair ranked))
    (else (cons (car ranked) (insert-ranked-candidate pair (cdr ranked))))))

(def (take-at-most lst n)
  (if (or (<= n 0) (null? lst))
    '()
    (cons (car lst) (take-at-most (cdr lst) (- n 1)))))

;;; Return up to 4 closest known relation names from `known-names` for `unknown-sym`.
(def (suggest-similar-relations unknown-sym known-names)
  (let ((needle (symbol->string unknown-sym)))
    (let loop ((rest known-names) (ranked '()))
      (if (null? rest)
        (map cdr (take-at-most ranked 4))
        (let* ((cand-sym (car rest))
               (cand-str (symbol->string cand-sym))
               (score (relation-similarity-score needle cand-str)))
          (loop (cdr rest)
                (if (> score 0)
                  (insert-ranked-candidate (cons score cand-str) ranked)
                  ranked)))))))

;;; Format a compact, schema-grounded structured diagnostic so even small/fast
;;; models can self-repair arbitrarily complex queries on the very next turn.
(def (raise-repair-diagnostic error-class message expected-schema suggested-fix)
  (error
    (call-with-output-string
      (lambda (p)
        (display "DSL_REPAIR_DIAGNOSTIC\n" p)
        (display "error_class: " p)
        (display error-class p)
        (display "\nmessage: " p)
        (display message p)
        (when expected-schema
          (display "\nexpected_schema: " p)
          (display expected-schema p))
        (when suggested-fix
          (display "\nsuggested_fix: " p)
          (display suggested-fix p))))))

(def (named-schema-for-relation rel-sym sig-table)
  (case rel-sym
    ((file) "file(file_id, lang, path)")
    ((entity) "entity(id, file_id, lang, kind, language_kind, name, line)")
    ((edge) "edge(src_id, dst_id, rel, path, line)")
    ((attr) "attr(id, key, val)")
    ((is_test_path) "is_test_path(path)")
    ((prod_path) "prod_path(path)")
    ((path_has_prefix) "path_has_prefix(path, prefix)")
    ((path_contains) "path_contains(path, needle)")
    ((name_contains) "name_contains(name, needle)")
    ((name_has_prefix) "name_has_prefix(name, prefix)")
    ((symbol_ref) "symbol_ref(name, path, line)")
    ((impl_trait) "impl_trait(type_name, trait_name, path, line)")
    ((contains_edge) "contains_edge(parent_id, child_id, path, line)")
    ((imports_edge) "imports_edge(src_id, dst_id, path, line)")
    ((calls_edge) "calls_edge(src_id, dst_id, path, line)")
    ((calls_symbol calls-symbol) "calls_symbol(caller_name, callee_name, path, line)")
    ((implements_edge) "implements_edge(src_id, dst_id, path, line)")
    ((ffi_binds_edge ffi-binds-edge) "ffi_binds_edge(scheme_fn, c_sym, path, line)")
    ((entity_locator entity-locator) "entity_locator(id, locator)")
    ((depends_on depends-on) "depends_on(src_id, dst_id)")
    ((ref-at) "ref-at(callsite, name)")
    ((same-module-def) "same-module-def(callsite, name, def_id)")
    ((workspace-def) "workspace-def(name, def_id)")
    ((in-test-file) "in-test-file(def_id)")
    ((module-in-scope) "module-in-scope(mod_path)")
    ((hot-path-module) "hot-path-module(mod_path)")
    ((module-depends) "module-depends(mod_path, target)")
    (else
     (let ((sig (hash-get sig-table rel-sym)))
       (cond
         ((memq rel-sym +general-ontology-5col-rels+)
          (string-append (symbol->string rel-sym) "(id, file_id, lang, name, line)"))
         ((and sig (= (length sig) 4))
          (string-append (symbol->string rel-sym) "(id, file_id, name, line)"))
         (sig
          (call-with-output-string
            (lambda (p)
              (display rel-sym p)
              (write sig p))))
         (else #f))))))

(def (column-aliases-for-relation rel-sym sig-table)
  (case rel-sym
    ((file) '((fid file_id id) (lang language) (path file_path)))
    ((entity) '((id entity_id) (fid file_id) (lang language) (kind) (lk language_kind) (name sym) (line)))
    ((edge) '((src from) (dst to) (rel kind) (path) (line)))
    ((attr) '((id entity_id) (key k) (val value v)))
    ((is_test_path prod_path) '((path file_path p)))
    ((path_has_prefix) '((path file_path p) (prefix)))
    ((path_contains) '((path file_path p) (needle)))
    ((name_contains) '((name sym) (needle)))
    ((name_has_prefix) '((name sym) (prefix)))
    ((symbol_ref) '((name sym) (path file_path p) (line)))
    ((impl_trait) '((type type_name ty) (trait trait_name tr) (path) (line)))
    ((contains_edge) '((parent src) (child dst) (path) (line)))
    ((imports_edge calls_edge implements_edge) '((src from) (dst to) (path) (line)))
    ((calls_symbol calls-symbol) '((caller caller_name src) (callee callee_name dst) (path) (line)))
    ((ffi_binds_edge ffi-binds-edge) '((scheme_fn src) (c_sym dst) (path) (line)))
    ((entity_locator) '((id) (loc locator)))
    ((depends_on) '((src from) (dst to)))
    (else
     (let ((sig (hash-get sig-table rel-sym)))
       (cond
         ((memq rel-sym +general-ontology-5col-rels+)
          '((id) (fid file_id) (lang language) (name sym) (line)))
         ((and sig (= (length sig) 4))
          '((id) (fid file_id) (name sym) (line)))
         (else '()))))))

(def (build-registry-signature-table registry)
  (let ((table (make-hash-table-eq)))
    (hash-put! table 'file '(integer? string? string?))
    (hash-put! table 'entity '(string? integer? string? string? string? string? integer?))
    (hash-put! table 'edge '(string? string? string? string? integer?))
    (hash-put! table 'attr '(string? string? string?))
    (hash-put! table 'is_test_path '(string?))
    (hash-put! table 'prod_path '(string?))
    (hash-put! table 'path_has_prefix '(string? string?))
    (hash-put! table 'path_contains '(string? string?))
    (hash-put! table 'name_contains '(string? string?))
    (hash-put! table 'name_has_prefix '(string? string?))
    (hash-put! table 'symbol_ref '(string? string? integer?))
    ;; Built-in Dusa Scenario B semantic disambiguation & architecture boundary predicates
    (hash-put! table 'ref-at '(symbol? symbol?))
    (hash-put! table 'same-module-def '(symbol? symbol? symbol?))
    (hash-put! table 'workspace-def '(symbol? symbol?))
    (hash-put! table 'in-test-file '(symbol?))
    (hash-put! table 'module-in-scope '(symbol?))
    (hash-put! table 'hot-path-module '(symbol?))
    (hash-put! table 'module-depends '(symbol? symbol?))
    (hash-put! table 'contains-edge '(symbol? symbol? symbol? integer?))
    (hash-put! table 'imports-edge '(symbol? symbol? symbol? integer?))
    (hash-put! table 'calls-edge '(symbol? symbol? symbol? integer?))
    (hash-put! table 'calls-symbol '(symbol? symbol? symbol? integer?))
    (hash-put! table 'implements-edge '(symbol? symbol? symbol? integer?))
    (hash-put! table 'entity-locator '(symbol? symbol?))
    (hash-put! table 'depends-on '(symbol? symbol?))
    (for-each
      (lambda (meta)
        (let ((rel-name (car meta))
              (lang (cadr meta)))
          (cond
            ((string=? lang "edb") #f)
            ((string=? lang "general")
             (hash-put! table rel-name '(string? integer? string? string? integer?)))
            ((string=? lang "graph")
             (hash-put! table rel-name '(string? string? string? integer?)))
            ((string=? lang "graph-pair")
             (hash-put! table rel-name '(string? string?)))
            (else
             (hash-put! table rel-name '(string? integer? string? integer?))))))
      (.get registry relation-meta))
    table))

(def (raise-unknown-relation-error rel-sym known-table)
  (let* ((names (let ((acc '()))
                  (hash-for-each (lambda (k _v) (set! acc (cons k acc))) known-table)
                  acc))
         (suggestions (suggest-similar-relations rel-sym names))
         (msg (call-with-output-string
                (lambda (p)
                  (display "unknown :gerbil-ascent relation '" p)
                  (display rel-sym p)
                  (display "'" p)
                  (when (pair? suggestions)
                    (display "; did you mean: " p)
                    (display (string-join suggestions ", ") p)
                    (display "?" p)))))
         (first-schema (and (pair? suggestions)
                            (named-schema-for-relation (string->symbol (car suggestions)) known-table))))
    (raise-repair-diagnostic
      'unknown_relation
      msg
      (or first-schema "EDB: file(fid,lang,path), entity(id,fid,lang,kind,lk,name,line), edge(src,dst,rel,path,line), attr(id,k,v), is_test_path(p), prod_path(p), path_has_prefix(p,pfx), path_contains(p,sub); Ontology: callable/type_decl(id,fid,lang,name,line), impl_trait(type,trait,path,line); Special: rust_fn/rust_struct/rust_enum/rust_trait/rust_impl(id,fid,name,line)")
      "Replace the unknown relation with one of the suggested relations above (or use (brute-search \"...\" path-prefix: \"...\") for text/regex search).")))

(def (raise-arity-mismatch-error rel-sym expected-sig got-arity sig-table)
  (let ((schema (or (named-schema-for-relation rel-sym sig-table)
                    (call-with-output-string
                      (lambda (p) (display rel-sym p) (write expected-sig p))))))
    (raise-repair-diagnostic
      'arity_mismatch
      (call-with-output-string
        (lambda (p)
          (display "ASCENT relation '" p)
          (display rel-sym p)
          (display "' expects " p)
          (display (length expected-sig) p)
          (display " column(s), got " p)
          (display got-arity p)))
      schema
      (string-append "Rewrite the atom using exact arity: (" schema ")"))))

(def (scope-clause? clause)
  (and (pair? clause)
       (list? clause)
       (eq? (car clause) 'scope)))

(def (normalize-scope-clause clause tags)
  (let ((args (cdr clause)))
    (cond
      ((and (= (length args) 1) (string? (car args)))
       (values (list 'scope path-prefix: (car args))
               (add-repair-tag tags 'normalize-scope-directive)))
      ((and (>= (length args) 2)
            (or (equal? (car args) path:)
                (equal? (car args) prefix:)
                (eq? (car args) 'path)
                (eq? (car args) 'prefix)
                (eq? (car args) 'path-prefix)))
       (values (cons 'scope (cons path-prefix: (cdr args)))
               (add-repair-tag tags 'normalize-scope-directive)))
      (else (values clause tags)))))

(def (pad-atom-wildcards atom sig-table wildcard-sym tags)
  (cond
    ((not (and (pair? atom) (symbol? (car atom))))
     (values atom tags))
    ((and (eq? (car atom) 'not) (= (length atom) 2) (pair? (cadr atom)))
     (let-values (((inner t1) (pad-atom-wildcards (cadr atom) sig-table wildcard-sym tags)))
       (values (list 'not inner) t1)))
    (else
     (let ((sig (hash-get sig-table (car atom)))
           (args (cdr atom)))
       (if (and sig (pair? args) (< (length args) (length sig)))
         (let ((missing (- (length sig) (length args))))
           (values (cons (car atom) (append args (make-list missing wildcard-sym)))
                   (add-repair-tag tags 'pad-relation-wildcards)))
         (values atom tags))))))

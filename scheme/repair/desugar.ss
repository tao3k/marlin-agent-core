;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative L1/L2 Surface Form Lowering (`scheme/repair/desugar.ss`).
;;;
;;; Compiles concise high-level structural & constraint-solving forms directly
;;; into closed `:gerbil-ascent` and Dusa POPL 2025 programs with zero runtime
;;; shims:
;;;   - L1 Ascent: `find-impact`, `find-collisions`, `find-callers`,
;;;                `find-impls`, `find-symbols`
;;;   - L2 Dusa:   `solve-placement`, `solve-resolve`
(import "./diagnostics.ss")
(export desugar-high-level-search-form)

(def (dsl-keyword-name tok)
  (cond
    ((keyword? tok)
     (string->symbol (keyword->string tok)))
    ((symbol? tok)
     (let* ((s (symbol->string tok))
            (n (string-length s)))
       (cond
         ((and (> n 1) (char=? (string-ref s (- n 1)) #\:))
          (string->symbol (substring s 0 (- n 1))))
         ((and (> n 1) (char=? (string-ref s 0) #\:))
          (string->symbol (substring s 1 n)))
         (else #f))))
    (else #f)))

(def (parse-dsl-kwargs args)
  (let loop ((rest args) (pos '()) (kw '()))
    (cond
      ((null? rest) (values (reverse pos) (reverse kw)))
      ((dsl-keyword-name (car rest))
       => (lambda (k)
            (if (null? (cdr rest))
              (values (reverse pos) (reverse kw))
              (loop (cddr rest) pos (cons (cons k (cadr rest)) kw)))))
      (else
       (loop (cdr rest) (cons (car rest) pos) kw)))))

(def (kw-ref kwargs keys default)
  (let loop ((ks keys))
    (cond
      ((null? ks) default)
      ((assq (car ks) kwargs) => cdr)
      (else (loop (cdr ks))))))

(def (datum->lower-string x default)
  (cond
    ((string? x) (string-downcase x))
    ((symbol? x) (string-downcase (symbol->string x)))
    (else default)))

(def (datum->symbol-atom x)
  (cond
    ((symbol? x) x)
    ((string? x) (string->symbol x))
    ((and (pair? x) (eq? (car x) 'quote) (pair? (cdr x)) (symbol? (cadr x)))
     (cadr x))
    (else #f)))

(def (datum->symbol-list x default)
  (cond
    ((and (pair? x) (eq? (car x) 'quote) (pair? (cdr x)) (list? (cadr x)))
     (let ((syms (filter symbol? (map datum->symbol-atom (cadr x)))))
       (if (null? syms) default syms)))
    ((list? x)
     (let ((syms (filter symbol? (map datum->symbol-atom x))))
       (if (null? syms) default syms)))
    ((datum->symbol-atom x) => list)
    (else default)))

(def (datum->string-list x)
  (cond
    ((string? x) (if (> (string-length x) 0) (list x) '()))
    ((and (pair? x) (eq? (car x) 'quote) (pair? (cdr x)) (list? (cadr x)))
     (filter (lambda (s) (and (string? s) (> (string-length s) 0))) (cadr x)))
    ((list? x)
     (filter (lambda (s) (and (string? s) (> (string-length s) 0))) x))
    (else '())))

(def (positive-int-or x default)
  (if (and (exact-integer? x) (> x 0)) x default))

(def (kind-lang->entity-pattern lang-str kind-str file-var name-var line-var (lang-kind-str #f))
  (if (and (string? lang-kind-str) (> (string-length lang-kind-str) 0)
           (not (member lang-kind-str '("any" "all" "*"))))
    `(entity _ ,file-var
             ,(if (string=? lang-str "") '_ lang-str)
             ,(if (or (string=? kind-str "") (string=? kind-str "any")) '_ kind-str)
             ,lang-kind-str ,name-var ,line-var)
    (cond
      ((and (string=? lang-str "rust") (member kind-str '("struct" "structs")))
       `(rust_struct _ ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("fn" "func" "function" "functions")))
       `(rust_fn _ ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("method" "methods")))
       `(rust_method _ ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("enum" "enums")))
       `(rust_enum _ ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("trait" "traits")))
       `(rust_trait _ ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("macro" "macros")))
       `(rust_macro _ ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("impl" "impls")))
       `(rust_impl _ ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "scheme") (member kind-str '("def" "fn" "function")))
       `(scheme_def _ ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "scheme") (member kind-str '("poo" "object")))
       `(scheme_poo_def _ ,file-var ,name-var ,line-var))
      ((member kind-str '("type" "types"))
       `(type_decl _ ,file-var ,(if (string=? lang-str "") '_ lang-str) ,name-var ,line-var))
      ((member kind-str '("callable" "fn" "func" "function" "functions"))
       `(callable _ ,file-var ,(if (string=? lang-str "") '_ lang-str) ,name-var ,line-var))
      (else
       `(entity _ ,file-var
                ,(if (string=? lang-str "") '_ lang-str)
                ,(if (or (string=? kind-str "") (string=? kind-str "any")) '_ kind-str)
                _ ,name-var ,line-var)))))

(def (build-scope-clause path-prefix (scope-limit #f))
  (cond
    ((and (string? path-prefix) (> (string-length path-prefix) 0) scope-limit)
     `((scope path-prefix: ,path-prefix limit: ,scope-limit)))
    ((and (string? path-prefix) (> (string-length path-prefix) 0))
     `((scope path-prefix: ,path-prefix)))
    (scope-limit
     `((scope limit: ,scope-limit)))
    (else '())))

(def (build-path-filter-atoms path-vars exclude-tests? exclude-prefixes path-contains)
  (apply append
    (map (lambda (pv)
           (append
             (if exclude-tests? `((not (is_test_path ,pv))) '())
             (map (lambda (pfx) `(not (path_has_prefix ,pv ,pfx))) exclude-prefixes)
             (if (and (string? path-contains) (> (string-length path-contains) 0))
               `((path_contains ,pv ,path-contains))
               '())))
         path-vars)))

(def (build-name-filter-atoms name-var name-contains-list name-prefixes)
  (append
    (cond
      ((null? name-contains-list) '())
      ((null? (cdr name-contains-list))
       `((name_contains ,name-var ,(car name-contains-list))))
      (else
       `((or ,@(map (lambda (needle) `(name_contains ,name-var ,needle)) name-contains-list)))))
    (cond
      ((null? name-prefixes) '())
      ((null? (cdr name-prefixes))
       `((name_has_prefix ,name-var ,(car name-prefixes))))
      (else
       `((or ,@(map (lambda (pfx) `(name_has_prefix ,name-var ,pfx)) name-prefixes)))))))

(def (normalize-dsl-lang raw-lang prefix default-lang)
  (let ((s (cond
             ((not raw-lang)
              (cond
                ((and (string? prefix) (>= (string-length prefix) 7)
                      (string=? (substring prefix 0 7) "scheme/"))
                 "scheme")
                ((and (string? prefix) (>= (string-length prefix) 7)
                      (string=? (substring prefix 0 7) "crates/"))
                 "rust")
                (else default-lang)))
             (else (datum->lower-string raw-lang default-lang)))))
    (if (or (string=? s "") (string=? s "any") (string=? s "all") (string=? s "*"))
      ""
      s)))

(def (desugar-find-collisions args)
  (let-values (((pos kwargs) (parse-dsl-kwargs args)))
    (let* ((raw-kind (kw-ref kwargs '(kind kinds) (if (pair? pos) (car pos) "struct")))
           (kind-str (datum->lower-string raw-kind "struct"))
           (prefix (kw-ref kwargs '(path-prefix prefix scope) #f))
           (lang-str (normalize-dsl-lang (kw-ref kwargs '(lang language) #f) prefix "rust"))
           (lang-pat (if (string=? lang-str "") '_ lang-str))
           (exclude-tests? (if (kw-ref kwargs '(exclude-tests? exclude-tests prod-only? prod-only) #f) #t #f))
           (exclude-prefixes (datum->string-list (kw-ref kwargs '(exclude-path-prefix exclude-prefix) #f)))
           (contains-str (kw-ref kwargs '(path-contains) #f))
           (name-contains-list (datum->string-list (kw-ref kwargs '(name-contains name) #f)))
           (name-prefixes (datum->string-list (kw-ref kwargs '(name-prefix) #f)))
           (min-files (max 2 (min 4 (positive-int-or (kw-ref kwargs '(min-files min-occurrences) 2) 2))))
           (lim (positive-int-or (kw-ref kwargs '(limit) 40) 40))
           (ctx (kw-ref kwargs '(context) #f))
           (r1 (kind-lang->entity-pattern lang-str kind-str 'f1 'name 'l1))
           (r2 (kind-lang->entity-pattern lang-str kind-str 'f2 'name 'l2))
           (r3 (kind-lang->entity-pattern lang-str kind-str 'f3 'name 'l3))
           (r4 (kind-lang->entity-pattern lang-str kind-str 'f4 'name 'l4))
           (extra-joins
             (cond
               ((= min-files 2) '())
               ((= min-files 3)
                `(,r3 (file f3 ,lang-pat p3) (< f2 f3)
                  ,@(build-path-filter-atoms '(p3) exclude-tests? exclude-prefixes contains-str)))
               (else
                `(,r3 (file f3 ,lang-pat p3) (< f2 f3)
                  ,r4 (file f4 ,lang-pat p4) (< f3 f4)
                  ,@(build-path-filter-atoms '(p3 p4) exclude-tests? exclude-prefixes contains-str)))))
           (path-filters (build-path-filter-atoms '(p1 p2) exclude-tests? exclude-prefixes contains-str))
           (name-filters (build-name-filter-atoms 'name name-contains-list name-prefixes)))
      `(ascent-fragment
         ,@(build-scope-clause prefix)
         (limit ,lim)
         ,@(if (and (exact-integer? ctx) (> ctx 0)) `((context ,ctx)) '())
         ((answer name p1 l1 p2 l2)
          <-- ,r1 ,r2
              (file f1 ,lang-pat p1)
              (file f2 ,lang-pat p2)
              (< f1 f2)
              ,@extra-joins
              ,@path-filters
              ,@name-filters)))))

(def (desugar-find-symbols args)
  (let-values (((pos kwargs) (parse-dsl-kwargs args)))
    (let* ((name-contains-list (datum->string-list (kw-ref kwargs '(name-contains name) #f)))
           (name-prefixes (datum->string-list (kw-ref kwargs '(name-prefix) #f)))
           (has-name-filter? (or (pair? name-contains-list) (pair? name-prefixes)))
           (raw-kind (kw-ref kwargs '(kind kinds)
                             (if (pair? pos)
                               (car pos)
                               (if (or has-name-filter? (kw-ref kwargs '(languageKind language-kind language_kind) #f))
                                 "any"
                                 "struct"))))
           (kind-str (datum->lower-string raw-kind "struct"))
           (raw-lk (kw-ref kwargs '(languageKind language-kind language_kind) #f))
           (lk-str (and raw-lk (datum->lower-string raw-lk #f)))
           (prefix (kw-ref kwargs '(path-prefix prefix scope) #f))
           (lang-str (normalize-dsl-lang (kw-ref kwargs '(lang language languageId) #f)
                                         prefix
                                         (if has-name-filter? "" "rust")))
           (lang-pat (if (string=? lang-str "") '_ lang-str))
           (exclude-tests? (if (kw-ref kwargs '(exclude-tests? exclude-tests prod-only? prod-only) #f) #t #f))
           (exclude-prefixes (datum->string-list (kw-ref kwargs '(exclude-path-prefix exclude-prefix) #f)))
           (contains-str (kw-ref kwargs '(path-contains) #f))
           (lim (positive-int-or (kw-ref kwargs '(limit) 80) 80))
           (ctx (kw-ref kwargs '(context) #f))
           (rel (kind-lang->entity-pattern lang-str kind-str 'f 'name 'line lk-str))
           (exclude-doc-langs
             (if (and (string=? lang-str "")
                      (not (member kind-str '("heading" "section" "doc" "document"))))
               '((not (file f "org" path))
                 (not (file f "markdown" path)))
               '()))
           (path-filters (build-path-filter-atoms '(path) exclude-tests? exclude-prefixes contains-str))
           (name-filters (build-name-filter-atoms 'name name-contains-list name-prefixes)))
      `(ascent-fragment
         ,@(build-scope-clause prefix)
         (limit ,lim)
         ,@(if (and (exact-integer? ctx) (> ctx 0)) `((context ,ctx)) '())
         ((answer name path line)
          <-- ,rel
              (file f ,lang-pat path)
              ,@exclude-doc-langs
              ,@path-filters
              ,@name-filters)))))

(def (desugar-find-callers args)
  (let-values (((pos kwargs) (parse-dsl-kwargs args)))
    (let* ((target (kw-ref kwargs '(callee symbol name target)
                           (if (pair? pos) (car pos) #f)))
           (target-str (cond ((string? target) target)
                             ((symbol? target) (symbol->string target))
                             (else #f)))
           (prefix (kw-ref kwargs '(path-prefix prefix scope) #f))
           (exclude-tests? (if (kw-ref kwargs '(exclude-tests? exclude-tests prod-only? prod-only) #f) #t #f))
           (exclude-prefixes (datum->string-list (kw-ref kwargs '(exclude-path-prefix exclude-prefix) #f)))
           (contains-str (kw-ref kwargs '(path-contains) #f))
           (name-contains-list (datum->string-list (kw-ref kwargs '(name-contains) #f)))
           (name-prefixes (datum->string-list (kw-ref kwargs '(name-prefix) #f)))
           (lim (positive-int-or (kw-ref kwargs '(limit) 40) 40))
           (ctx (kw-ref kwargs '(context) #f))
           (path-filters (build-path-filter-atoms '(path) exclude-tests? exclude-prefixes contains-str))
           (name-filters (build-name-filter-atoms 'caller name-contains-list name-prefixes)))
      (unless (and target-str (> (string-length target-str) 0))
        (raise-repair-diagnostic
          'missing_callee_name
          "(find-callers ...) requires a target function name string"
          "(find-callers \"target_fn\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40)"
          "(find-callers \"execute_package_search_dsl\" path-prefix: \"crates/\")"))
      `(ascent-fragment
         ,@(build-scope-clause prefix)
         (limit ,lim)
         ,@(if (and (exact-integer? ctx) (> ctx 0)) `((context ,ctx)) '())
         ((answer caller path line)
          <-- (calls_symbol caller ,target-str path line)
              ,@path-filters
              ,@name-filters)))))

(def (desugar-find-impls args)
  (let-values (((pos kwargs) (parse-dsl-kwargs args)))
    (let* ((target (kw-ref kwargs '(trait name target)
                           (if (pair? pos) (car pos) #f)))
           (target-str (cond ((string? target) target)
                             ((symbol? target) (symbol->string target))
                             (else #f)))
           (prefix (kw-ref kwargs '(path-prefix prefix scope) #f))
           (exclude-tests? (if (kw-ref kwargs '(exclude-tests? exclude-tests prod-only? prod-only) #f) #t #f))
           (exclude-prefixes (datum->string-list (kw-ref kwargs '(exclude-path-prefix exclude-prefix) #f)))
           (contains-str (kw-ref kwargs '(path-contains) #f))
           (name-contains-list (datum->string-list (kw-ref kwargs '(name-contains) #f)))
           (name-prefixes (datum->string-list (kw-ref kwargs '(name-prefix) #f)))
           (lim (positive-int-or (kw-ref kwargs '(limit) 40) 40))
           (ctx (kw-ref kwargs '(context) #f))
           (path-filters (build-path-filter-atoms '(path) exclude-tests? exclude-prefixes contains-str))
           (name-filters (build-name-filter-atoms 'type_name name-contains-list name-prefixes)))
      (unless (and target-str (> (string-length target-str) 0))
        (raise-repair-diagnostic
          'missing_trait_name
          "(find-impls ...) requires a target trait name string"
          "(find-impls \"TraitName\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40)"
          "(find-impls \"ModelGateway\" path-prefix: \"crates/\")"))
      `(ascent-fragment
         ,@(build-scope-clause prefix)
         (limit ,lim)
         ,@(if (and (exact-integer? ctx) (> ctx 0)) `((context ,ctx)) '())
         ((answer type_name path line)
          <-- (impl_trait type_name ,target-str path line)
              ,@path-filters
              ,@name-filters)))))

(def (desugar-find-impact args)
  (let-values (((pos kwargs) (parse-dsl-kwargs args)))
    (let* ((target (kw-ref kwargs '(symbol name target)
                           (if (pair? pos) (car pos) #f)))
           (target-str (cond ((string? target) target)
                             ((symbol? target) (symbol->string target))
                             (else #f)))
           (prefix (kw-ref kwargs '(path-prefix prefix scope) #f))
           (exclude-tests? (if (kw-ref kwargs '(exclude-tests? exclude-tests prod-only? prod-only) #f) #t #f))
           (exclude-prefixes (datum->string-list (kw-ref kwargs '(exclude-path-prefix exclude-prefix) #f)))
           (contains-str (kw-ref kwargs '(path-contains) #f))
           (lim (positive-int-or (kw-ref kwargs '(limit) 60) 60))
           (ctx (kw-ref kwargs '(context) #f))
           (extra-filters (build-path-filter-atoms '(path) #f exclude-prefixes contains-str))
           (scopes (if exclude-tests?
                     '(("prod" prod_path))
                     '(("prod" prod_path) ("test" is_test_path)))))
      (unless (and target-str (> (string-length target-str) 0))
        (raise-repair-diagnostic
          'missing_impact_symbol
          "(find-impact ...) requires a target symbol name string"
          "(find-impact \"SymbolName\" path-prefix: \"crates/\" exclude-tests?: #f limit: 60)"
          "(find-impact \"WorkspaceSearchIndex\" path-prefix: \"crates/\")"))
      (let ((rules
              (apply append
                (map (lambda (sc)
                       (let ((scope-label (car sc))
                             (scope-rel (cadr sc)))
                         `(((answer "def" ,scope-label ,target-str path line)
                            <-- (entity _ f _ _ _ ,target-str line)
                                (file f _ path)
                                (,scope-rel path)
                                ,@extra-filters)
                           ((answer "caller" ,scope-label caller path line)
                            <-- (calls_symbol caller ,target-str path line)
                                (,scope-rel path)
                                ,@extra-filters)
                           ((answer "impl" ,scope-label ty path line)
                            <-- (impl_trait ty ,target-str path line)
                                (,scope-rel path)
                                ,@extra-filters)
                           ((answer "impl" ,scope-label tr path line)
                            <-- (impl_trait ,target-str tr path line)
                                (,scope-rel path)
                                ,@extra-filters)
                           ((answer "import" ,scope-label src path line)
                            <-- (imports_edge src ,target-str path line)
                                (,scope-rel path)
                                ,@extra-filters)
                           ((answer "ref" ,scope-label ,target-str path line)
                            <-- (symbol_ref ,target-str path line)
                                (,scope-rel path)
                                (not (def_site path line))
                                ,@extra-filters))))
                     scopes))))
        `(ascent-fragment
           ,@(build-scope-clause prefix)
           (limit ,lim)
           ,@(if (and (exact-integer? ctx) (> ctx 0)) `((context ,ctx)) '())
           (relation def_site ((path string?) (line integer?)))
           ((def_site path line)
            <-- (entity _ f _ _ _ ,target-str line)
                (file f _ path))
           ,@rules)))))

(def (desugar-solve-placement args)
  (let-values (((pos kwargs) (parse-dsl-kwargs args)))
    (let* ((prefix (kw-ref kwargs '(path-prefix prefix scope)
                           (if (and (pair? pos) (string? (car pos))) (car pos) #f)))
           (langs (datum->symbol-list (kw-ref kwargs '(use lang langs) '(rust)) '(rust)))
           (scope-limit (positive-int-or (kw-ref kwargs '(module-limit limit max-modules) 6) 6))
           (choices (datum->symbol-list
                      (kw-ref kwargs '(choices tiers domain) '(resident provider))
                      '(resident provider)))
           (raw-forbid (kw-ref kwargs '(forbid-hot-path forbid-hot-path-tier) #f))
           (forbid-sym (and raw-forbid
                            (not (eq? raw-forbid 'none))
                            (datum->symbol-atom raw-forbid)))
           (budget (positive-int-or (kw-ref kwargs '(state-budget budget world-limit max-worlds) 64) 64)))
      `(search-choice-program
         (use ,@langs)
         ,@(build-scope-clause prefix scope-limit)
         (choice tier (m lvl))
         (rule (closed (tier ?m) ,choices) (module-in-scope ?m))
         ,@(if forbid-sym
             `((rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m ,forbid-sym)))
             '())
         (solve tier state-budget: ,budget provenance?: #t)))))

(def (desugar-solve-resolve args)
  (let-values (((pos kwargs) (parse-dsl-kwargs args)))
    (let* ((prefix (kw-ref kwargs '(path-prefix prefix scope)
                           (if (and (pair? pos) (string? (car pos))) (car pos) #f)))
           (langs (datum->symbol-list (kw-ref kwargs '(use lang langs) '(rust)) '(rust)))
           (scope-limit (positive-int-or (kw-ref kwargs '(module-limit limit max-files) 6) 6))
           (exclude-tests? (kw-ref kwargs '(exclude-tests? exclude-tests) #t))
           (budget (positive-int-or (kw-ref kwargs '(state-budget budget world-limit max-worlds) 32) 32)))
      `(search-choice-program
         (use ,@langs)
         ,@(build-scope-clause prefix scope-limit)
         (choice resolve (callsite target))
         (rule (closed (resolve ?call) (?def))
               (ref-at ?call ?name)
               (same-module-def ?call ?name ?def))
         (rule (open (resolve ?call) ?def)
               (ref-at ?call ?name)
               (workspace-def ?name ?def))
         ,@(if exclude-tests?
             '((rule (closed (resolve ?call) ())
                     (resolve ?call ?def)
                     (in-test-file ?def)))
             '())
         (solve resolve state-budget: ,budget provenance?: #t)))))

(def (desugar-high-level-search-form form)
  (if (not (and (pair? form) (symbol? (car form))))
    form
    (case (car form)
      ((find-collisions) (desugar-find-collisions (cdr form)))
      ((find-symbols)    (desugar-find-symbols (cdr form)))
      ((find-callers)    (desugar-find-callers (cdr form)))
      ((find-impls)      (desugar-find-impls (cdr form)))
      ((find-impact)     (desugar-find-impact (cdr form)))
      ((solve-placement) (desugar-solve-placement (cdr form)))
      ((solve-resolve)   (desugar-solve-resolve (cdr form)))
      (else form))))

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

(def (kind-lang->entity-pattern lang-str kind-str file-var name-var line-var (lang-kind-str #f) (id-var '_))
  (if (and (string? lang-kind-str) (> (string-length lang-kind-str) 0)
           (not (member lang-kind-str '("any" "all" "*"))))
    `(entity ,id-var ,file-var
             ,(if (string=? lang-str "") '_ lang-str)
             ,(if (or (string=? kind-str "") (string=? kind-str "any")) '_ kind-str)
             ,lang-kind-str ,name-var ,line-var)
    (cond
      ((and (string=? lang-str "rust") (member kind-str '("struct" "structs")))
       `(rust_struct ,id-var ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("fn" "func" "function" "functions")))
       `(rust_fn ,id-var ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("method" "methods")))
       `(rust_method ,id-var ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("enum" "enums")))
       `(rust_enum ,id-var ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("trait" "traits")))
       `(rust_trait ,id-var ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("macro" "macros")))
       `(rust_macro ,id-var ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "rust") (member kind-str '("impl" "impls")))
       `(rust_impl ,id-var ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "scheme") (member kind-str '("def" "fn" "function")))
       `(scheme_def ,id-var ,file-var ,name-var ,line-var))
      ((and (string=? lang-str "scheme") (member kind-str '("poo" "object")))
       `(scheme_poo_def ,id-var ,file-var ,name-var ,line-var))
      ((member kind-str '("type" "types"))
       `(type_decl ,id-var ,file-var ,(if (string=? lang-str "") '_ lang-str) ,name-var ,line-var))
      ((member kind-str '("callable" "fn" "func" "function" "functions"))
       `(callable ,id-var ,file-var ,(if (string=? lang-str "") '_ lang-str) ,name-var ,line-var))
      (else
       `(entity ,id-var ,file-var
                ,(if (string=? lang-str "") '_ lang-str)
                ,(if (or (string=? kind-str "") (string=? kind-str "any")) '_ kind-str)
                _ ,name-var ,line-var)))))

(def (normalize-dsl-path-prefix raw)
  (and (string? raw)
       (let loop ((s raw))
         (let ((len (string-length s)))
           (cond
             ((and (>= len 2) (string=? (substring s 0 2) "./"))
              (loop (substring s 2 len)))
             ((and (>= len 1) (char=? (string-ref s 0) #\/))
              (loop (substring s 1 len)))
             ((or (= len 0) (string=? s ".")) #f)
             (else s))))))

(def (build-scope-clause path-prefix (scope-limit #f))
  (let ((norm (normalize-dsl-path-prefix path-prefix)))
    (cond
      ((and norm scope-limit)
       `((scope path-prefix: ,norm limit: ,scope-limit)))
      (norm
       `((scope path-prefix: ,norm)))
      (scope-limit
       `((scope limit: ,scope-limit)))
      (else '()))))

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

(def (datum->attr-value-string v)
  (cond
    ((string? v) v)
    ((boolean? v) (if v "true" "false"))
    ((symbol? v) (symbol->string v))
    ((number? v) (number->string v))
    (else #f)))

(def (parse-special-attr-pairs raw)
  (let ((items (if (and (pair? raw) (eq? (car raw) 'quote) (pair? (cdr raw)))
                 (cadr raw)
                 raw)))
    (if (not (list? items))
      '()
      (filter-map
        (lambda (entry)
          (cond
            ((and (pair? entry) (pair? (cdr entry)) (null? (cddr entry)))
             (let ((k (datum->attr-value-string (car entry)))
                   (v (datum->attr-value-string (cadr entry))))
               (and k v (> (string-length k) 0) (> (string-length v) 0) (cons k v))))
            ((and (pair? entry) (not (list? entry)))
             (let ((k (datum->attr-value-string (car entry)))
                   (v (datum->attr-value-string (cdr entry))))
               (and k v (> (string-length k) 0) (> (string-length v) 0) (cons k v))))
            (else #f)))
        items))))

(def (build-special-attr-atoms id-var special-pairs)
  (map (lambda (kv) `(attr ,id-var ,(car kv) ,(cdr kv))) special-pairs))

(def (build-name-filter-atoms name-var name-contains-list name-prefixes (name-all-list '()))
  (append
    (map (lambda (needle) `(name_contains ,name-var ,needle)) name-all-list)
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
           (name-all-list (datum->string-list (kw-ref kwargs '(name-all all-of) #f)))
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
           (name-filters (build-name-filter-atoms 'name name-contains-list name-prefixes name-all-list)))
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
    (let* ((name-all-list (datum->string-list (kw-ref kwargs '(name-all all-of) #f)))
           (name-contains-list (datum->string-list (kw-ref kwargs '(name-contains name-any name) #f)))
           (name-prefixes (datum->string-list (kw-ref kwargs '(name-prefix) #f)))
           (raw-exact (kw-ref kwargs '(exact-name symbol) #f))
           (exact-name (and raw-exact (datum->attr-value-string raw-exact)))
           (special-pairs (parse-special-attr-pairs (kw-ref kwargs '(special attrs) #f)))
           (has-name-filter? (or (pair? name-all-list) (pair? name-contains-list)
                                 (pair? name-prefixes) (and exact-name (> (string-length exact-name) 0))))
           (raw-kind (kw-ref kwargs '(kind kinds)
                             (if (pair? pos)
                               (car pos)
                               (if (or has-name-filter?
                                       (pair? special-pairs)
                                       (kw-ref kwargs '(languageKind language-kind language_kind) #f))
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
           (id-var (if (pair? special-pairs) 'eid '_))
           (name-pat (if (and exact-name (> (string-length exact-name) 0)) exact-name 'name))
           (rel (kind-lang->entity-pattern lang-str kind-str 'f name-pat 'line lk-str id-var))
           (exclude-doc-langs
             (if (and (string=? lang-str "")
                      (not (member kind-str '("heading" "section" "doc" "document"))))
               '((not (file f "org" path))
                 (not (file f "markdown" path)))
               '()))
           (path-filters (build-path-filter-atoms '(path) exclude-tests? exclude-prefixes contains-str))
           (special-filters (build-special-attr-atoms id-var special-pairs))
           (name-filters (if (string? name-pat)
                           '()
                           (build-name-filter-atoms 'name name-contains-list name-prefixes name-all-list))))
      `(ascent-fragment
         ,@(build-scope-clause prefix)
         (limit ,lim)
         ,@(if (and (exact-integer? ctx) (> ctx 0)) `((context ,ctx)) '())
         ((answer ,name-pat path line)
          <-- ,rel
              (file f ,lang-pat path)
              ,@exclude-doc-langs
              ,@special-filters
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
    (let* ((raw-target (kw-ref kwargs '(symbols symbol names name target targets)
                               (if (pair? pos)
                                 (if (and (null? (cdr pos)) (list? (car pos)))
                                   (car pos)
                                   pos)
                                 #f)))
           (targets
             (let* ((raw-list (cond
                                ((string? raw-target) (list raw-target))
                                ((symbol? raw-target) (list (symbol->string raw-target)))
                                ((list? raw-target)
                                 (filter-map (lambda (t)
                                               (cond ((string? t) (and (> (string-length t) 0) t))
                                                     ((symbol? t) (symbol->string t))
                                                     (else #f)))
                                             raw-target))
                                (else '())))
                    (dedup '()))
               (for-each (lambda (t)
                           (unless (member t dedup)
                             (set! dedup (append dedup (list t)))))
                         raw-list)
               (if (> (length dedup) 6)
                 (let loop ((xs dedup) (n 6) (acc '()))
                   (if (or (null? xs) (<= n 0)) (reverse acc) (loop (cdr xs) (- n 1) (cons (car xs) acc))))
                 dedup)))
           (prefix (kw-ref kwargs '(path-prefix prefix scope) #f))
           (exclude-tests? (if (kw-ref kwargs '(exclude-tests? exclude-tests prod-only? prod-only) #f) #t #f))
           (exclude-prefixes (datum->string-list (kw-ref kwargs '(exclude-path-prefix exclude-prefix) #f)))
           (contains-str (kw-ref kwargs '(path-contains) #f))
           (lim (positive-int-or (kw-ref kwargs '(limit) 80) 80))
           (ctx (kw-ref kwargs '(context) #f))
           (extra-filters (build-path-filter-atoms '(path) #f exclude-prefixes contains-str))
           (scopes (if exclude-tests?
                     '(("prod" prod_path))
                     '(("prod" prod_path) ("test" is_test_path)))))
      (unless (pair? targets)
        (raise-repair-diagnostic
          'missing_impact_symbol
          "(find-impact ...) requires one or more target symbol name strings"
          "(find-impact \"SymbolName\" path-prefix: \"crates/\" exclude-tests?: #f limit: 80) | (find-impact (\"SymA\" \"SymB\") limit: 80)"
          "(find-impact \"WorkspaceSearchIndex\" path-prefix: \"crates/\")"))
      (let ((def-rules
              (map (lambda (target-str)
                     `((def_site path line)
                       <-- (entity _ f _ _ _ ,target-str line)
                           (file f _ path)))
                   targets))
            (rules
              (apply append
                (map (lambda (target-str)
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
                                    ((answer "ffi" ,scope-label c_sym path line)
                                     <-- (ffi_binds_edge ,target-str c_sym path line)
                                         (,scope-rel path)
                                         ,@extra-filters)
                                    ((answer "ffi" ,scope-label scheme_fn path line)
                                     <-- (ffi_binds_edge scheme_fn ,target-str path line)
                                         (,scope-rel path)
                                         ,@extra-filters)
                                    ((answer "ref" ,scope-label ,target-str path line)
                                     <-- (symbol_ref ,target-str path line)
                                         (,scope-rel path)
                                         (not (def_site path line))
                                         ,@extra-filters))))
                              scopes)))
                     targets))))
        `(ascent-fragment
           ,@(build-scope-clause prefix)
           (limit ,lim)
           ,@(if (and (exact-integer? ctx) (> ctx 0)) `((context ,ctx)) '())
           (relation def_site ((path string?) (line integer?)))
           ,@def-rules
           ,@rules)))))

(def (desugar-trace-path args)
  (let-values (((pos kwargs) (parse-dsl-kwargs args)))
    (let* ((raw-from (kw-ref kwargs '(from source start) (if (pair? pos) (car pos) #f)))
           (raw-to (kw-ref kwargs '(to target dst end)
                           (if (and (pair? pos) (pair? (cdr pos))) (cadr pos) #f)))
           (from-str (and raw-from (datum->attr-value-string raw-from)))
           (to-str (and raw-to (datum->attr-value-string raw-to)))
           (prefix (kw-ref kwargs '(path-prefix prefix scope) #f))
           (exclude-tests? (if (kw-ref kwargs '(exclude-tests? exclude-tests prod-only? prod-only) #f) #t #f))
           (exclude-prefixes (datum->string-list (kw-ref kwargs '(exclude-path-prefix exclude-prefix) #f)))
           (contains-str (kw-ref kwargs '(path-contains) #f))
           (lim (positive-int-or (kw-ref kwargs '(limit) 40) 40))
           (ctx (kw-ref kwargs '(context) #f))
           (path-filters (build-path-filter-atoms '(path) exclude-tests? exclude-prefixes contains-str)))
      (unless (and from-str (> (string-length from-str) 0)
                   to-str (> (string-length to-str) 0))
        (raise-repair-diagnostic
          'missing_trace_endpoints
          "(trace-path ...) requires both `from:` and `to:` symbol names"
          "(trace-path from: \"CallerFn\" to: \"TargetFn\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40)"
          "(trace-path from: \"App::submit\" to: \"request_message\" path-prefix: \"crates/\")"))
      `(ascent-fragment
         ,@(build-scope-clause prefix)
         (limit ,lim)
         ,@(if (and (exact-integer? ctx) (> ctx 0)) `((context ,ctx)) '())
         (relation link ((src string?) (dst string?) (rel string?) (path string?) (line integer?)))
         (relation fwd ((node string?)))
         (relation bwd ((node string?)))
         ((link s d "calls" path line)
          <-- (calls_symbol s d path line) ,@path-filters)
         ((link s d "impl" path line)
          <-- (impl_trait s d path line) ,@path-filters)
         ((link s d "ffi" path line)
          <-- (ffi_binds_edge s d path line) ,@path-filters)
         ((link d s "ffi" path line)
          <-- (ffi_binds_edge s d path line) ,@path-filters)
         ((link s d "imports" path line)
          <-- (imports_edge s d path line) ,@path-filters)
         ((fwd ,from-str) <-- (link ,from-str _ _ _ _))
         ((fwd dst) <-- (fwd src) (link src dst _ _ _))
         ((bwd ,to-str) <-- (link _ ,to-str _ _ _))
         ((bwd src) <-- (link src dst _ _ _) (bwd dst))
         ((answer rel src dst path line)
          <-- (link src dst rel path line)
              (fwd src)
              (bwd dst))))))

(def (desugar-solve-placement args)
  (let-values (((pos kwargs) (parse-dsl-kwargs args)))
    (let* ((prefix (kw-ref kwargs '(path-prefix prefix scope)
                           (if (and (pair? pos) (string? (car pos))) (car pos) #f)))
           (langs (datum->symbol-list (kw-ref kwargs '(use lang langs) '(rust)) '(rust)))
           (raw-limit (kw-ref kwargs '(module-limit limit max-modules slice) 'auto))
           (auto-slice? (or (eq? raw-limit 'auto)
                            (and (string? raw-limit) (string=? (string-downcase raw-limit) "auto"))))
           (scope-limit (if auto-slice? 9 (positive-int-or raw-limit 8)))
           (choices (datum->symbol-list
                      (kw-ref kwargs '(choices tiers domain) '(resident provider))
                      '(resident provider)))
           (raw-forbid (kw-ref kwargs '(forbid-hot-path forbid-hot-path-tier) #f))
           (forbid-sym (and raw-forbid
                            (not (eq? raw-forbid 'none))
                            (datum->symbol-atom raw-forbid)))
           (budget (positive-int-or (kw-ref kwargs '(state-budget budget world-limit max-worlds) 128) 128)))
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
      ((trace-path)      (desugar-trace-path (cdr form)))
      ((solve-placement) (desugar-solve-placement (cdr form)))
      ((solve-resolve)   (desugar-solve-resolve (cdr form)))
      (else form))))

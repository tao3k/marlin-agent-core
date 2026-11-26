;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional unified Search DSL powered DIRECTLY by upstream libraries,
;;; with ZERO regexes, ZERO `eval-syntax`, ZERO `:gerbil/expander` runtime
;;; dependency, 100% AOT `linked-ui` safety, and an automatic safe Pre-Tool-Call
;;; Model AST Repair Hook (`./repair.ss`):
;;;
;;;   1. `:gerbil-ascent/rules/closed-source` (`ascent-closed-dsl-parts`):
;;;      Zero-eval closed lowering for upstream `(ascent ...)` and
;;;      `(ascent-fragment ...)` queries, composed directly with the REAL
;;;      compiled `:gerbil-ascent` `ascent-fragment` objects in
;;;      `SearchLanguageRegistryValue` over ASP's universal `semantic-open-edb.v1`
;;;      `(lang, kind, language_kind)` protocol.
;;;   2. `:gerbil-ascent/program/scheme-choice` (`lower-relational-choice-program`,
;;;      `relational-choice-solve`) + `:gerbil-ascent/program/api`:
;;;      Zero-eval AOT-safe lowering & evaluation for upstream Lean-verified
;;;      Dusa POPL 2025 `(relational-choice-program ...)` with derivation
;;;      provenance (`provenance?: #t`).
;;;   3. `:gerbil-parser` (`grammar.ss` `parse-search-selector`):
;;;      Typed structured sub-syntax parsing directly in Scheme.
;;;   4. Safe Model Tool-Call AST Repair Hook (`repair.ss`):
;;;      Deterministically heals LLM S-expression slips (code fences, trailing
;;;      parens, `<-` arrows, omitted column types, missing head `(relation ...)`,
;;;      missing `(limits ...)`) with an auditable `repairs` receipt while
;;;      failing closed on legacy `@sym:`, `#.(...)`, and host callbacks.
;;;   5. Live Introspection of composed `:gerbil-ascent` Relations ->
;;;      Helm interactive TAB completion table & `workspace_search` Tool Schema.
(import (only-in :clan/poo/object .get .o .ref .slot? seal-object!)
        (only-in :std/string/misc string-contains string-trim)
        (only-in :gerbil-ascent/rules/closed-source ascent-closed-dsl-parts)
        (only-in :gerbil-ascent/program/api
                 gerbil-ascent-variable
                 gerbil-ascent-wildcard
                 gerbil-ascent-literal
                 gerbil-ascent-atom
                 gerbil-ascent-program
                 gerbil-ascent-open-session
                 gerbil-ascent-session-replace-source!
                 gerbil-ascent-session-run
                 relational-choice-solve
                 relational-program-query)
        (only-in :gerbil-ascent/program/scheme-checked
                 relational-source
                 relational-with-role
                 relational-rule/reader-origin)
        (only-in :gerbil-ascent/program/scheme-choice
                 make-relational-choice-head
                 lower-relational-choice-program)
        "./syntax.ss"
        "./grammar.ss"
        "./objects.ss"
        "./repair.ss")
(export read-ascent-search-form
        repair-ascent-search-form
        compile-search-dsl-prefilter
        compile-closed-ascent-session
        run-compiled-ascent-session
        prune-ascent-parts-for-query
        evaluate-search-dsl-over-edb
        evaluate-choice-program-over-edb
        search-dsl-completions
        search-dsl-tool-description
        search-dsl-tool-summary
        parse-search-dsl-selector
        project-language-conformance-pack
        project-all-conformance-packs
        project-language-json-schema
        extend-search-language-registry)

(def +default-max-files+ 128)
(def +default-choice-state-budget+ 1024)
(def +edb-relation-names+ '(file entity edge attr))

;;; Read and safely auto-repair a native `:gerbil-ascent` query string.
;;; Returns the repaired S-expression form.
(def (read-ascent-search-form text (registry SearchLanguageRegistryValue))
  (let-values (((form _repairs) (repair-and-read-ascent-query text registry)))
    form))

;;; Read and safely auto-repair a native `:gerbil-ascent` query string,
;;; returning `#(repaired-form repair-tags-list)`.
(def (repair-ascent-search-form text (registry SearchLanguageRegistryValue))
  (let-values (((form repairs) (repair-and-read-ascent-query text registry)))
    (vector form repairs)))

(def (parse-search-dsl-selector source)
  (search-selector-artifact-summary source))

;;; ============================================================================
;;; Rust Open EDB Prefilter Hint Extraction (Zero Regex Rules!)
;;; ============================================================================

(def (append-unique items value)
  (if (member value items) items (append items (list value))))

(def (datum->hint-string x)
  (cond
    ((string? x) x)
    ((and (pair? x) (eq? (car x) 'value) (pair? (cdr x))
          (pair? (cadr x)) (eq? (caadr x) 'quote) (symbol? (cadadr x)))
     (symbol->string (cadadr x)))
    (else #f)))

(def (relation->language-entry rel-name registry)
  (let ((entry (assq rel-name (.get registry relation-meta))))
    (and entry (cadr entry))))

(def (choose-path-prefix current candidate)
  (if (and (string? candidate) (> (string-length candidate) 0))
    (or current candidate)
    current))

(def (extract-atom-hints atom registry langs path-prefix tokens)
  (if (not (and (pair? atom) (symbol? (car atom))))
    (values langs path-prefix tokens)
    (let* ((head (car atom))
           (args (cdr atom))
           (rel-role (relation->language-entry head registry)))
      (cond
        ((and rel-role (string=? rel-role "general"))
         ;; 5-col General Language Ontology relation: (callable id f lang name line)
         (let* ((lang (and (>= (length args) 3) (datum->hint-string (list-ref args 2))))
                (name (and (>= (length args) 4) (datum->hint-string (list-ref args 3))))
                (l1 (if lang (append-unique langs lang) langs))
                (t1 (if (and name (>= (string-length name) 3)) (append-unique tokens name) tokens)))
           (values l1 path-prefix t1)))
        ((and rel-role (not (string=? rel-role "edb")))
         ;; Derived POO language relation, e.g. `(rust_fn id f "open_or_build_cached" line)`
         (let* ((l1 (append-unique langs rel-role))
                (name (and (>= (length args) 3) (datum->hint-string (caddr args))))
                (t1 (if (and name (>= (string-length name) 3)) (append-unique tokens name) tokens)))
           (values l1 path-prefix t1)))
        ((eq? head 'file)
         (let* ((lang (and (>= (length args) 2) (datum->hint-string (cadr args))))
                (path (and (>= (length args) 3) (datum->hint-string (caddr args))))
                (l1 (if lang (append-unique langs lang) langs))
                (p1 (choose-path-prefix path-prefix path))
                (t1 (if (and path (>= (string-length path) 3)) (append-unique tokens path) tokens)))
           (values l1 p1 t1)))
        ((eq? head 'string-prefix?)
         ;; Guard form `(if (string-prefix? "src/rust/" path))` -> path-prefix pushdown!
         (let* ((prefix (and (>= (length args) 2) (datum->hint-string (car args))))
                (p1 (choose-path-prefix path-prefix prefix)))
           (values langs p1 tokens)))
        ((eq? head 'entity)
         ;; (entity id file_id lang kind language_kind name line)
         (let* ((lang (and (>= (length args) 3) (datum->hint-string (list-ref args 2))))
                (name (and (>= (length args) 6) (datum->hint-string (list-ref args 5))))
                (l1 (if lang (append-unique langs lang) langs))
                (t1 (if (and name (>= (string-length name) 3)) (append-unique tokens name) tokens)))
           (values l1 path-prefix t1)))
        ((eq? head 'edge)
         ;; (edge src dst rel path line)
         (let* ((src (and (>= (length args) 1) (datum->hint-string (car args))))
                (dst (and (>= (length args) 2) (datum->hint-string (cadr args))))
                (t1 (if (and src (>= (string-length src) 3)) (append-unique tokens src) tokens))
                (t2 (if (and dst (>= (string-length dst) 3)) (append-unique t1 dst) t1)))
           (values langs path-prefix t2)))
        (else (values langs path-prefix tokens))))))

(def (walk-form-hints form registry langs path-prefix tokens)
  (cond
    ((not (pair? form)) (values langs path-prefix tokens))
    ((symbol? (car form))
     (let-values (((l1 p1 t1) (extract-atom-hints form registry langs path-prefix tokens)))
       (let loop ((rest (cdr form)) (l l1) (p p1) (t t1))
         (if (not (pair? rest))
           (values l p t)
           (let-values (((nl np nt) (walk-form-hints (car rest) registry l p t)))
             (loop (cdr rest) nl np nt))))))
    (else
     (let loop ((rest form) (l langs) (p path-prefix) (t tokens))
       (if (not (pair? rest))
         (values l p t)
         (let-values (((nl np nt) (walk-form-hints (car rest) registry l p t)))
           (loop (cdr rest) nl np nt)))))))

;;; Compile any native `:gerbil-ascent` query (after automatic safe AST repair)
;;; into a zero-regex rooted Rust Open EDB prefilter 5-vector:
;;;   #(search-edb-prefilter languages-vec path-prefix tokens-vec max-files)
(def (compile-search-dsl-prefilter text (registry SearchLanguageRegistryValue))
  (let-values (((form _repairs) (repair-and-read-ascent-query text registry)))
    (let-values (((langs path-prefix tokens) (walk-form-hints form registry '() #f '())))
      (vector 'search-edb-prefilter
              (list->vector langs)
              path-prefix
              (list->vector tokens)
              +default-max-files+))))

;;; ============================================================================
;;; 1. Zero-Eval Closed Upstream `:gerbil-ascent` Pruning, Composition & Execution
;;; ============================================================================

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

;;; Prune `fragments` to ONLY the transitive dependency cone of `query-part`,
;;; returning `(values pruned-program active-edb-names)`.
;;; Example: a `(rust_impl ...)` query shrinks 50+ rules across 8 languages down
;;; to 2 rules (`rust_impl` + `answer`) and 1 active EDB relation (`entity`).
(def (prune-ascent-parts-for-query fragments query-part)
  (let* ((all-parts (append fragments (list query-part)))
         (all-rules (apply append (map (lambda (p) (.ref p 'rules)) all-parts)))
         (needed (reachable-relation-names query-part all-rules))
         (pruned-rules
           (filter (lambda (rule)
                     (ormap (lambda (h) (hash-get needed h))
                            (rule-head-relations rule)))
                   all-rules))
         (relations (make-hash-table-eq))
         (order '()))
    (for-each
      (lambda (part)
        (for-each
          (lambda (rel)
            (let* ((name (.ref rel 'name))
                   (keep? (or (hash-get needed name) (memq name +edb-relation-names+))))
              (when keep?
                (let ((old (hash-get relations name)))
                  (when (and old (not (= (.ref old 'arity) (.ref rel 'arity))))
                    (error "incompatible ASCENT relation arity across fragments" name))
                  (unless old
                    (hash-put! relations name rel)
                    (set! order (cons name order)))))))
          (.ref part 'relations)))
      all-parts)
    (let* ((ordered (map (lambda (n) (hash-ref relations n)) (reverse order)))
           (active-edb (filter (lambda (n) (hash-get needed n)) +edb-relation-names+))
           (program (gerbil-ascent-program
                      ordered pruned-rules 100000 100000 100000 +edb-relation-names+)))
      (values program active-edb))))

(def (select-query-output-relation query-part)
  (let* ((names (map (lambda (r) (.ref r 'name)) (.ref query-part 'relations)))
         (derived (filter (lambda (n) (not (memq n +edb-relation-names+))) names)))
    (cond
      ((memq 'answer derived) 'answer)
      ((pair? derived) (car (reverse derived)))
      (else (error "ASCENT query must declare at least one derived output relation")))))

;;; Compile a repaired closed `(ascent ...)` / `(ascent-fragment ...)` form into
;;; a reusable pruned `:gerbil-ascent` session:
;;;   #(compiled-ascent-session session target-rel repairs active-edb-names)
(def (compile-closed-ascent-session form registry repairs)
  (let ((query-parts (ascent-closed-dsl-parts (list form))))
    (unless (and (pair? query-parts) (null? (cdr query-parts)))
      (error "ASCENT query rejected: only closed inert :gerbil-ascent declarations are admitted (explicit column predicates and closed guards; host callbacks forbidden)" form))
    (let* ((query-part (car query-parts))
           (target-rel (select-query-output-relation query-part)))
      (let-values (((program active-edb)
                    (prune-ascent-parts-for-query (.get registry ascent-fragments) query-part)))
        (let ((session (gerbil-ascent-open-session program)))
          (gerbil-ascent-session-run session)
          (vector 'compiled-ascent-session session target-rel repairs active-edb))))))

;;; Execute a compiled pruned `:gerbil-ascent` session against an EDB alist,
;;; updating ONLY the active EDB relations actually referenced by the pruned cone.
(def (run-compiled-ascent-session compiled edb)
  (let ((session (vector-ref compiled 1))
        (target-rel (vector-ref compiled 2))
        (repairs (vector-ref compiled 3))
        (active-edb (vector-ref compiled 4)))
    (for-each
      (lambda (rel-name)
        (gerbil-ascent-session-replace-source! session rel-name (edb-table-ref edb rel-name)))
      active-edb)
    (let* ((result (gerbil-ascent-session-run session))
           (rows ((.ref result 'rows-of) target-rel)))
      (vector 'search-result 'ascent target-rel rows repairs))))

;;; Evaluate an upstream closed `(ascent ...)` or `(ascent-fragment ...)` form
;;; strictly via `:gerbil-ascent/rules/closed-source#ascent-closed-dsl-parts`
;;; (ZERO `eval-syntax`, 100% AOT `linked-ui` safe!) with dependency-cone pruning.
(def (evaluate-closed-ascent-over-edb form edb registry repairs)
  (let ((compiled (compile-closed-ascent-session form registry repairs)))
    (run-compiled-ascent-session compiled edb)))

;;; ============================================================================
;;; 2. Zero-Eval Upstream Dusa POPL 2025 `(relational-choice-program ...)`
;;; ============================================================================

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
    ((or (exact-integer? term) (boolean? term) (char? term) (string? term))
     (gerbil-ascent-literal (normalize-choice-scalar term)))
    (else (error "invalid relational-choice term (expected ?var, ?_, scalar, or (value 'sym))" term))))

(def (lower-choice-atom atom)
  (unless (and (pair? atom) (symbol? (car atom)))
    (error "invalid relational-choice body atom" atom))
  (gerbil-ascent-atom (car atom) (map lower-choice-term (cdr atom))))

(def (make-choice-rule-origin body-atoms)
  (vector #f (make-list (length body-atoms) #f)))

;;; Evaluate an already-constructed `:gerbil-ascent` `relational-choice-program`
;;; object directly (zero parsing!) and return models with Dusa provenance.
(def (evaluate-choice-program-over-edb choice-prog target-rel (repairs '()))
  (let* ((solutions (relational-choice-solve
                      choice-prog
                      state-budget: +default-choice-state-budget+))
         (models (map (lambda (sol)
                        (vector 'choice-model
                                (relational-program-query sol target-rel)
                                (relational-program-query sol target-rel provenance?: #t)))
                      solutions)))
    (vector 'search-result 'choice target-rel models repairs)))

;;; Zero-eval AOT-safe lowering of upstream `(relational-choice-program ...)`
;;; text directly into `:gerbil-ascent/program/scheme-choice#lower-relational-choice-program`
;;; with Open EDB + POO language helper relations bound automatically.
(def (evaluate-closed-choice-over-edb form edb registry repairs)
  (let ((extra-rels '())
        (choice-rels '())
        (choice-names '())
        (plain-rules '())
        (choice-heads '())
        (input-limit 65536)
        (derived-limit 65536)
        (output-limit 65536))
    (for-each
      (lambda (clause)
        (unless (and (pair? clause) (symbol? (car clause)))
          (error "invalid relational-choice-program clause" clause))
        (case (car clause)
          ((relation)
           (let* ((name (cadr clause))
                  (cols (caddr clause))
                  (rows (if (and (>= (length clause) 4)
                                 (pair? (cadddr clause))
                                 (eq? (car (cadddr clause)) 'quote))
                          (normalize-choice-rows (cadr (cadddr clause)))
                          '())))
             (unless (memq name +edb-relation-names+)
               (set! extra-rels
                 (append extra-rels
                         (list (relational-with-role
                                 (relational-source name (length cols) rows)
                                 (if (null? rows) 'derived 'input))))))))
          ((choice)
           (let* ((name (cadr clause))
                  (cols (caddr clause)))
             (set! choice-names (append-unique choice-names name))
             (set! choice-rels
               (append choice-rels
                       (list (relational-with-role
                               (relational-source name (length cols) '())
                               'input))))))
          ((rule)
           (let* ((head (cadr clause))
                  (body (map lower-choice-atom (cddr clause)))
                  (origin (make-choice-rule-origin body)))
             (cond
               ((and (pair? head) (eq? (car head) 'closed))
                (let* ((target (cadr head))
                       (vals (map lower-choice-term (caddr head))))
                  (set! choice-heads
                    (append choice-heads
                            (list (make-relational-choice-head
                                    'closed (car target)
                                    (map lower-choice-term (cdr target))
                                    vals body origin))))))
               ((and (pair? head) (eq? (car head) 'open))
                (let* ((target (cadr head))
                       (val (lower-choice-term (caddr head))))
                  (set! choice-heads
                    (append choice-heads
                            (list (make-relational-choice-head
                                    'open (car target)
                                    (map lower-choice-term (cdr target))
                                    (list val) body origin))))))
               (else
                (set! plain-rules
                  (append plain-rules
                          (list (relational-rule/reader-origin
                                  (list (lower-choice-atom head))
                                  body origin))))))))
          ((limits)
           (when (= (length clause) 4)
             (set! input-limit (min 100000 (cadr clause)))
             (set! derived-limit (min 100000 (caddr clause)))
             (set! output-limit (min 100000 (cadddr clause)))))
          (else (error "unsupported relational-choice-program clause" clause))))
      (cdr form))
    (unless (pair? choice-names)
      (error "relational-choice-program requires at least one (choice ...) declaration"))
    ;; Bind universal 7-col `entity` & 5-col `edge` Open EDB + 8 General Language
    ;; Ontology relations + all POO per-language Special relations.
    (let* ((edb-rels
             (list
               (relational-with-role (relational-source 'file 3 (normalize-choice-rows (edb-table-ref edb 'file))) 'input)
               (relational-with-role (relational-source 'entity 7 (normalize-choice-rows (edb-table-ref edb 'entity))) 'input)
               (relational-with-role (relational-source 'edge 5 (normalize-choice-rows (edb-table-ref edb 'edge))) 'input)
               (relational-with-role (relational-source 'attr 3 (normalize-choice-rows (edb-table-ref edb 'attr))) 'input)))
           (general-ontology-specs
             '((callable     . ("function" "method"))
               (type_decl    . ("struct" "class" "enum" "trait" "interface" "type"))
               (module_decl  . ("module"))
               (import_decl  . ("import"))
               (binding_decl . ("binding" "constant"))
               (macro_decl   . ("macro"))
               (comment_node . ("doc"))
               (test_decl    . ("test"))))
           (general-helper-rels
             (map (lambda (spec)
                    (relational-with-role (relational-source (car spec) 5 '()) 'derived))
                  general-ontology-specs))
           (general-helper-rules
             (apply append
               (map (lambda (spec)
                      (let ((rel-name (car spec)))
                        (map (lambda (kind-str)
                               (let ((head (gerbil-ascent-atom rel-name
                                             (list (gerbil-ascent-variable '?id)
                                                   (gerbil-ascent-variable '?f)
                                                   (gerbil-ascent-variable '?lang)
                                                   (gerbil-ascent-variable '?name)
                                                   (gerbil-ascent-variable '?line))))
                                     (body (list (gerbil-ascent-atom 'entity
                                                   (list (gerbil-ascent-variable '?id)
                                                         (gerbil-ascent-variable '?f)
                                                         (gerbil-ascent-variable '?lang)
                                                         (gerbil-ascent-literal (string->symbol kind-str))
                                                         (gerbil-ascent-wildcard)
                                                         (gerbil-ascent-variable '?name)
                                                         (gerbil-ascent-variable '?line))))))
                                 (relational-rule/reader-origin (list head) body (make-choice-rule-origin body))))
                             (cdr spec))))
                    general-ontology-specs)))
           (poo-helper-entries
             (filter (lambda (m)
                       (and (not (string=? (cadr m) "edb"))
                            (not (string=? (cadr m) "general"))))
                     (.get registry relation-meta)))
           (poo-helper-rels
             (map (lambda (m)
                    (relational-with-role (relational-source (car m) 4 '()) 'derived))
                  poo-helper-entries))
           (poo-helper-rules
             (map (lambda (m)
                    (let* ((rel-name (car m))
                           (lang-sym (string->symbol (cadr m)))
                           (kind-pat (caddr m))
                           (kind-term (if (string? kind-pat)
                                        (gerbil-ascent-literal (string->symbol kind-pat))
                                        (gerbil-ascent-wildcard)))
                           (lang-kind-pat (cadddr m))
                           (lk-term (if (string? lang-kind-pat)
                                      (gerbil-ascent-literal (string->symbol lang-kind-pat))
                                      (gerbil-ascent-wildcard)))
                           (head (gerbil-ascent-atom rel-name
                                   (list (gerbil-ascent-variable '?id)
                                         (gerbil-ascent-variable '?f)
                                         (gerbil-ascent-variable '?name)
                                         (gerbil-ascent-variable '?line))))
                           (body (list (gerbil-ascent-atom 'entity
                                         (list (gerbil-ascent-variable '?id)
                                               (gerbil-ascent-variable '?f)
                                               (gerbil-ascent-literal lang-sym)
                                               kind-term
                                               lk-term
                                               (gerbil-ascent-variable '?name)
                                               (gerbil-ascent-variable '?line))))))
                      (relational-rule/reader-origin (list head) body (make-choice-rule-origin body))))
                  poo-helper-entries))
           (base-program
             (gerbil-ascent-program
               (append edb-rels general-helper-rels poo-helper-rels choice-rels extra-rels)
               (append general-helper-rules poo-helper-rules plain-rules)
               input-limit derived-limit output-limit
               (append +edb-relation-names+ choice-names)))
           (choice-program
             (lower-relational-choice-program base-program choice-names choice-heads)))
      (evaluate-choice-program-over-edb choice-program (car (reverse choice-names)) repairs))))

;;; Evaluate any native `:gerbil-ascent` query (`ascent`, `ascent-fragment`,
;;; `relational-choice-program`) or `:gerbil-parser` `(search-selector "...")`
;;; over an Open EDB alist streamed from ASP Rust (ZERO `eval-syntax`!,
;;; automatic safe Pre-Tool-Call Model AST Repair Hook enabled).
(def (evaluate-search-dsl-over-edb text edb (registry SearchLanguageRegistryValue))
  (let-values (((form repairs) (repair-and-read-ascent-query text registry)))
    (case (car form)
      ((ascent ascent-fragment)
       (evaluate-closed-ascent-over-edb form edb registry repairs))
      ((relational-choice-program)
       (evaluate-closed-choice-over-edb form edb registry repairs))
      ((search-selector)
       (unless (and (= (length form) 2) (string? (cadr form)))
         (error "(search-selector ...) requires one string argument" form))
       (parse-search-dsl-selector (cadr form)))
      (else (error "unsupported search form" (car form))))))

;;; ============================================================================
;;; 3. Live Introspection -> Helm TAB Completion Table & Tool Schema
;;; ============================================================================

(def (last-token-fragment text)
  (let ((trimmed (string-trim text)))
    (if (string=? trimmed "")
      ""
      (let loop ((i (- (string-length trimmed) 1)))
        (if (< i 0)
          trimmed
          (let ((ch (string-ref trimmed i)))
            (if (or (char-whitespace? ch) (char=? ch #\() (char=? ch #\)) (char=? ch #\"))
              (substring trimmed (+ i 1) (string-length trimmed))
              (loop (- i 1)))))))))

;;; Introspect REAL compiled `:gerbil-ascent` relations from the composed POO
;;; `SearchLanguageRegistry.` into Helm TAB completion entries.
(def (introspect-relation-completions registry)
  (let ((meta (.get registry relation-meta)))
    (map (lambda (rel)
           (let* ((name (.ref rel 'name))
                  (name-str (symbol->string name))
                  (sig (render-relation-signature rel))
                  (info (assq name meta))
                  (lang (if info (cadr info) "ascent"))
                  (doc (if info (list-ref info 4) "Derived :gerbil-ascent relation"))
                  (kind (if (memq name +edb-relation-names+) "edb" "relation")))
             (list name-str kind sig lang doc sig)))
         (collect-unique-relations (.get registry ascent-fragments)))))

;;; Return a rooted vector of TAB completion table rows:
;;;   #(#(form kind signature lang doc insert-text) ...)
(def (search-dsl-completions input-text (registry SearchLanguageRegistryValue))
  (let* ((trimmed (string-trim input-text))
         (needle (string-downcase (last-token-fragment trimmed)))
         (root-only? (or (string=? trimmed "") (string=? trimmed "(")))
         (entries (append (.get registry syntax-forms)
                          (introspect-relation-completions registry)))
         (matched
           (filter
             (lambda (entry)
               (let ((form (string-downcase (car entry)))
                     (kind (cadr entry))
                     (lang (string-downcase (cadddr entry)))
                     (doc (string-downcase (list-ref entry 4))))
                 (cond
                   (root-only? (or (string=? kind "ascent")
                                   (string=? kind "dusa")
                                   (string=? kind "parser")
                                   (string=? kind "edb")))
                   ((string=? needle "") #t)
                   (else (or (string-contains form needle)
                             (string-contains lang needle)
                             (string-contains doc needle))))))
             entries)))
    (list->vector (map list->vector matched))))

(def (search-dsl-tool-description (_registry SearchLanguageRegistryValue))
  (.get WorkspaceSearchToolSpecValue summary))

(def (search-dsl-tool-summary (_registry SearchLanguageRegistryValue))
  (.get WorkspaceSearchToolSpecValue summary))

;;; ============================================================================
;;; 4. Pure Functional Language Conformance Pack Projection (ASP RFC 10.36)
;;; ============================================================================

(def (contract-entry->conformance-pack entry)
  (let ((lang-sym (list-ref entry 0))
        (provider-lang (list-ref entry 1))
        (extensions (list-ref entry 2))
        (markers (list-ref entry 3))
        (kind-map (list-ref entry 4))
        (special-rels (list-ref entry 5))
        (probes (list-ref entry 6)))
    (list->hash-table
      (list
        (cons "schemaId" "agent.semantic-protocols.language-conformance-pack")
        (cons "schemaVersion" "1")
        (cons "languageId" (symbol->string lang-sym))
        (cons "providerLanguageId" provider-lang)
        (cons "extensions" extensions)
        (cons "projectMarkers" markers)
        (cons "kindMappings"
              (map (lambda (km)
                     (list->hash-table
                       (list (cons "nativeLanguageKind" (list-ref km 0))
                             (cons "generalKind" (list-ref km 1))
                             (cons "description" (list-ref km 2)))))
                   kind-map))
        (cons "specialRelations" (map symbol->string special-rels))
        (cons "probes"
              (map (lambda (probe)
                     (list->hash-table
                       (list (cons "probeId" (list-ref probe 0))
                             (cons "path" (list-ref probe 1))
                             (cons "source" (list-ref probe 2))
                             (cons "expectValid" (list-ref probe 3))
                             (cons "expectedNodes"
                                   (map (lambda (node)
                                          (list->hash-table
                                            (list (cons "kind" (list-ref node 0))
                                                  (cons "languageKind" (list-ref node 1))
                                                  (cons "name" (list-ref node 2)))))
                                        (list-ref probe 4))))))
                   probes))))))

;;; Project ONE language's POO `defasp-language` contract into structured data
;;; aligned 1:1 with ASP's `schemas/language-conformance-pack.schema.json`.
;;; Matches either the Scheme language symbol (`'scheme`, `'rust`, `'julia`) or
;;; the ASP provider language id (`"gerbil-scheme"`, `"rust"`, `"julia"`).
(def (project-language-conformance-pack lang-key (registry SearchLanguageRegistryValue))
  (let* ((needle (if (symbol? lang-key) (symbol->string lang-key) lang-key))
         (contracts (.get registry language-contracts))
         (matched
           (find (lambda (entry)
                   (or (string=? (symbol->string (list-ref entry 0)) needle)
                       (string=? (list-ref entry 1) needle)))
                 contracts)))
    (if matched
      (contract-entry->conformance-pack matched)
      (error "unknown ASP language contract in SearchLanguageRegistry" lang-key))))

;;; Project ALL native provider language contracts from `SearchLanguageRegistryValue`.
(def (project-all-conformance-packs (registry SearchLanguageRegistryValue))
  (map contract-entry->conformance-pack (.get registry language-contracts)))

(def (unique-strings items)
  (let loop ((rest items) (acc '()))
    (cond
      ((null? rest) (reverse acc))
      ((member (car rest) acc) (loop (cdr rest) acc))
      (else (loop (cdr rest) (cons (car rest) acc))))))

;;; Project ONE language's POO `defasp-language` contract into a Yuanxu-compatible
;;; JSON Schema alist (consumed directly by `:tao3k/yuanxu/modules/json-schema`
;;; `compile-schema` / `schema-contract` or exported to JSON Schema Draft 2020-12).
;;; General JSON data stays language-agnostic; this targeted schema enforces the
;;; language's exact native `languageKind` & `kind` surface dynamically!
(def (project-language-json-schema lang-key (registry SearchLanguageRegistryValue))
  (let* ((needle (if (symbol? lang-key) (symbol->string lang-key) lang-key))
         (contracts (.get registry language-contracts))
         (matched
           (find (lambda (entry)
                   (or (string=? (symbol->string (list-ref entry 0)) needle)
                       (string=? (list-ref entry 1) needle)))
                 contracts)))
    (unless matched
      (error "unknown ASP language contract in SearchLanguageRegistry" lang-key))
    (let* ((provider-lang (list-ref matched 1))
           (kind-map (list-ref matched 4))
           (native-kinds (unique-strings (map car kind-map)))
           (general-kinds (unique-strings (map cadr kind-map))))
      `((type . "object")
        (additionalProperties . #f)
        (required . ("languageId" "kind" "languageKind" "name" "line"))
        (properties
         . ((languageId   . ((type . "string") (const . ,provider-lang)))
            (kind         . ((type . "string") (enum . ,general-kinds)))
            (languageKind . ((type . "string") (enum . ,native-kinds)))
            (name         . ((type . "string") (minLength . 1)))
            (line         . ((type . "integer") (minimum . 1)))))))))

;;; ============================================================================
;;; 5. Pure Dynamic Language Extension from ASP HTTP-Forwarded Descriptors
;;; ============================================================================

(def (append-unique-by-key existing incoming key-fn)
  (let ((incoming-keys (map key-fn incoming)))
    (append (filter (lambda (item) (not (member (key-fn item) incoming-keys))) existing)
            incoming)))

;;; Pure functional POO registry extension: given `base-registry` and a rooted
;;; C-ABI provider descriptor 5-vector forwarded from ASP over HTTP:
;;;   #(lang-str provider-lang-str extensions-vec kind-map-vec special-specs-vec)
;;; where:
;;;   - `extensions-vec`   = #(".zig" ...)
;;;   - `kind-map-vec`     = #(#(native-kind general-kind doc) ...)
;;;   - `special-specs-vec`= #(#(rel-name-str kind-pat-or-#f lang-kind-pat-or-#f doc-str) ...)
;;; synthesizes REAL zero-eval `:gerbil-ascent` relations & Horn rules and
;;; returns a new sealed POO registry without recompiling Marlin or ASP.
(def (extend-search-language-registry base-registry desc-vec)
  (unless (and (vector? desc-vec) (= (vector-length desc-vec) 5))
    (error "extend-search-language-registry requires #(lang provider-lang extensions kind-map special-specs)"))
  (let* ((lang-str (vector-ref desc-vec 0))
         (lang-sym (string->symbol lang-str))
         (provider-lang (vector-ref desc-vec 1))
         (extensions (vector->list (vector-ref desc-vec 2)))
         (kind-map (map vector->list (vector->list (vector-ref desc-vec 3))))
         (special-specs (map vector->list (vector->list (vector-ref desc-vec 4))))
         (dyn-fragments
           (if (null? special-specs)
             '()
             (ascent-closed-dsl-parts
               (list
                 `(ascent-fragment
                    (relation entity
                      ((id string?) (file_id integer?) (lang string?)
                       (kind string?) (language_kind string?)
                       (name string?) (line integer?)))
                    ,@(map (lambda (spec)
                             `(relation ,(string->symbol (car spec))
                                ((id string?) (file_id integer?) (name string?) (line integer?))))
                           special-specs)
                    ,@(map (lambda (spec)
                             (let ((rel-sym (string->symbol (car spec)))
                                   (kind-pat (if (and (string? (cadr spec))
                                                      (> (string-length (cadr spec)) 0))
                                               (cadr spec)
                                               '_))
                                   (lk-pat (if (and (string? (caddr spec))
                                                    (> (string-length (caddr spec)) 0))
                                             (caddr spec)
                                             '_)))
                               `((,rel-sym id f name line)
                                 <-- (entity id f ,lang-str ,kind-pat ,lk-pat name line))))
                           special-specs))))))
         (dyn-meta
           (map (lambda (spec)
                  (let* ((rel-sym (string->symbol (car spec)))
                         (kind-pat (if (string? (cadr spec)) (cadr spec) '_))
                         (lk-pat (if (string? (caddr spec)) (caddr spec) '_))
                         (doc (list-ref spec 3)))
                    (list rel-sym lang-str kind-pat lk-pat doc)))
                special-specs))
         (existing-contract
           (find (lambda (c) (eq? (car c) lang-sym))
                 (.get base-registry language-contracts)))
         (dyn-contract
           (if existing-contract
             (list lang-sym
                   provider-lang
                   (unique-strings (append (list-ref existing-contract 2) extensions))
                   (list-ref existing-contract 3)
                   (append-unique-by-key (list-ref existing-contract 4) kind-map car)
                   (append-unique-by-key
                     (list-ref existing-contract 5)
                     (map (lambda (spec) (string->symbol (car spec))) special-specs)
                     (lambda (s) s))
                   (list-ref existing-contract 6))
             (list lang-sym
                   provider-lang
                   extensions
                   '()
                   kind-map
                   (map (lambda (spec) (string->symbol (car spec))) special-specs)
                   '())))
         (next-langs (if (memq lang-sym (.get base-registry languages))
                       (.get base-registry languages)
                       (append (.get base-registry languages) (list lang-sym))))
         (next-frags (append (.get base-registry ascent-fragments) dyn-fragments))
         (next-meta (append-unique-by-key
                      (.get base-registry relation-meta)
                      dyn-meta
                      car))
         (next-contracts (append-unique-by-key
                           (.get base-registry language-contracts)
                           (list dyn-contract)
                           (lambda (c) (symbol->string (car c))))))
    (seal-object!
      (.o languages: next-langs
          ascent-fragments: next-frags
          relation-meta: next-meta
          language-contracts: next-contracts
          syntax-forms: (.get base-registry syntax-forms)))))

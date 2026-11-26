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
(import (only-in :clan/poo/object .get .ref)
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
        evaluate-search-dsl-over-edb
        evaluate-choice-program-over-edb
        search-dsl-completions
        search-dsl-tool-description
        search-dsl-tool-summary
        parse-search-dsl-selector)

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

(def (relation->language-hint rel-name registry)
  (let ((entry (assq rel-name (.get registry relation-meta))))
    (and entry
         (let ((lang (cadr entry)))
           (and (not (string=? lang "edb")) lang)))))

(def (extract-atom-hints atom registry langs tokens)
  (if (not (and (pair? atom) (symbol? (car atom))))
    (values langs tokens)
    (let* ((head (car atom))
           (args (cdr atom))
           (rel-lang (relation->language-hint head registry)))
      (cond
        (rel-lang
         ;; Derived POO language relation, e.g. `(rust_fn id f "open_or_build_cached" line)`
         (let* ((l1 (append-unique langs rel-lang))
                (name (and (>= (length args) 3) (datum->hint-string (caddr args))))
                (t1 (if (and name (>= (string-length name) 3)) (append-unique tokens name) tokens)))
           (values l1 t1)))
        ((eq? head 'file)
         (let* ((lang (and (>= (length args) 2) (datum->hint-string (cadr args))))
                (path (and (>= (length args) 3) (datum->hint-string (caddr args))))
                (l1 (if lang (append-unique langs lang) langs))
                (t1 (if (and path (>= (string-length path) 3)) (append-unique tokens path) tokens)))
           (values l1 t1)))
        ((eq? head 'entity)
         ;; (entity id file_id lang kind language_kind name line)
         (let* ((lang (and (>= (length args) 3) (datum->hint-string (list-ref args 2))))
                (name (and (>= (length args) 6) (datum->hint-string (list-ref args 5))))
                (l1 (if lang (append-unique langs lang) langs))
                (t1 (if (and name (>= (string-length name) 3)) (append-unique tokens name) tokens)))
           (values l1 t1)))
        ((eq? head 'edge)
         ;; (edge src dst rel path line)
         (let* ((src (and (>= (length args) 1) (datum->hint-string (car args))))
                (dst (and (>= (length args) 2) (datum->hint-string (cadr args))))
                (t1 (if (and src (>= (string-length src) 3)) (append-unique tokens src) tokens))
                (t2 (if (and dst (>= (string-length dst) 3)) (append-unique t1 dst) t1)))
           (values langs t2)))
        (else (values langs tokens))))))

(def (walk-form-hints form registry langs tokens)
  (cond
    ((not (pair? form)) (values langs tokens))
    ((symbol? (car form))
     (let-values (((l1 t1) (extract-atom-hints form registry langs tokens)))
       (let loop ((rest (cdr form)) (l l1) (t t1))
         (if (not (pair? rest))
           (values l t)
           (let-values (((nl nt) (walk-form-hints (car rest) registry l t)))
             (loop (cdr rest) nl nt))))))
    (else
     (let loop ((rest form) (l langs) (t tokens))
       (if (not (pair? rest))
         (values l t)
         (let-values (((nl nt) (walk-form-hints (car rest) registry l t)))
           (loop (cdr rest) nl nt)))))))

;;; Compile any native `:gerbil-ascent` query (after automatic safe AST repair)
;;; into a zero-regex rooted Rust Open EDB prefilter 5-vector:
;;;   #(search-edb-prefilter languages-vec path-prefix tokens-vec max-files)
(def (compile-search-dsl-prefilter text (registry SearchLanguageRegistryValue))
  (let-values (((form _repairs) (repair-and-read-ascent-query text registry)))
    (let-values (((langs tokens) (walk-form-hints form registry '() '())))
      (vector 'search-edb-prefilter
              (list->vector langs)
              #f
              (list->vector tokens)
              +default-max-files+))))

;;; ============================================================================
;;; 1. Zero-Eval Closed Upstream `:gerbil-ascent` Composition & Execution
;;; ============================================================================

(def (edb-table-ref edb key)
  (let ((entry (assq key edb)))
    (if entry (cdr entry) '())))

(def (compose-ascent-parts parts)
  (let ((relations (make-hash-table-eq))
        (order '())
        (rules (apply append (map (lambda (p) (.ref p 'rules)) parts))))
    (for-each
      (lambda (part)
        (for-each
          (lambda (rel)
            (let* ((name (.ref rel 'name))
                   (old (hash-get relations name)))
              (when (and old (not (= (.ref old 'arity) (.ref rel 'arity))))
                (error "incompatible ASCENT relation arity across fragments" name))
              (unless old
                (hash-put! relations name rel)
                (set! order (cons name order)))))
          (.ref part 'relations)))
      parts)
    (let ((ordered (map (lambda (n) (hash-ref relations n)) (reverse order))))
      (gerbil-ascent-program ordered rules 100000 100000 100000 +edb-relation-names+))))

(def (select-query-output-relation query-part)
  (let* ((names (map (lambda (r) (.ref r 'name)) (.ref query-part 'relations)))
         (derived (filter (lambda (n) (not (memq n +edb-relation-names+))) names)))
    (cond
      ((memq 'answer derived) 'answer)
      ((pair? derived) (car (reverse derived)))
      (else (error "ASCENT query must declare at least one derived output relation")))))

;;; Evaluate an upstream closed `(ascent ...)` or `(ascent-fragment ...)` form
;;; strictly via `:gerbil-ascent/rules/closed-source#ascent-closed-dsl-parts`
;;; (ZERO `eval-syntax`, 100% AOT `linked-ui` safe!) composed directly with the
;;; REAL compiled `:gerbil-ascent` fragments owned by `SearchLanguageRegistryValue`.
(def (evaluate-closed-ascent-over-edb form edb registry repairs)
  (let ((query-parts (ascent-closed-dsl-parts (list form))))
    (unless (and (pair? query-parts) (null? (cdr query-parts)))
      (error "ASCENT query rejected: only closed inert :gerbil-ascent declarations are admitted (explicit column predicates and closed guards; host callbacks forbidden)" form))
    (let* ((query-part (car query-parts))
           (target-rel (select-query-output-relation query-part))
           (program (compose-ascent-parts
                      (append (.get registry ascent-fragments) (list query-part))))
           (session (gerbil-ascent-open-session program)))
      (gerbil-ascent-session-run session)
      (for-each
        (lambda (rel-name)
          (gerbil-ascent-session-replace-source! session rel-name (edb-table-ref edb rel-name)))
        +edb-relation-names+)
      (let* ((result (gerbil-ascent-session-run session))
             (rows ((.ref result 'rows-of) target-rel)))
        (vector 'search-result 'ascent target-rel rows repairs)))))

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
    ;; Bind universal 7-col `entity` & 5-col `edge` Open EDB + POO derived helper relations.
    (let* ((edb-rels
             (list
               (relational-with-role (relational-source 'file 3 (normalize-choice-rows (edb-table-ref edb 'file))) 'input)
               (relational-with-role (relational-source 'entity 7 (normalize-choice-rows (edb-table-ref edb 'entity))) 'input)
               (relational-with-role (relational-source 'edge 5 (normalize-choice-rows (edb-table-ref edb 'edge))) 'input)
               (relational-with-role (relational-source 'attr 3 (normalize-choice-rows (edb-table-ref edb 'attr))) 'input)))
           (poo-helper-entries
             (filter (lambda (m) (not (string=? (cadr m) "edb")))
                     (.get registry relation-meta)))
           (poo-helper-rels
             (map (lambda (m)
                    (relational-with-role (relational-source (car m) 4 '()) 'derived))
                  poo-helper-entries))
           (poo-helper-rules
             (map (lambda (m)
                    (let* ((rel-name (car m))
                           (lang-sym (string->symbol (cadr m)))
                           (kind-sym (string->symbol (caddr m)))
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
                                               (gerbil-ascent-literal kind-sym)
                                               lk-term
                                               (gerbil-ascent-variable '?name)
                                               (gerbil-ascent-variable '?line))))))
                      (relational-rule/reader-origin (list head) body (make-choice-rule-origin body))))
                  poo-helper-entries))
           (base-program
             (gerbil-ascent-program
               (append edb-rels poo-helper-rels choice-rels extra-rels)
               (append poo-helper-rules plain-rules)
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

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Hygienic Gerbil Scheme metaprogramming (`defsyntax` / `defrules`) built
;;; directly on the upstream `:gerbil-ascent` library and `:clan/poo/object`.
;;;
;;; Two-Plane Orthogonal Design (ASP RFC 10.36):
;;;   - Native language providers own low-level lexical syntax (comment openers,
;;;     delimiter pairs, string quotes) and report `ProviderSyntaxSurface` up.
;;;   - Gerbil Scheme owns the General Language Ontology (`callable`, `type_decl`,
;;;     `module_decl`, `import_decl`, `binding_decl`, `macro_decl`, `comment_node`,
;;;     `test_decl`) and per-language POO extensions (`defasp-language` via `.mix`):
;;;       1. Native compiler `language_kind` -> General Ontology `kind` mappings
;;;       2. Language-specific special `:gerbil-ascent` relations (`rust_impl`,
;;;          `scheme_poo_def`, `julia_macro`, `ts_interface`, `py_class`, ...)
;;;       3. Golden conformance probes projected to ASP's black-box provider harness
(import (only-in :clan/poo/object .def .get .o .ref seal-object!)
        (only-in :gerbil-ascent/program/syntax ascent ascent-fragment)
        (only-in :gerbil-ascent/program/api
                 relational-program
                 relational-fragment
                 relational-choice-program
                 relational-choice-solve))
(export (import: :gerbil-ascent/program/syntax)
        relational-program
        relational-fragment
        relational-choice-program
        relational-choice-solve
        defasp-language
        defsearch-language
        defsearch-tool-spec
        collect-unique-relations
        render-search-spec-summary
        render-relation-signature)

;;; Deduplicate compiled `:gerbil-ascent` relation objects across POO fragments
;;; preserving declaration order.
(def (collect-unique-relations fragments)
  (let ((seen (make-hash-table-eq))
        (ordered '()))
    (for-each
      (lambda (frag)
        (for-each
          (lambda (rel)
            (let ((name (.ref rel 'name)))
              (unless (hash-get seen name)
                (hash-put! seen name #t)
                (set! ordered (cons rel ordered)))))
          (.ref frag 'relations)))
      fragments)
    (reverse ordered)))

(def (render-relation-signature rel)
  (let* ((name (.ref rel 'name))
         (arity (.ref rel 'arity))
         (cols (let loop ((i 0) (acc '()))
                 (if (= i arity)
                   (reverse acc)
                   (loop (+ i 1)
                         (cons (string->symbol
                                 (call-with-output-string
                                   (lambda (p) (display "c" p) (display i p))))
                               acc))))))
    (call-with-output-string
      (lambda (port) (write (cons name cols) port)))))

(def (render-search-spec-summary form-heads edb-rels lang-names)
  (call-with-output-string
    (lambda (port)
      (display "Unified 3-tier search, AST reader & constraint solver (prefer over bash). SEQUENTIAL DAG: 1 targeted call on Turn 1 (never pair workspace-map with find-symbols; Turn 1 returns workspace_layers, dsl_tips & next_dag_templates; Turn 2+ auto-compacts & reuses M1 facts). Omit exclude-tests?: #t for tests.\nForms: " port)
      (display (string-join
                 (map (lambda (h)
                        (call-with-output-string
                          (lambda (p) (write (list h '...) p))))
                      form-heads)
                 ", ")
               port)
      (display ". Langs: " port)
      (display (string-join lang-names ",") port)
      (display ".\n" port)
      (display "L1 (start here): (find-symbols name-contains: (\"stemA\" \"stemB\") lang: \"any\" limit: 60) [CamelHumps or name-all:] | (find-impact (\"SymA\" \"SymB\") limit: 120) [one-shot cross-layer DEFINITIONS, CROSS-FFI, affected_layers, PROD & TEST] | (find-collisions kind: (\"struct\" \"fn\") path-prefix: \"crates/\" lang: \"rust\" min-files: 2 exclude-tests?: #t) | (find-callers \"f\") | (find-impls \"T\") | (trace-path from: \"A\" to: \"B\") | (ascent-fragment ...) EDB:" port)
      (display (string-join (map symbol->string edb-rels) ",") port)
      (display "+verified_tier (plain vars).\n" port)
      (display "L0 (<1ms): (read-file \"p\" signature-only?: #t) | (read-file \"p\" symbols: (\"A\" \"B\") signature-only?: #t) | (read-file \"p\" symbol: \"F\") | (read-file \"p\" cursor: 1 max-lines: 360) | (file-outline \"p/\" projection: \"kind-summary|callable-skeleton\") | (brute-search \"a|b\" path-prefix: \"d/\" exclude-tests?: #t limit: 24).\n" port)
      (display "L2 Dusa (1 call): (solve-placement path-prefix: \"<c>/src/\" module-limit: auto choices: (resident provider) forbid-hot-path: provider) | (solve-resolve path-prefix: \"<c>/src/\" exclude-tests?: #t) | (search-choice-program ... hot-path-module provenance?: #t) (?vars)." port))))

;;; Declare a full POO language extension over ASP's General Language Ontology
;;; (ZERO regexes, ZERO duplicated lexical delimiter/comment rules!):
;;;   - `provider-language`: canonical ASP provider `languageId`
;;;   - `extensions` / `project-markers`: workspace routing anchors
;;;   - `kind-map`: native AST `language_kind` -> General Ontology `kind` mapping
;;;   - `relations`: language-specific `:gerbil-ascent` POO extension relations
;;;   - `conformance-probes`: minimal golden probes projected to ASP's harness
(defrules defasp-language
  (languages provider-language extensions project-markers kind-map relations conformance-probes)
  ((_ FragmentName
      (languages lang-sym ...)
      (provider-language provider-lang-str)
      (extensions ext-str ...)
      (project-markers marker-str ...)
      (kind-map (native-kind general-kind kind-doc) ...)
      (relations (rel-name lang-str kind-str lang-kind-pat doc-str) ...)
      (conformance-probes (probe-id probe-path probe-src probe-valid? ((exp-kind exp-lk exp-name) ...)) ...))
   (.def FragmentName
     (languages => append '(lang-sym ...))
     (ascent-fragments => append
       (list
         (ascent-fragment
           (relation entity
             ((id string?) (file_id integer?) (lang string?)
              (kind string?) (language_kind string?)
              (name string?) (line integer?)))
           (relation rel-name ((id string?) (file_id integer?) (name string?) (line integer?))) ...
           ((rel-name id f name line)
            <-- (entity id f lang-str kind-str lang-kind-pat name line)) ...)))
     (relation-meta => append
       (list (list 'rel-name lang-str 'kind-str 'lang-kind-pat doc-str) ...))
     (language-contracts => append
       (list
         (list (car '(lang-sym ...))
               provider-lang-str
               (list ext-str ...)
               (list marker-str ...)
               (list (list native-kind general-kind kind-doc) ...)
               (list 'rel-name ...)
               (list (list probe-id probe-path probe-src probe-valid?
                           (list (list exp-kind exp-lk exp-name) ...))
                     ...)))))))

;;; Lightweight POO fragment macro for embedded document/model languages
;;; (SysML, Quint, Org) without an external HTTP compiler server.
(defrules defsearch-language (languages relations)
  ((_ FragmentName
      (languages lang-sym ...)
      (relations (rel-name lang-str kind-str lang-kind-pat doc-str) ...))
   (.def FragmentName
     (languages => append '(lang-sym ...))
     (ascent-fragments => append
       (list
         (ascent-fragment
           (relation entity
             ((id string?) (file_id integer?) (lang string?)
              (kind string?) (language_kind string?)
              (name string?) (line integer?)))
           (relation rel-name ((id string?) (file_id integer?) (name string?) (line integer?))) ...
           ((rel-name id f name line)
            <-- (entity id f lang-str kind-str lang-kind-pat name line)) ...)))
     (relation-meta => append
       (list (list 'rel-name lang-str 'kind-str 'lang-kind-pat doc-str) ...)))))

;;; Compile-time `defsyntax` projecting `WorkspaceSearchToolSpecValue` from
;;; real `:gerbil-ascent` forms, Open EDB relations, and the POO registry.
(defsyntax (defsearch-tool-spec stx)
  (syntax-case stx (forms edb registry)
    ((_ SpecName
        (forms form-head ...)
        (edb edb-rel ...)
        (registry reg-expr))
     (identifier? #'SpecName)
     #'(def SpecName
         (let* ((reg reg-expr)
                (form-heads '(form-head ...))
                (edb-rels '(edb-rel ...))
                (lang-names (map symbol->string (.get reg languages)))
                (relations (collect-unique-relations (.get reg ascent-fragments)))
                (relation-names (map (lambda (r) (.ref r 'name)) relations))
                (summary-text (render-search-spec-summary form-heads edb-rels lang-names)))
           (seal-object!
             (.o forms: form-heads
                 edb-relations: edb-rels
                 derived-relations: relation-names
                 languages: lang-names
                 summary: summary-text)))))
    (_ (raise-syntax-error #f "invalid defsearch-tool-spec form" stx))))

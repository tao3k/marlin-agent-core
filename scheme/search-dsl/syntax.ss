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
        render-l0-search-tool-summary
        render-l1-structural-tool-summary
        render-l2-constraint-tool-summary
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

(def (render-l0-search-tool-summary lang-names)
  (call-with-output-string
    (lambda (port)
      (display "PRIMARY pure-Rust RAM codebase topology, search, AST skeleton & file reader (<1ms, lock-free parallel; ALWAYS prefer over bash/rg/grep/find/ls/cat). Batch 3-6 independent read-only calls in PARALLEL per turn (combining workspace_search, structural_query, and constraint_solve in the same turn). Langs: " port)
      (display (string-join lang-names ",") port)
      (display ".\nForms:\n" port)
      (display "1) (workspace-map \"crates/\" depth: 2 exclude-tests?: #t limit: 32) — <2KB crate/package bird's-eye map (files, languages, AST hydration %, entry files, key types). ALWAYS use this instead of `ls` or wide `(file-outline \"crates/\")` on Turn 1!\n" port)
      (display "2) (brute-search \"pat\" path-prefix: \"crates/\" exclude-tests?: #t exclude-path-prefix: \"tests/\" lang: \"rust\" context: 2 limit: 24) — trigram/BM25/regex (|, .*, [a-z] work automatically; use \"re:^...\" for multiline anchors). Avoid empty \"\" scans.\n" port)
      (display "3) (file-outline \"path-or-dir/\" projection: \"callable-skeleton|struct-enum-detail|kind-summary\" lang: \"rust\" kind: (\"struct\" \"enum\") name-contains: \"stem\" exclude-tests?: #t limit: 60 max-per-file: 12) — zero-I/O AST spans+signatures, inline struct fields/enum variants, or compact kind inventory (RAM-accurate even on unhydrated files).\n" port)
      (display "4) (read-file \"path/to/file.rs\" symbols: (\"StructA\" \"fn_b\") context: 25) or (read-file \"path/to/file.rs\" symbol: \"Fn\") or (read-file \"path/to/file.rs\" cursor: 1 max-lines: 120) — zero-I/O RAM reader (prefer symbol:/symbols: to fetch exact bodies in 1 pass; cursor:/start: pages max 200 lines).\n" port)
      (display "For cross-layer impact analysis, call/dependency path tracing, ASP special-facet symbol discovery, or collisions invoke `structural_query` directly in parallel. For finite-choice disambiguation or placement solving invoke `constraint_solve` directly in parallel." port))))

(def (render-l1-structural-tool-summary edb-rels lang-names)
  (call-with-output-string
    (lambda (port)
      (display "Structural AST & cross-language graph query engine (:gerbil-ascent over ASP Open EDB). Invoke DIRECTLY in parallel on Turn 1-2 for complete blast radius (defs + callers + impls + imports + FFI c-define bridges + 100% RAM refs), conjunctive/ASP-special symbol discovery, call/dependency path tracing, collisions, callers, and trait impls. Langs: " port)
      (display (string-join lang-names ",") port)
      (display ".\nHigh-level forms (all accept path-prefix:, exclude-tests?: #t, exclude-path-prefix:, limit:, context:):\n" port)
      (display "1) (find-impact \"SymbolName\" path-prefix: \"crates/\" limit: 60) — ONE-SHOT complete def/caller/impl/import/ffi/ref blast radius tagged prod vs test\n" port)
      (display "2) (find-symbols name-all: (\"search\" \"request\") lang: \"any\" exclude-tests?: #t limit: 60) — fast conjunctive AND symbol discovery (use name-all: for AND, name-contains: for OR, exact-name: for exact match, kind: \"struct|fn|enum|trait|any\", special: ((visibility \"pub\") (async #t)) for ASP language facets like visibility/async/unsafe/exported/moduleRole)\n" port)
      (display "3) (trace-path from: \"CallerFn\" to: \"TargetFn\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40) — multi-hop call/impl/FFI/import subgraph connecting `from:` to `to:` in 1 query\n" port)
      (display "4) (find-collisions kind: \"struct\" path-prefix: \"crates/\" lang: \"rust\" min-files: 2 exclude-tests?: #t limit: 40) — niche-diverse cross-file duplicates\n" port)
      (display "5) (find-callers \"fn_name\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40 context: 2) / (find-impls \"TraitName\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40)\n" port)
      (display "Custom Datalog: (ascent-fragment (scope path-prefix: \"crates/\") (limit 40) ((answer ...) <-- ...)) over EDB " port)
      (display (string-join (map symbol->string edb-rels) ",") port)
      (display " + is_test_path/prod_path/path_has_prefix/path_contains/name_contains/name_has_prefix/symbol_ref + rust_struct/rust_fn/calls_symbol/impl_trait/ffi_binds_edge/scheme_def/verified_tier (plain vars, no ?)." port))))

(def (render-l2-constraint-tool-summary)
  (call-with-output-string
    (lambda (port)
      (display "Dusa Finite-Choice Constraint Solver over workspace modules & symbols. Invoke DIRECTLY in parallel on Turn 1-2 (no prior directory scan needed) whenever a task asks for architecture placement solving (assigning modules to tiers under forbidden constraints -> exact SAT world counts, forced [deduction] vs free [branch] provenance, concrete witnesses) or multi-definition callsite disambiguation.\n" port)
      (display "High-level forms (compile directly to Dusa search-choice-program):\n" port)
      (display "1) (solve-placement path-prefix: \"crates/marlin-agent-search-engine/src/\" module-limit: 6 choices: (resident provider) forbid-hot-path: provider state-budget: 64) — module-limit: bounds modules in slice; state-budget: bounds search states (hot-path rules use 100% file-path predicates)\n" port)
      (display "2) (solve-resolve path-prefix: \"crates/marlin-agent-search-engine/src/\" module-limit: 6 exclude-tests?: #t state-budget: 32)\n" port)
      (display "Custom Dusa: (search-choice-program (use rust) (scope path-prefix: \"...\" limit: 6) (choice tier (m lvl)) (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m)) (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m provider)) (solve tier state-budget: 64 provenance?: #t)) (vars MUST start with ?; built-ins: module-in-scope, hot-path-module, module-depends, ref-at, same-module-def, workspace-def, in-test-file, verified-tier)." port))))

(def (render-search-spec-summary form-heads edb-rels lang-names)
  (call-with-output-string
    (lambda (port)
      (display "PRIMARY codebase search, outline & read tool (ALWAYS prefer over bash/rg/grep/find/ls/sed/cat). Unified 3-tier search over resident RAM index & ASP Open EDB. Issue multiple independent calls in PARALLEL in one turn to finish in 2-4 rounds! Forms: " port)
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
      (display "[L0 Pure-Rust RAM (<1ms, lock-free parallel)] " port)
      (display "1) (brute-search \"pat\" path-prefix: \"crates/\" exclude-path-prefix: \"tests/\" lang: \"rust\" context: 2 limit: 32) — multiline regex \"re:...\" enabled by default; list files with \"\". " port)
      (display "2) (file-outline \"path-or-dir/\" limit: 80 max-per-file: 12) — instant AST symbol map (fairly distributed across files in dirs); use instead of regex scanning! " port)
      (display "3) (read-file \"path/to/file.rs\" start: 1 end: 120) or (read-file \"path/to/file.rs\" around: 320 context: 30) — zero-I/O RAM line-range reader (up to 200 lines/call); NEVER use re:. to read a file.\n" port)
      (display "[L1 Structural Datalog (:gerbil-ascent)] EDB: " port)
      (display (string-join (map symbol->string edb-rels) ",") port)
      (display " => file(fid,lang,path) entity(id,fid,lang,kind,lk,name,line) edge(src,dst,rel,path,line) attr(id,k,v). " port)
      (display "Ontology: callable/type_decl(id,fid,lang,name,line), contains_edge/imports_edge/calls_edge/implements_edge(src,dst,path,line), calls_symbol(caller_name,callee_name,path,line), impl_trait(type_name,trait_name,path,line), entity_locator(id,loc), depends_on(src,dst). " port)
      (display "Special: rust_fn/rust_method/rust_impl/rust_struct/rust_enum/rust_trait/rust_macro(id,fid,name,line), scheme_def/scheme_poo_def, py_fn/py_class, ts_fn/ts_interface, julia_fn/julia_macro. " port)
      (display "First-class syntax: omit (relation ...), plain vars (no ?), quoted lang (\"rust\"), top-level (scope path-prefix: \"...\"), (limit N), (context N) (auto-attaches RAM source snippets for <=20 hits!), negation (not (...)), disjunction (or c1 c2), and guards (= k \"v\") / (!= a b) / (not (= a b)) / (< a b). Example: (ascent-fragment (scope path-prefix: \"crates/\") (limit 40) ((answer name p1 p2) <-- (rust_struct _ f1 name _) (rust_struct _ f2 name _) (file f1 \"rust\" p1) (file f2 \"rust\" p2) (!= f1 f2))).\n" port)
      (display "[L2 Dusa Finite-Choice Constraint Solver] " port)
      (display "Use ONLY for finite-choice disambiguation & architecture constraints (vars MUST start with ?; no != guards). Keep (scope ... limit: N) small (e.g. limit: 4..8) because k unconstrained choices yield 2^k worlds! Built-ins: ref-at(?call,?name), same-module-def(?call,?name,?def), workspace-def(?name,?def), in-test-file(?def), module-in-scope(?m), hot-path-module(?m), module-depends(?m,?dst). Example: (search-choice-program (use rust) (scope path-prefix: \"crates/marlin-agent-search-engine/src/\" limit: 6) (choice tier (m lvl)) (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m)) (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m provider)) (solve tier state-budget: 64 provenance?: #t))." port))))

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
                (summary-text (render-search-spec-summary form-heads edb-rels lang-names))
                (l0-text (render-l0-search-tool-summary lang-names))
                (l1-text (render-l1-structural-tool-summary edb-rels lang-names))
                (l2-text (render-l2-constraint-tool-summary)))
           (seal-object!
             (.o forms: form-heads
                 edb-relations: edb-rels
                 derived-relations: relation-names
                 languages: lang-names
                 summary: summary-text
                 l0-summary: l0-text
                 l1-summary: l1-text
                 l2-summary: l2-text)))))
    (_ (raise-syntax-error #f "invalid defsearch-tool-spec form" stx))))

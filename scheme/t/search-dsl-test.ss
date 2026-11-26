;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Regression suite for Marlin's unified Search DSL built directly on
;;; upstream `:gerbil-ascent` (zero-eval closed-source + Dusa finite-choice),
;;; `:gerbil-parser`, POO `defsearch-language` over ASP's zero-regex
;;; `semantic-open-edb.v1` `(lang, kind, language_kind)` protocol, and the
;;; Safe Pre-Tool-Call Model AST Repair Hook (`repair.ss`).
(import :std/test
        (only-in :std/string/misc string-contains)
        (only-in :clan/poo/object .get .ref)
        (only-in :gerbil-ascent/rules/closed-source ascent-closed-dsl-parts)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :marlin-agent-core/search-dsl/syntax
                 relational-choice-program)
        (only-in :marlin-agent-core/search-dsl/extract
                 extract-open-edb-facts-batch)
        (only-in :marlin-agent-core/search-dsl/objects
                 SearchLanguageRegistryValue)
        (only-in :marlin-agent-core/search-dsl/funcs
                 read-ascent-search-form
                 repair-ascent-search-form
                 compile-search-dsl-prefilter
                 compile-closed-ascent-session
                 run-compiled-ascent-session
                 prune-ascent-parts-for-query
                 evaluate-search-dsl-over-edb
                 evaluate-choice-program-over-edb
                 search-dsl-completions
                 parse-search-dsl-selector
                 project-language-conformance-pack
                 project-all-conformance-packs
                 project-language-json-schema
                 extend-search-language-registry))
(export search-dsl-test)

;;; Sample Open EDB matching ASP `schemas/semantic-open-edb.v1.schema.json`:
;;;   file(id, lang, path)
;;;   entity(id, file_id, lang, kind, language_kind, name, line)
;;;   edge(src, dst, rel, path, line)
;;;   attr(id, key, val)
(def sample-edb
  '((file . ((1 "rust" "crates/search/src/index.rs")
             (2 "rust" "crates/tooling/src/search_tool.rs")
             (3 "scheme" "scheme/search-dsl/objects.ss")
             (4 "sysml" "dev/sysml/search-engine.sysml")))
    (entity . (("e1" 1 "rust"   "function" "fn_item"         "open_or_build_cached"         42)
               ("e2" 2 "rust"   "function" "fn_item"         "execute_resident_search_tool" 90)
               ("e3" 2 "rust"   "item"     "impl_item"       "ResidentSearchToolHandler"    120)
               ("e4" 2 "rust"   "macro"    "macro_rules"     "register_search_tool"         140)
               ("e5" 3 "scheme" "item"     "poo_def"         "RustSearchFragment."          110)
               ("e6" 4 "sysml"  "item"     "requirement_def" "nativeRustIntegration"        236)))
    (edge . (("e2" "e1" "imports"    "crates/tooling/src/search_tool.rs" 12)
             ("e3" "ToolRuntime" "implements" "crates/tooling/src/search_tool.rs" 120)
             ("e5" "e2" "configures" "scheme/search-dsl/objects.ss"      110)))
    (attr . (("e1" "visibility" "pub")
             ("e3" "trait"      "ToolRuntime")))))

(def search-dsl-test
  (test-suite "Marlin Native :gerbil-ascent + :gerbil-parser + POO + AST Repair Hook"

    (test-case "Yuanxu scope export and defsearch-tool-spec introspected summary"
      (let* ((dsl (.get marlin-agent-core.init.exports Marlin search-dsl))
             (desc ((.get dsl tool-description))))
        (check (eq? (.get dsl identity) 'marlin/search-dsl) => #t)
        (check (string? desc) => #t)
        (check (<= (string-length desc) 256) => #t)
        (check (and (string-contains desc "(ascent-fragment ...)")
                    (string-contains desc "(relational-choice-program ...)")
                    (string-contains desc "rust")
                    (string-contains desc "scheme")
                    (string-contains desc "sysml")
                    #t)
               => #t)))

    (test-case "Fail-closed security rejects legacy @sym:, pseudo S-exp, read-eval, and host callbacks"
      (check-exception (read-ascent-search-form "@lang:rust @sym:open_or_build_cached") true)
      (check-exception (read-ascent-search-form "(search (where (file ?f ?l ?p)) (return ?p))") true)
      (check-exception (read-ascent-search-form "#.(+ 1 2)") true)
      (check-exception (read-ascent-search-form "(ascent-fragment (relation a ((x string?))) \"unterminated") true)
      ;; Closed-source must fail-closed on arbitrary Scheme callbacks in `(if ...)`!
      (check-exception
        (evaluate-search-dsl-over-edb
          "(ascent-fragment
             (relation answer ((path string?)))
             ((answer path) <-- (file f \"rust\" path) (if (f) (display f))))"
          sample-edb)
        true))

    (test-case "Zero-regex prefilter compiler derives Rust language + tokens from POO relation (rust_fn)"
      (let* ((pref (compile-search-dsl-prefilter
                     "(ascent-fragment
                        (relation answer ((path string?) (line integer?)))
                        ((answer path line) <-- (file f \"rust\" path) (rust_fn id f \"open_or_build_cached\" line)))")))
        (check (vector-length pref) => 5)
        (check (vector-ref pref 0) => 'search-edb-prefilter)
        (check (vector-ref pref 1) => #("rust"))
        (check (vector-ref pref 3) => #("open_or_build_cached"))))

    (test-case "POO relations distinguish native language_kind (rust_impl, rust_macro, scheme_poo_def, sysml_requirement)"
      (let ((impl-res (evaluate-search-dsl-over-edb
                        "(ascent-fragment
                           (relation answer ((name string?) (trait_name string?) (line integer?)))
                           ((answer name trait_name line)
                            <-- (rust_impl id f name line)
                                (attr id \"trait\" trait_name)))"
                        sample-edb))
            (macro-res (evaluate-search-dsl-over-edb
                         "(ascent-fragment
                            (relation answer ((path string?) (name string?)))
                            ((answer path name)
                             <-- (file f \"rust\" path)
                                 (rust_macro id f name _)))"
                         sample-edb))
            (poo-res (evaluate-search-dsl-over-edb
                       "(ascent-fragment
                          (relation answer ((name string?) (line integer?)))
                          ((answer name line) <-- (scheme_poo_def id f name line)))"
                       sample-edb))
            (sysml-res (evaluate-search-dsl-over-edb
                         "(ascent-fragment
                            (relation answer ((name string?)))
                            ((answer name) <-- (sysml_requirement id f name _)))"
                         sample-edb)))
        (check (vector-ref impl-res 3) => '(("ResidentSearchToolHandler" "ToolRuntime" 120)))
        (check (vector-ref impl-res 4) => '())
        (check (vector-ref macro-res 3) => '(("crates/tooling/src/search_tool.rs" "register_search_tool")))
        (check (vector-ref poo-res 3) => '(("RustSearchFragment." 110)))
        (check (vector-ref sysml-res 3) => '(("nativeRustIntegration")))))

    (test-case "Safe Pre-Tool-Call Model AST Repair Hook heals fences, parens, arrows, missing decls & column predicates"
      ;; 1. Markdown fence + missing closing paren + omitted `(relation answer ...)`
      (let ((r1 (evaluate-search-dsl-over-edb
                  "```scheme\n(ascent-fragment\n  ((answer path name line)\n   <-- (file f \"rust\" path)\n       (rust_impl id f name line))\n```"
                  sample-edb)))
        (check (vector-ref r1 0) => 'search-result)
        (check (vector-ref r1 1) => 'ascent)
        (check (vector-ref r1 2) => 'answer)
        (check (vector-ref r1 3) => '(("crates/tooling/src/search_tool.rs" "ResidentSearchToolHandler" 120)))
        (let ((tags (vector-ref r1 4)))
          (check (and (memq 'strip-code-fence tags) #t) => #t)
          (check (and (memq 'close-missing-parens tags) #t) => #t)
          (check (and (memq 'synthesize-head-relation tags) #t) => #t)))
      ;; 2. Untyped columns + `<-` arrow typo + extra trailing `)`
      (let ((r2 (evaluate-search-dsl-over-edb
                  "(ascent-fragment
                     (relation answer (path name line))
                     ((answer path name line) <- (file f \"rust\" path) (rust_fn id f name line) (if (line) (> line 50)))))"
                  sample-edb)))
        (check (vector-ref r2 3) => '(("crates/tooling/src/search_tool.rs" "execute_resident_search_tool" 90)))
        (let ((tags (vector-ref r2 4)))
          (check (and (memq 'trim-extra-closing-parens tags) #t) => #t)
          (check (and (memq 'normalize-rule-arrow tags) #t) => #t)
          (check (and (memq 'infer-column-predicates tags) #t) => #t)))
      ;; 3. Bare rule without `(ascent-fragment ...)` wrapper
      (let ((r3 (evaluate-search-dsl-over-edb
                  "((answer name) <-- (scheme_poo_def _ _ name _))"
                  sample-edb)))
        (check (vector-ref r3 3) => '(("RustSearchFragment.")))
        (let ((tags (vector-ref r3 4)))
          (check (and (memq 'wrap-bare-rule tags) #t) => #t)
          (check (and (memq 'synthesize-head-relation tags) #t) => #t)))
      ;; 4. Missing `(limits ...)` and bare `'hot` / `'warm` values in Dusa choice program
      (let ((r4 (evaluate-search-dsl-over-edb
                  "(relational-choice-program
                     (choice tier (f t))
                     (rule (closed (tier ?f) ('hot 'warm))
                           (file ?f 'rust))
                     (rule (closed (tier ?f) ('hot))
                           (rust_fn ?_ ?f 'open_or_build_cached)))"
                  sample-edb)))
        (check (vector-ref r4 1) => 'choice)
        (check (length (vector-ref r4 3)) => 2)
        (let ((tags (vector-ref r4 4)))
          (check (and (memq 'normalize-choice-values tags) #t) => #t)
          (check (and (memq 'pad-relation-wildcards tags) #t) => #t)
          (check (and (memq 'append-default-limits tags) #t) => #t)))
      ;; 5. Omitted trailing wildcards on 5-col `edge` (3-col shorthand `(edge a b "imports")`)
      (let ((r5 (evaluate-search-dsl-over-edb
                  "(ascent-fragment
                     ((answer a b) <-- (edge a b \"imports\")))"
                  sample-edb)))
        (check (vector-ref r5 3) => '(("e2" "e1")))
        (let ((tags (vector-ref r5 4)))
          (check (and (memq 'pad-relation-wildcards tags) #t) => #t)
          (check (and (memq 'synthesize-head-relation tags) #t) => #t)))
      ;; 6. Fail-closed unknown relation produces POO-registry "did you mean?" diagnostic
      (let ((err-msg
              (with-catch
                (lambda (e) (error-message e))
                (lambda ()
                  (read-ascent-search-form
                    "(ascent-fragment ((answer n) <-- (rust_function _ _ n _)))")
                  ""))))
        (check (and (string-contains err-msg "unknown :gerbil-ascent relation")
                    (string-contains err-msg "did you mean:")
                    (string-contains err-msg "rust_fn")
                    #t)
               => #t)))

    (test-case "Real upstream (ascent-fragment ...) evaluates recursive multi-hop closure over 5-col edges"
      (let ((res (evaluate-search-dsl-over-edb
                   "(ascent-fragment
                      (relation reach ((a string?) (b string?)))
                      ((reach a b) <-- (edge a b _ _ _))
                      ((reach a c) <-- (reach a b) (edge b c _ _ _)))"
                   sample-edb)))
        (check (vector-ref res 2) => 'reach)
        (check (and (member '("e5" "e1") (vector-ref res 3)) #t) => #t)))

    (test-case "Real upstream Dusa POPL 2025 (relational-choice-program ...) works as direct macro and zero-eval text"
      (let* ((direct-prog
               (relational-choice-program
                 (relation file (id lang path) '((1 rust index) (2 rust tool)))
                 (relation entity (id fid lang kind lkind name line) '((e1 1 rust function fn_item open 42)))
                 (choice tier (f t))
                 (rule (closed (tier ?f) ((value 'hot) (value 'warm)))
                       (file ?f (value 'rust) ?_))
                 (rule (closed (tier ?f) ((value 'hot)))
                       (entity ?_ ?f (value 'rust) (value 'function) ?_ (value 'open) ?_))
                 (limits 1000 1000 1000)))
             (direct-res (evaluate-choice-program-over-edb direct-prog 'tier))
             (text-res (evaluate-search-dsl-over-edb
                         "(relational-choice-program
                            (choice tier (f t))
                            (rule (closed (tier ?f) ((value 'hot) (value 'warm)))
                                  (file ?f (value 'rust) ?_))
                            (rule (closed (tier ?f) ((value 'hot)))
                                  (rust_impl ?_ ?f (value 'ResidentSearchToolHandler) ?_))
                            (limits 4096 4096 4096))"
                         sample-edb)))
        (check (vector-ref direct-res 1) => 'choice)
        (check (length (vector-ref direct-res 3)) => 2)
        (check (vector-ref text-res 1) => 'choice)
        (let ((models (vector-ref text-res 3)))
          (check (length models) => 2)
          (check (andmap (lambda (m)
                           (and (eq? (vector-ref m 0) 'choice-model)
                                (member '(2 hot) (vector-ref m 1))
                                (pair? (vector-ref m 2))))
                         models)
                 => #t))))

    (test-case "Structured selector sub-language parses via :gerbil-parser"
      (let ((summary (parse-search-dsl-selector "lang:rust & rust::open_or_build_cached")))
        (check (vector-ref summary 0) => 'parse-artifact-summary)
        (check (vector-ref summary 1) => 'accepted)
        (check (vector-ref summary 2) => "search-selector")
        (check (vector-ref summary 4) => 0)))

    (test-case "Helm TAB completion table introspects real :gerbil-ascent POO relations (including rust_impl & rust_macro)"
      (let ((root-rows (search-dsl-completions "("))
            (rust-rows (search-dsl-completions "(ascent-fragment ((a n) <-- (rust_")))
        (check (> (vector-length root-rows) 4) => #t)
        (check (>= (vector-length rust-rows) 9) => #t)
        (check (vector-ref (vector-ref rust-rows 0) 0) => "rust_fn")
        (check (vector-ref (vector-ref rust-rows 0) 1) => "relation")))

    (test-case "Zero-regex Scheme, SysML, and Quint native AST extractor emits protocol-native Open EDB facts"
      (let* ((batch
               (extract-open-edb-facts-batch
                 (vector
                   (vector "scheme/search-dsl/objects.ss" "scheme"
                           "(import :clan/poo/object)\n(.def RustSearchFragment. languages: '(rust))\n(def Registry (.mix RustSearchFragment. Base.))\n(deflanguage selector (syntax))\n(c-define (eval-fn r) (int64) int64 \"marlin_eval_fn\" \"extern\")\n")
                   (vector "dev/sysml/search-engine.sysml" "sysml"
                           "package MarlinSearch {\n  requirement <'MAR-R01'> openEdb {}\n  allocation a1 allocate openEdb to target.kernel;\n}\n")
                   (vector "dev/quint/search.qnt" "quint"
                           "module Search {\n  action step = all { true }\n  val safe = true\n}\n"))))
             (ents (vector->list (vector-ref batch 0)))
             (edges (vector->list (vector-ref batch 1))))
        (check (and (ormap (lambda (e) (and (string=? (vector-ref e 2) "RustSearchFragment.")
                                            (string=? (vector-ref e 5) "poo_def")))
                           ents) #t) => #t)
        (check (and (ormap (lambda (e) (and (string=? (vector-ref e 2) "selector")
                                            (string=? (vector-ref e 5) "deflanguage")))
                           ents) #t) => #t)
        (check (and (ormap (lambda (e) (and (string=? (vector-ref e 2) "marlin_eval_fn")
                                            (string=? (vector-ref e 5) "c_define")))
                           ents) #t) => #t)
        (check (and (ormap (lambda (e) (and (string=? (vector-ref e 2) "openEdb")
                                            (string=? (vector-ref e 5) "requirement_def")))
                           ents) #t) => #t)
        (check (and (ormap (lambda (e) (and (string=? (vector-ref e 2) "safe")
                                            (string=? (vector-ref e 5) "quint_invariant")))
                           ents) #t) => #t)
        (check (and (ormap (lambda (g) (and (string=? (vector-ref g 1) "mixes")
                                            (string=? (vector-ref g 2) "RustSearchFragment.")))
                           edges) #t) => #t)
        (check (and (ormap (lambda (g) (and (string=? (vector-ref g 1) "allocates")
                                            (string=? (vector-ref g 0) "openEdb")))
                           edges) #t) => #t)))

    (test-case "General Language Ontology + POO extensions (including Julia) compose cleanly"
      (let* ((polyglot-edb
               '((file . ((1 "rust" "src/lib.rs")
                          (2 "python" "src/math_ops.py")
                          (3 "julia" "src/math.jl")
                          (4 "scheme" "src/sample.ss")))
                 (entity . (("r1" 1 "rust"   "function" "fn_item"     "add_rs"   5)
                            ("r2" 1 "rust"   "struct"   "struct_item" "Widget"   1)
                            ("p1" 2 "python" "function" "FunctionDef" "add_py"   4)
                            ("j1" 3 "julia"  "function" "function"    "add_jl"   6)
                            ("j2" 3 "julia"  "macro"    "macro"       "fastmath" 12)
                            ("s1" 4 "scheme" "function" "def"         "add_scm"  2)))
                 (edge . ())
                 (attr . ())))
             (callables (evaluate-search-dsl-over-edb
                          "(ascent-fragment
                             (relation answer ((lang string?) (name string?)))
                             ((answer lang name) <-- (callable _ _ lang name _)))"
                          polyglot-edb))
             (types (evaluate-search-dsl-over-edb
                      "(ascent-fragment
                         (relation answer ((lang string?) (name string?)))
                         ((answer lang name) <-- (type_decl _ _ lang name _)))"
                      polyglot-edb))
             (jl-macros (evaluate-search-dsl-over-edb
                          "(ascent-fragment
                             (relation answer ((name string?)))
                             ((answer name) <-- (julia_macro _ _ name _)))"
                          polyglot-edb)))
        (check (length (vector-ref callables 3)) => 4)
        (check (and (member '("rust" "add_rs") (vector-ref callables 3))
                    (member '("python" "add_py") (vector-ref callables 3))
                    (member '("julia" "add_jl") (vector-ref callables 3))
                    (member '("scheme" "add_scm") (vector-ref callables 3))
                    #t)
               => #t)
        (check (vector-ref types 3) => '(("rust" "Widget")))
        (check (vector-ref jl-macros 3) => '(("fastmath")))))

    (test-case "defasp-language projects all 5 provider conformance packs without lexical duplication"
      (let* ((packs (project-all-conformance-packs))
             (rust-pack (project-language-conformance-pack 'rust))
             (gerbil-pack (project-language-conformance-pack "gerbil-scheme"))
             (julia-pack (project-language-conformance-pack 'julia)))
        (check (length packs) => 5)
        (check (hash-ref rust-pack "schemaId") => "agent.semantic-protocols.language-conformance-pack")
        (check (hash-ref rust-pack "schemaVersion") => "1")
        (check (hash-ref rust-pack "providerLanguageId") => "rust")
        (check (hash-ref gerbil-pack "providerLanguageId") => "gerbil-scheme")
        (check (hash-ref julia-pack "providerLanguageId") => "julia")
        ;; Verify zero lexical pairs/comments hardcoded in Scheme conformance packs
        (check (hash-key? rust-pack "lexicalProfile") => #f)
        (check (hash-key? rust-pack "pairs") => #f)
        (check (hash-key? julia-pack "comments") => #f)
        (check (>= (length (hash-ref julia-pack "kindMappings")) 5) => #t)
        (check (>= (length (hash-ref gerbil-pack "probes")) 2) => #t)))

    (test-case "Unified 5-language General + Special search works identically in :gerbil-ascent and Dusa choice mode"
      (let* ((five-lang-edb
               '((file . ((1 "rust"       "src/lib.rs")
                          (2 "python"     "src/math_ops.py")
                          (3 "typescript" "src/math.ts")
                          (4 "julia"      "src/math.jl")
                          (5 "scheme"     "src/sample.ss")))
                 (entity . (("r1" 1 "rust"       "function" "fn_item"               "add_rs"     5)
                            ("r2" 1 "rust"       "item"     "impl_item"             "WidgetImpl" 10)
                            ("p1" 2 "python"     "function" "function_def"          "add_py"     4)
                            ("p2" 2 "python"     "struct"   "class_def"             "PyBox"      1)
                            ("t1" 3 "typescript" "function" "function_declaration"  "add_ts"     6)
                            ("t2" 3 "typescript" "struct"   "class_declaration"     "TsBox"      1)
                            ("t3" 3 "typescript" "trait"    "interface_declaration" "TsPoint"    3)
                            ("j1" 4 "julia"      "function" "function"              "add_jl"     6)
                            ("j2" 4 "julia"      "trait"    "abstract"              "AbstractOp" 1)
                            ("s1" 5 "scheme"     "function" "def"                   "add_scm"    2)
                            ("s2" 5 "scheme"     "item"     "poo_def"               "Sample."    8)))
                 (edge . ())
                 (attr . ())))
             (all-callables
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment
                    (relation answer ((lang string?) (name string?)))
                    ((answer lang name) <-- (callable _ _ lang name _)))"
                 five-lang-edb))
             (all-types
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment
                    (relation answer ((lang string?) (name string?)))
                    ((answer lang name) <-- (type_decl _ _ lang name _)))"
                 five-lang-edb))
             (py-cls
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment
                    (relation answer ((name string?)))
                    ((answer name) <-- (py_class _ _ name _)))"
                 five-lang-edb))
             (ts-cls
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment
                    (relation answer ((name string?)))
                    ((answer name) <-- (ts_class _ _ name _)))"
                 five-lang-edb))
             (ts-iface
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment
                    (relation answer ((name string?)))
                    ((answer name) <-- (ts_interface _ _ name _)))"
                 five-lang-edb))
             (jl-abs
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment
                    (relation answer ((name string?)))
                    ((answer name) <-- (julia_abstract _ _ name _)))"
                 five-lang-edb))
             (dusa-general-and-special
               (evaluate-search-dsl-over-edb
                 "(relational-choice-program
                    (choice priority (id p))
                    (rule (closed (priority ?id) ((value 'high) (value 'normal)))
                          (callable ?id ?_ ?_ ?_ ?_))
                    (rule (closed (priority ?id) ((value 'high)))
                          (ts_interface ?id ?_ (value 'TsPoint) ?_))
                    (limits 4096 4096 4096))"
                 five-lang-edb)))
        ;; General `callable` hits all 5 languages in one query!
        (check (length (vector-ref all-callables 3)) => 5)
        ;; General `type_decl` hits Python class_def (struct), TS class/interface (struct/trait), Julia abstract (trait)
        (check (length (vector-ref all-types 3)) => 4)
        ;; Special POO relations hit exact native `language_kind` even when `kind` is normalized!
        (check (vector-ref py-cls 3) => '(("PyBox")))
        (check (vector-ref ts-cls 3) => '(("TsBox")))
        (check (vector-ref ts-iface 3) => '(("TsPoint")))
        (check (vector-ref jl-abs 3) => '(("AbstractOp")))
        ;; Dusa choice program binds BOTH General Ontology (`callable`) and Special (`ts_interface`)!
        (check (vector-ref dusa-general-and-special 1) => 'choice)
        (check (pair? (vector-ref dusa-general-and-special 3)) => #t)))

    (test-case "defasp-language projects Yuanxu-compatible targeted JSON Schemas for all 5 languages"
      (let* ((rust-schema (project-language-json-schema 'rust))
             (props (cdr (assq 'properties rust-schema)))
             (lk-enum (cdr (assq 'enum (cdr (assq 'languageKind props)))))
             (lang-const (cdr (assq 'const (cdr (assq 'languageId props)))))
             (julia-schema (project-language-json-schema 'julia))
             (jl-lk (cdr (assq 'enum (cdr (assq 'languageKind (cdr (assq 'properties julia-schema))))))))
        (check (cdr (assq 'type rust-schema)) => "object")
        (check (cdr (assq 'additionalProperties rust-schema)) => #f)
        (check lang-const => "rust")
        (check (and (member "fn_item" lk-enum)
                    (member "impl_item" lk-enum)
                    (member "macro_rules" lk-enum)
                    #t)
               => #t)
        (check (and (member "abstract" jl-lk)
                    (member "macro" jl-lk)
                    #t)
               => #t)))

    (test-case "extend-search-language-registry adds dynamic language Special relations & 5-col General repair"
      (let* ((zig-desc
               #("zig"
                 "zig"
                 #(".zig")
                 #(#("comptime_fn" "function" "Zig compile-time generic function")
                   #("struct_decl" "struct" "Zig struct declaration"))
                 #(#("zig_comptime_fn" #f "comptime_fn" "Zig compile-time generic function")
                   #("zig_struct_decl" #f "struct_decl" "Zig struct declaration"))))
             (next-reg (extend-search-language-registry SearchLanguageRegistryValue zig-desc))
             (zig-edb '((file . ((1 "zig" "src/main.zig")))
                        (entity . (("z1" 1 "zig" "function" "comptime_fn" "buildMatrix" 1)))
                        (edge . ())
                        (attr . ())))
             (special-res
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment ((answer name) <-- (zig_comptime_fn _ _ name _)))"
                 zig-edb
                 next-reg))
             ;; Shorthand omitting `(relation answer ...)` verifies 5-column General Ontology repair!
             (general-shorthand-res
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment ((answer name) <-- (callable _ _ \"zig\" name _)))"
                 zig-edb
                 next-reg))
             (comps (search-dsl-completions "(ascent-fragment ((a n) <-- (zig_" next-reg))
             (zig-schema (project-language-json-schema 'zig next-reg))
             (zig-lk (cdr (assq 'enum (cdr (assq 'languageKind (cdr (assq 'properties zig-schema))))))))
        (check (vector-ref special-res 3) => '(("buildMatrix")))
        (check (vector-ref general-shorthand-res 3) => '(("buildMatrix")))
        (check (and (member "comptime_fn" zig-lk) (member "struct_decl" zig-lk) #t) => #t)
        (check (let loop ((i 0))
                 (cond
                   ((>= i (vector-length comps)) #f)
                   ((string=? (vector-ref (vector-ref comps i) 0) "zig_comptime_fn") #t)
                   (else (loop (+ i 1)))))
               => #t)))

    (test-case "Dependency-cone pruning shrinks 50+ rules to 2 rules & compiled session retracts cleanly"
      (let* ((qform '(ascent-fragment
                       (relation answer ((name string?)))
                       ((answer name) <-- (rust_impl _ _ name _))))
             (qpart (car (ascent-closed-dsl-parts (list qform))))
             (compiled (compile-closed-ascent-session qform SearchLanguageRegistryValue '()))
             (edb-a '((entity . (("e1" 1 "rust" "item" "impl_item" "AlphaImpl" 10)))))
             (edb-b '((entity . (("e2" 1 "rust" "item" "impl_item" "BetaImpl" 20))))))
        (let-values (((prog active-edb)
                      (prune-ascent-parts-for-query
                        (.get SearchLanguageRegistryValue ascent-fragments)
                        qpart)))
          (check (length (.ref prog 'rules)) => 2)
          (check active-edb => '(entity)))
        (check (vector-ref (run-compiled-ascent-session compiled edb-a) 3) => '(("AlphaImpl")))
        (check (vector-ref (run-compiled-ascent-session compiled edb-b) 3) => '(("BetaImpl")))))

    (test-case "Prefilter pushes down General Ontology language, symbol token, and path-prefix"
      (let ((pf-gen (compile-search-dsl-prefilter
                      "(ascent-fragment ((answer name) <-- (callable _ _ \"rust\" \"open_cached\" _)))"))
            (pf-path (compile-search-dsl-prefilter
                       "(ascent-fragment
                          (relation answer ((path string?)))
                          ((answer path) <-- (file _ \"rust\" path) (if (string-prefix? \"crates/search/\" path))))")))
        (check (vector-ref pf-gen 1) => #("rust"))
        (check (vector-ref pf-gen 3) => #("open_cached"))
        (check (vector-ref pf-path 1) => #("rust"))
        (check (vector-ref pf-path 2) => "crates/search/")))

    (test-case "Built-in language additive extension merges new Rust syntax kinds without losing built-ins"
      (let* ((rust-addon
               #("rust"
                 "rust"
                 #(".rs")
                 #(#("gen_fn" "function" "Rust generator function"))
                 #(#("rust_gen_fn" #f "gen_fn" "Rust generator function"))))
             (next-reg (extend-search-language-registry SearchLanguageRegistryValue rust-addon))
             (rust-schema (project-language-json-schema 'rust next-reg))
             (lk-enum (cdr (assq 'enum (cdr (assq 'languageKind (cdr (assq 'properties rust-schema)))))))
             (edb '((entity . (("r1" 1 "rust" "function" "gen_fn" "stream_rows" 5)
                               ("r2" 1 "rust" "item" "impl_item" "Registry" 10)))))
             (gen-res (evaluate-search-dsl-over-edb
                        "(ascent-fragment ((answer name) <-- (rust_gen_fn _ _ name _)))"
                        edb next-reg))
             (impl-res (evaluate-search-dsl-over-edb
                         "(ascent-fragment ((answer name) <-- (rust_impl _ _ name _)))"
                         edb next-reg)))
        (check (and (member "fn_item" lk-enum) (member "gen_fn" lk-enum) #t) => #t)
        (check (vector-ref gen-res 3) => '(("stream_rows")))
        (check (vector-ref impl-res 3) => '(("Registry")))))))

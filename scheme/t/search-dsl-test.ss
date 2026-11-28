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
                 extract-scheme-open-edb-facts
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
                 select-diverse-rows-up-to
                 evaluate-search-dsl-over-edb
                 evaluate-choice-program-over-edb
                 search-dsl-completions
                 json-schema-string->completion-entries
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
        (check (<= (string-length desc) 4096) => #t)
        (check (and (string-contains desc "(brute-search ...)")
                    (string-contains desc "(ascent-fragment ...)")
                    (string-contains desc "(search-choice-program ...)")
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

    (test-case "Zero-regex prefilter compiler derives Rust language + tokens from POO relation (rust_fn) without polluting tokens with file path"
      (let* ((pref (compile-search-dsl-prefilter
                     "(ascent-fragment
                        (relation answer ((path string?) (line integer?)))
                        ((answer path line) <-- (file f \"rust\" \"crates/foo/src/lib.rs\") (rust_fn id f \"open_or_build_cached\" line)))")))
        (check (vector-length pref) => 7)
        (check (vector-ref pref 0) => 'search-edb-prefilter)
        (check (vector-ref pref 1) => #("rust"))
        (check (vector-ref pref 2) => "crates/foo/src/lib.rs")
        (check (vector-ref pref 3) => #("open_or_build_cached"))
        (check (>= (vector-ref pref 4) 1024) => #t)
        (check (vector-ref pref 6) => #())
        (let ((multi-pref (compile-search-dsl-prefilter
                            "(find-impact (\"SymA\" \"SymB\") limit: 80)")))
          (check (vector-length multi-pref) => 7)
          (check (vector-ref multi-pref 3) => #())
          (check (vector-ref multi-pref 6) => #("SymA" "SymB")))))

    (test-case "Ascent supports (limit N), ((limit N)) unwrap repair, impl_trait, and rejects illegal clauses clearly"
      (let ((limited (evaluate-search-dsl-over-edb
                       "(ascent-fragment
                          ((limit 1))
                          ((answer name line) <-- (rust_fn _ _ name line)))"
                       sample-edb))
            (trait-res (evaluate-search-dsl-over-edb
                         "(ascent-fragment
                            ((answer ty tr path line) <-- (impl_trait ty tr path line)))"
                         sample-edb)))
        (check (length (vector-ref limited 3)) => 1)
        (check (memq 'unwrap-limit-clause (vector-ref limited 4)) ? pair?)
        (check (vector-ref trait-res 3) => '(("ResidentSearchToolHandler" "ToolRuntime" "crates/tooling/src/search_tool.rs" 120))))
      (check-exception
        (evaluate-search-dsl-over-edb
          "(ascent-fragment (bogus-clause 1 2) ((answer p) <-- (file _ _ p)))"
          sample-edb)
        true))

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
          ;; Omitting `(relation answer ...)` is first-class surface syntax (silent, no repair noise).
          (check (memq 'synthesize-head-relation tags) => #f)
          ;; Real repairs echo the canonical S-expression in slot 6.
          (check (string? (vector-ref r1 6)) => #t)))
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
          (check (memq 'synthesize-head-relation tags) => #f)))
      ;; 4. Missing `(limits ...)` and bare `'hot` / `'warm` values are silent first-class sugar;
      ;;    only omitted trailing wildcards trigger `pad-relation-wildcards`.
      (let ((r4 (evaluate-search-dsl-over-edb
                  "(relational-choice-program
                     (choice tier (f t))
                     (rule (closed (tier ?f) ('hot 'warm))
                           (file ?f 'rust))
                     (rule (closed (tier ?f) ('hot))
                           (rust_fn ?_ ?f 'open_or_build_cached)))"
                  sample-edb)))
        (check (vector-ref r4 1) => 'choice)
        (let ((report (vector-ref r4 3)))
          (check (vector-ref report 0) => 'choice-report)
          (check (vector-ref report 1) => 2))
        (let ((tags (vector-ref r4 4)))
          (check (memq 'normalize-choice-values tags) => #f)
          (check (and (memq 'pad-relation-wildcards tags) #t) => #t)
          (check (memq 'append-default-limits tags) => #f)
          (check (string? (vector-ref r4 5)) => #t)))
      ;; 5. Omitted trailing wildcards on 5-col `edge` (3-col shorthand `(edge a b "imports")`)
      (let ((r5 (evaluate-search-dsl-over-edb
                  "(ascent-fragment
                     ((answer a b) <-- (edge a b \"imports\")))"
                  sample-edb)))
        (check (vector-ref r5 3) => '(("e2" "e1")))
        (let ((tags (vector-ref r5 4)))
          (check (and (memq 'pad-relation-wildcards tags) #t) => #t)
          (check (memq 'synthesize-head-relation tags) => #f)))
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
        (check (vector-ref (vector-ref direct-res 3) 0) => 'choice-report)
        (check (vector-ref (vector-ref direct-res 3) 1) => 2)
        (check (vector-ref text-res 1) => 'choice)
        (let* ((report (vector-ref text-res 3))
               (worlds (vector-ref report 4)))
          (check (vector-ref report 0) => 'choice-report)
          (check (length worlds) => 2)
          (check (andmap (lambda (w)
                           (and (eq? (vector-ref w 0) 'choice-world)
                                (member '(2 hot) (vector-ref w 2))
                                (pair? (vector-ref w 3))))
                         worlds)
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
        (check (vector-ref (vector-ref dusa-general-and-special 3) 0) => 'choice-report)
        (check (> (vector-ref (vector-ref dusa-general-and-special 3) 1) 0) => #t)))

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
        (check (vector-ref impl-res 3) => '(("Registry")))))

    (test-case "General Ontology graph edges, entity_locator, and recursive depends_on closure"
      (let* ((graph-edb
               '((file . ((1 "rust" "src/a.rs") (2 "rust" "src/b.rs")))
                 (entity . (("src/a.rs#fn:run" 1 "rust" "function" "fn_item" "run" 4)
                            ("src/b.rs#fn:helper" 2 "rust" "function" "fn_item" "helper" 2)))
                 (edge . (("src/a.rs#impl:App" "src/a.rs#fn:run" "contains" "src/a.rs" 4)
                          ("src/a.rs" "src/b.rs" "imports" "src/a.rs" 1)
                          ("src/b.rs" "src/c.rs" "imports" "src/b.rs" 1)))
                 (attr . (("src/a.rs#fn:run" "locator" "src/a.rs#fn:run")))))
             (loc-res (evaluate-search-dsl-over-edb
                        "(ascent-fragment ((answer name loc) <-- (rust_fn id _ name _) (entity_locator id loc)))"
                        graph-edb SearchLanguageRegistryValue))
             (contains-res (evaluate-search-dsl-over-edb
                             "(ascent-fragment ((answer p c) <-- (contains_edge p c _ _)))"
                             graph-edb SearchLanguageRegistryValue))
             (dep-res (evaluate-search-dsl-over-edb
                        "(ascent-fragment ((answer dst) <-- (depends_on \"src/a.rs\" dst)))"
                        graph-edb SearchLanguageRegistryValue)))
        (check (vector-ref loc-res 3) => '(("run" "src/a.rs#fn:run")))
        (check (vector-ref contains-res 3) => '(("src/a.rs#impl:App" "src/a.rs#fn:run")))
        (check (and (member '("src/b.rs") (vector-ref dep-res 3))
                    (member '("src/c.rs") (vector-ref dep-res 3))
                    #t)
               => #t)))

    (test-case "Scenario B.1: Dusa search-choice-program disambiguates same-name callsites with deduction/branch witnesses and excludes test mocks"
      (let* ((edb
               '((file . ((1 "rust" "src/app.rs")
                          (2 "rust" "src/parser_a.rs")
                          (3 "rust" "src/parser_b.rs")
                          (4 "rust" "tests/mock_parser.rs")))
                 (entity . (("src/app.rs#fn:run" 1 "rust" "function" "fn_item" "run" 1)
                            ("src/app.rs#fn:local_init" 1 "rust" "function" "fn_item" "local_init" 8)
                            ("src/parser_a.rs#fn:parse" 2 "rust" "function" "fn_item" "parse" 3)
                            ("src/parser_b.rs#fn:parse" 3 "rust" "function" "fn_item" "parse" 5)
                            ("tests/mock_parser.rs#fn:parse" 4 "rust" "function" "fn_item" "parse" 2)))
                 (edge . (("src/app.rs#fn:run" "local_init" "calls" "src/app.rs" 2)
                          ("src/app.rs#fn:run" "parse" "calls" "src/app.rs" 4)))
                 (attr . ())))
             (query
               "(search-choice-program
                  (use rust)
                  (choice resolve (callsite target))
                  (rule (closed (resolve ?call) (?def))
                        (ref-at ?call ?name)
                        (same-module-def ?call ?name ?def))
                  (rule (open (resolve ?call) ?def)
                        (ref-at ?call ?name)
                        (workspace-def ?name ?def))
                  (rule (closed (resolve ?call) ())
                        (resolve ?call ?def)
                        (in-test-file ?def))
                  (solve resolve state-budget: 32 provenance?: #t))")
             (res (evaluate-search-dsl-over-edb query edb SearchLanguageRegistryValue))
             (report (vector-ref res 3))
             (world-count (vector-ref report 1))
             (worlds (vector-ref report 4)))
        ;; `local_init` is forced by same-module `closed` deduction;
        ;; `parse` has 3 defs across workspace, test mock is pruned by `closed ()`,
        ;; leaving EXACTLY 2 consistent worlds (parser_a vs parser_b)!
        (check (vector-ref res 1) => 'choice)
        (check (vector-ref res 2) => 'resolve)
        (check (vector-ref report 0) => 'choice-report)
        (check world-count => 2)
        (check (length worlds) => 2)))

    (test-case "Scenario B.2: Dusa search-choice-program verifies architecture tier boundaries with hot-path forbid constraint"
      (let* ((edb
               '((file . ((1 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/codec.rs")
                          (2 "scheme" "scheme/search-dsl/funcs.ss")
                          (3 "rust" "crates/marlin-agent-cli/src/main.rs")))
                 (entity . ())
                 (edge . ())
                 (attr . ())))
             (query
               "(search-choice-program
                  (use rust scheme)
                  (choice tier (mod level))
                  (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m))
                  (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m 'provider))
                  (solve tier state-budget: 64 provenance?: #t))")
             (res (evaluate-search-dsl-over-edb query edb SearchLanguageRegistryValue))
             (report (vector-ref res 3))
             (world-count (vector-ref report 1))
             (worlds (vector-ref report 4)))
        ;; 2 hot-path modules (`edb_bridge/codec.rs`, `search-dsl/funcs.ss`) are forced
        ;; to `resident`; 1 non-hot-path module (`cli/main.rs`) branches between
        ;; `resident` and `provider` -> EXACTLY 2 consistent architecture worlds!
        (check (vector-ref report 0) => 'choice-report)
        (check world-count => 2)
        (check (not (memq #f
                          (map (lambda (w)
                                 (let ((bindings (vector-ref w 2)))
                                   (and (member '(crates/marlin-agent-search-engine/src/edb_bridge/codec.rs resident) bindings)
                                        (member '(scheme/search-dsl/funcs.ss resident) bindings)
                                        #t)))
                               worlds)))
               => #t)))

    (test-case "Ascent ergonomics repair: ?var normalization, =/==/!= guards, literal let inlining, and bare lang quoting"
      (let* ((edb
               '((file . ((1 "rust" "src/a.rs")
                          (2 "rust" "src/b.rs")
                          (3 "scheme" "scheme/c.ss")))
                 (entity . (("a#s1" 1 "rust" "struct" "struct_item" "Config" 10)
                            ("a#e1" 1 "rust" "enum" "enum_item" "Mode" 20)
                            ("b#s1" 2 "rust" "struct" "struct_item" "Config" 5)
                            ("b#s2" 2 "rust" "struct" "struct_item" "Unique" 12)))
                 (edge . ())
                 (attr . ())))
             (r1 (evaluate-search-dsl-over-edb
                   "(ascent-fragment (limit 20)
                      ((answer kind name line) <--
                       (rust_struct ?id ?fid ?name ?line)
                       (file ?fid rust ?path)
                       (== ?path \"src/a.rs\")
                       (let kind \"struct\"))
                      ((answer kind name line) <--
                       (rust_enum ?id ?fid ?name ?line)
                       (file ?fid rust ?path)
                       (= ?path \"src/a.rs\")
                       (let kind \"enum\")))"
                   edb SearchLanguageRegistryValue))
             (r2 (evaluate-search-dsl-over-edb
                   "(ascent-fragment (limit 20)
                      ((answer name p1 p2) <--
                       (rust_struct _ f1 name _)
                       (rust_struct _ f2 name _)
                       (file f1 rust p1)
                       (file f2 rust p2)
                       (!= f1 f2)
                       (< f1 f2)))"
                   edb SearchLanguageRegistryValue)))
        (check (length (vector-ref r1 3)) => 2)
        (check (and (member '("struct" "Config" 10) (vector-ref r1 3))
                    (member '("enum" "Mode" 20) (vector-ref r1 3))
                    #t)
               => #t)
        (check (vector-ref r2 3) => '(("Config" "src/a.rs" "src/b.rs")))))

    (test-case "Ascent repair absorbs trailing = guard, inlines equality literals, and gives schema-aware arity & head-safety errors"
      (let* ((edb
               '((file . ((1 "rust" "src/gw.rs")))
                 (entity . (("gw#i1" 1 "rust" "item" "impl_item" "WeaverModelGateway" 42)
                            ("gw#i2" 1 "rust" "item" "impl_item" "OtherImpl" 80)))
                 (edge . (("gw#i1" "ModelGateway" "implements" "src/gw.rs" 42)
                          ("gw#i2" "OtherTrait" "implements" "src/gw.rs" 80)))
                 (attr . (("gw#i1" "trait" "ModelGateway")
                          ("gw#i2" "trait" "OtherTrait")))))
             ;; Misplaced `(= trait "ModelGateway")` outside the rule is absorbed AND inlined into head+body!
             (r-absorbed
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment (limit 20)
                    ((answer type trait path line) <-- (impl_trait type trait path line))
                    (= trait \"ModelGateway\"))"
                 edb SearchLanguageRegistryValue))
             ;; Arity overflow on `type_decl` (6 args instead of 5) reports expected arity & schema.
             (arity-err
               (with-catch
                 (lambda (e) (call-with-output-string (lambda (p) (display-exception e p))))
                 (lambda ()
                   (repair-ascent-search-form
                     "(ascent-fragment ((answer name) <-- (type_decl id fid \"rust\" kind name line)))")
                   "unexpected-ok")))
             ;; Unbound head variable `kind` reports actionable head-safety diagnostic.
             (unsafe-err
               (with-catch
                 (lambda (e) (call-with-output-string (lambda (p) (display-exception e p))))
                 (lambda ()
                   (repair-ascent-search-form
                     "(ascent-fragment ((answer kind name) <-- (rust_struct id fid name line)))")
                   "unexpected-ok"))))
        (check (vector-ref r-absorbed 3) => '(("WeaverModelGateway" "ModelGateway" "src/gw.rs" 42)))
        (check (and (string-contains arity-err "DSL_REPAIR_DIAGNOSTIC")
                    (string-contains arity-err "arity_mismatch")
                    (string-contains arity-err "type_decl(id, file_id, lang, name, line)") #t) => #t)
        (check (and (string-contains unsafe-err "DSL_REPAIR_DIAGNOSTIC")
                    (string-contains unsafe-err "unsafe_head_variable")
                    (string-contains unsafe-err "unsafe ASCENT head variable 'kind'") #t) => #t)))

    (test-case "Principled Ascent repair hoists rule-body scope/limit/string-prefix and normalizes string=? / unsupported guards"
      (let* ((edb
               '((file . ((1 "rust" "crates/a/src/lib.rs")
                          (2 "rust" "crates/b/src/lib.rs")))
                 (entity . (("a#s1" 1 "rust" "struct" "struct_item" "Shared" 10)
                            ("b#s1" 2 "rust" "struct" "struct_item" "Shared" 20)))
                 (edge . ())
                 (attr . ())))
             (repaired-hoist
               (repair-ascent-search-form
                 "(ascent-fragment (limit 20)
                    ((answer name p) <--
                     (rust_struct _ f name _)
                     (file f rust p)
                     (scope \"crates/a/\")))"))
             (prefilter-hoist
               (compile-search-dsl-prefilter
                 "(ascent-fragment
                    ((answer name p) <--
                     (rust_struct _ f name _)
                     (file f rust p)
                     (string-prefix? \"crates/a/\" p)))"))
             (repaired-eq
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment (limit 20)
                    ((answer name p) <--
                     (rust_struct _ f name _)
                     (file f rust p)
                     (if (name) (string=? name \"Shared\"))))"
                 edb SearchLanguageRegistryValue))
             (bad-guard
               (with-catch
                 (lambda (e) (call-with-output-string (lambda (p) (display-exception e p))))
                 (lambda ()
                   (repair-ascent-search-form
                     "(ascent-fragment ((answer p) <-- (file _ \"rust\" p) (if (p) (pregexp-match \"foo\" p))))")
                   "unexpected-ok"))))
        (check (and (memq 'hoist-rule-directive (vector-ref repaired-hoist 1))
                    (memq 'normalize-scope-directive (vector-ref repaired-hoist 1))
                    #t)
               => #t)
        (check (vector-ref prefilter-hoist 2) => "crates/a/")
        (check (length (vector-ref repaired-eq 3)) => 2)
        (check (and (string-contains bad-guard "DSL_REPAIR_DIAGNOSTIC")
                    (string-contains bad-guard "unsupported_guard_operator") #t)
               => #t)))

    (test-case "Dusa choice repair infers/synthesizes choice arity and gives structured diagnostics for malformed heads and != guards"
      (let ((repaired-arity
              (repair-ascent-search-form
                "(search-choice-program (choice dup (p)) (rule (open (dup ?n) ?p) (rust_struct ?i ?f ?n ?l) (file ?f \"rust\" ?p)))"))
            (repaired-synth
              (repair-ascent-search-form
                "(search-choice-program (use rust) (scope \"crates/a/\") (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m)) (solve tier state-budget: 16))"))
            (bad-open
              (with-catch
                (lambda (e) (call-with-output-string (lambda (p) (display-exception e p))))
                (lambda ()
                  (repair-ascent-search-form
                    "(search-choice-program (choice dup (k v)) (rule (open (dup ?n ?p1 ?p2)) (rust_struct ?i ?f ?n ?l)))")
                  "unexpected-ok")))
            (bad-tuple
              (with-catch
                (lambda (e) (call-with-output-string (lambda (p) (display-exception e p))))
                (lambda ()
                  (repair-ascent-search-form
                    "(search-choice-program (choice dup (k v)) (rule (open (dup ?n) (?p ?q)) (rust_struct ?i ?f ?n ?l)))")
                  "unexpected-ok")))
            (bad-guard
              (with-catch
                (lambda (e) (call-with-output-string (lambda (p) (display-exception e p))))
                (lambda ()
                  (repair-ascent-search-form
                    "(search-choice-program (choice dup (k v)) (rule (open (dup ?n) ?p) (rust_struct ?i ?f ?n ?l) (!= ?f ?g)))")
                  "unexpected-ok"))))
        (check (and (memq 'infer-choice-arity (vector-ref repaired-arity 1)) #t) => #t)
        (check (and (memq 'synthesize-choice-decl (vector-ref repaired-synth 1))
                    (memq 'normalize-scope-directive (vector-ref repaired-synth 1)) #t) => #t)
        (check (and (string-contains bad-open "invalid_dusa_rule_head") #t) => #t)
        (check (and (string-contains bad-tuple "invalid_dusa_open_value") #t) => #t)
        (check (and (string-contains bad-guard "unsupported_dusa_guard") #t) => #t)))

    (test-case "First-class (not (= a b)), (context N), calls_symbol view, and Dusa budget-exhausted combinatorics + partial witnesses"
      (let* ((call-edb
               '((file . ((1 "rust" "crates/a/src/lib.rs")
                          (2 "rust" "crates/b/src/lib.rs")
                          (3 "rust" "crates/c/src/lib.rs")
                          (4 "rust" "crates/d/src/lib.rs")
                          (5 "rust" "crates/e/src/lib.rs")
                          (6 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/codec.rs")))
                 (entity . (("a#fn:caller" 1 "rust" "function" "fn_item" "run_pipeline" 10)
                            ("b#fn:callee" 2 "rust" "function" "fn_item" "verify_claim" 20)
                            ("a#s1" 1 "rust" "struct" "struct_item" "Config" 3)
                            ("b#s1" 2 "rust" "struct" "struct_item" "Config" 5)))
                 (edge . (("a#fn:caller" "b#fn:callee" "calls" "crates/a/src/lib.rs" 12)
                          ("a#fn:caller" "external_helper" "calls" "crates/a/src/lib.rs" 14)
                          ("direct_caller" "direct_callee" "calls" "crates/b/src/lib.rs" 25)))
                 (attr . ())))
             ;; 1. `(not (= p1 p2))`, `(= name "Config")`, omitted `(relation ...)`, and `(context 2)` are silent first-class sugar!
             (neg-eq-res
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment (limit 10) (context 2)
                    ((answer name p1 p2) <--
                     (rust_struct _ f1 name _)
                     (rust_struct _ f2 name _)
                     (file f1 \"rust\" p1)
                     (file f2 \"rust\" p2)
                     (= name \"Config\")
                     (not (= p1 p2))))"
                 call-edb SearchLanguageRegistryValue))
             ;; 2. `calls_symbol` resolves entity_id->entity_id, entity_id->sym, and sym->sym cleanly!
             (calls-res
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment
                    ((answer caller callee path line) <-- (calls_symbol caller callee path line)))"
                 call-edb SearchLanguageRegistryValue))
             ;; 3. Dusa budget exhaustion reports combinatorics + partial SAT witness sample!
             (dusa-exhausted
               (evaluate-search-dsl-over-edb
                 "(search-choice-program
                    (use rust)
                    (choice tier (mod lvl))
                    (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m))
                    (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m provider))
                    (solve tier state-budget: 8 provenance?: #t))"
                 call-edb SearchLanguageRegistryValue))
             (exhaust-payload (vector-ref dusa-exhausted 3)))
        (check (length (vector-ref neg-eq-res 3)) => 2)
        (check (vector-ref neg-eq-res 4) => '())
        (check (vector-ref neg-eq-res 6) => #f)
        (check (vector-ref neg-eq-res 7) => 2)
        (check (and (member '("run_pipeline" "verify_claim" "crates/a/src/lib.rs" 12) (vector-ref calls-res 3))
                    (member '("run_pipeline" "external_helper" "crates/a/src/lib.rs" 14) (vector-ref calls-res 3))
                    (member '("direct_caller" "direct_callee" "crates/b/src/lib.rs" 25) (vector-ref calls-res 3))
                    #t)
               => #t)
        (check (vector-ref exhaust-payload 0) => 'choice-budget-exhausted)
        (check (vector-ref exhaust-payload 3) => 6)  ;; scoped_count = 6
        (check (vector-ref exhaust-payload 4) => 1)  ;; constrained (hot-path) = 1
        (check (vector-ref exhaust-payload 5) => 5)  ;; free_vars = 5
        (check (vector-ref exhaust-payload 6) => 2)  ;; branch_factor = 2
        (check (vector-ref exhaust-payload 7) => 32) ;; estimated_worlds = 2^5 = 32
        (check (>= (vector-ref exhaust-payload 8) 1) => #t)
        (check (> (length (vector-ref exhaust-payload 10)) 0) => #t)))

    (test-case "Ascent (or ...) DNF expansion, sound multi-rule prefilter, negation wildcarding, and Dusa 5-hot-path closed-forbid deduction"
      (let* ((edb
               '((file . ((1 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/mod.rs")
                          (2 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/types.rs")
                          (3 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/codec.rs")
                          (4 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/choice_codec.rs")
                          (5 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/runner.rs")
                          (6 "scheme" "scheme/policy/governance.ss")))
                 (entity . (("e1" 1 "rust" "struct" "struct_item" "Alpha" 10)
                            ("e2" 2 "rust" "struct" "struct_item" "Beta" 20)
                            ("e3" 6 "scheme" "function" "def_proc" "evaluate-policy" 5)))
                 (edge . (("e1" "foo_fn" "calls" "crates/marlin-agent-search-engine/src/edb_bridge/mod.rs" 12)
                          ("e2" "bar_fn" "calls" "crates/marlin-agent-search-engine/src/edb_bridge/types.rs" 22)))
                 (attr . (("e1" "trait" "ModelGateway")))))
             ;; 1. `(or (= callee "foo_fn") (= callee "bar_fn"))` expands cleanly to 2 rules (silent)
             ;;    AND prefilter intersects tokens across rules -> empty token list (never drops files!).
             (or-query
               "(ascent-fragment
                  ((answer caller callee path line)
                   <-- (calls_symbol caller callee path line)
                       (or (= callee \"foo_fn\") (= callee \"bar_fn\"))))")
             (or-pref (compile-search-dsl-prefilter or-query))
             (or-res (evaluate-search-dsl-over-edb or-query edb SearchLanguageRegistryValue))
             ;; 2. Cross-language multi-rule prefilter unions languages and intersects tokens!
             (poly-pref
               (compile-search-dsl-prefilter
                 "(ascent-fragment
                    ((answer lang name) <-- (rust_struct _ _ \"Alpha\" _) (= lang \"rust\") (= name \"Alpha\"))
                    ((answer lang name) <-- (scheme_def _ _ \"evaluate-policy\" _) (= lang \"scheme\") (= name \"evaluate-policy\")))"))
             ;; 3. Existential unbound negation variables `(not (impl_trait name tr p2 l2))` auto-wildcard to `_`!
             (neg-res
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment
                    ((answer name)
                     <-- (rust_struct _ _ name _)
                         (not (impl_trait name tr p2 l2))))"
                 edb SearchLanguageRegistryValue))
             ;; 4. Dusa 5 hot-path files in `edb_bridge/` previously blew up state-budget: 64;
             ;;    closed-forbid deduction propagation now solves all 5 in 1 SAT world!
             (dusa-5-hot
               (evaluate-search-dsl-over-edb
                 "(search-choice-program
                    (use rust)
                    (scope path-prefix: \"crates/marlin-agent-search-engine/src/edb_bridge/\" limit: 6)
                    (choice tier (m lvl))
                    (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m))
                    (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m provider))
                    (solve tier state-budget: 64 provenance?: #t))"
                 edb SearchLanguageRegistryValue))
             (dusa-report (vector-ref dusa-5-hot 3)))
        (check (vector-ref or-pref 3) => #())
        (check (length (vector-ref or-res 3)) => 2)
        (check (vector-ref or-res 4) => '())
        (check (vector-ref poly-pref 1) => #("rust" "scheme"))
        (check (vector-ref poly-pref 3) => #())
        (check (vector-ref neg-res 3) => '(("Beta")))
        (check (vector-ref dusa-report 0) => 'choice-report)
        (check (vector-ref dusa-report 1) => 1)))

    (test-case "High-level declarative L1/L2 forms lower directly into real Ascent & Dusa programs"
      (let* ((edb
               '((file . ((1 "rust" "crates/a/src/mod.rs")
                          (2 "rust" "crates/b/src/mod.rs")
                          (3 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/codec.rs")
                          (4 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/runner.rs")))
                 (entity . (("a#SharedCfg" 1 "rust" "struct" "struct_item" "SharedCfg" 10)
                            ("b#SharedCfg" 2 "rust" "struct" "struct_item" "SharedCfg" 25)
                            ("a#run"       1 "rust" "function" "function_item" "run" 40)
                            ("b#run"       2 "rust" "function" "function_item" "run" 55)
                            ("a#caller"    1 "rust" "function" "function_item" "caller_fn" 70)))
                 (edge . (("a#caller" "run" "calls" "crates/a/src/mod.rs" 72)))
                 (attr . ())))
             (struct-colls
               (evaluate-search-dsl-over-edb
                 "(find-collisions kind: \"struct\" path-prefix: \"crates/\" lang: \"rust\" limit: 20)"
                 edb SearchLanguageRegistryValue))
             (fn-colls-bare
               (evaluate-search-dsl-over-edb
                 "find-collisions kind: \"fn\" path-prefix: \"crates/\" lang: \"rust\" limit: 20"
                 edb SearchLanguageRegistryValue))
             (callers
               (evaluate-search-dsl-over-edb
                 "(find-callers \"run\" path-prefix: \"crates/\" limit: 20)"
                 edb SearchLanguageRegistryValue))
             (placement
               (evaluate-search-dsl-over-edb
                 "(solve-placement path-prefix: \"crates/marlin-agent-search-engine/src/edb_bridge/\" limit: 6 choices: (resident provider) forbid-hot-path: provider state-budget: 64)"
                 edb SearchLanguageRegistryValue))
             (placement-report (vector-ref placement 3)))
        (check (length (vector-ref struct-colls 3)) => 1)
        (check (car (car (vector-ref struct-colls 3))) => "SharedCfg")
        (check (length (vector-ref fn-colls-bare 3)) => 1)
        (check (car (car (vector-ref fn-colls-bare 3))) => "run")
        (check (length (vector-ref callers 3)) => 1)
        (check (car (car (vector-ref callers 3))) => "caller_fn")
        (check (vector-ref placement-report 0) => 'choice-report)
        ;; Both hot-path files are forced to `resident` -> exact 1 SAT world!
        (check (vector-ref placement-report 1) => 1))
      ;; Balanced bounded slice: mixed asp_http_client/* + edb_bridge/* under limit: 6
      ;; selects 4 constrained hot-path modules + 2 free modules -> exact 4 SAT worlds within budget 64!
      (let* ((mixed-edb
               '((file . ((1 "rust" "crates/marlin-agent-search-engine/src/asp_http_client/domain_types.rs")
                          (2 "rust" "crates/marlin-agent-search-engine/src/asp_http_client/mod.rs")
                          (3 "rust" "crates/marlin-agent-search-engine/src/asp_http_client/patch_client.rs")
                          (4 "rust" "crates/marlin-agent-search-engine/src/asp_http_client/protocol_types.rs")
                          (5 "rust" "crates/marlin-agent-search-engine/src/asp_http_client/sync.rs")
                          (6 "rust" "crates/marlin-agent-search-engine/src/engine.rs")
                          (7 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/choice_codec.rs")
                          (8 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/codec.rs")
                          (9 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/mod.rs")
                          (10 "rust" "crates/marlin-agent-search-engine/src/edb_bridge/runner.rs")))
                 (entity . ()) (edge . ()) (attr . ())))
             (mixed-res
               (evaluate-search-dsl-over-edb
                 "(solve-placement path-prefix: \"crates/marlin-agent-search-engine/src/\" module-limit: 6 forbid-hot-path: provider state-budget: 64)"
                 mixed-edb SearchLanguageRegistryValue))
             (mixed-report (vector-ref mixed-res 3)))
        (check (vector-ref mixed-report 0) => 'choice-report)
        (check (vector-ref mixed-report 1) => 4)
        (check (vector-ref mixed-report 5) => 6)
        (check (vector-ref mixed-report 6) => 10)))

    (test-case "Closed-Ascent path predicates, exclude-tests?, find-impact, and anti-mode-collapse diversity sampling"
      (let* ((edb
               '((file . ((1 "rust" "crates/a/src/core.rs")
                          (2 "rust" "crates/b/src/core.rs")
                          (3 "rust" "crates/a/tests/mock_core.rs")
                          (4 "rust" "crates/c/src/adapter.rs")
                          (5 "org"  "docs/shared-notes.org")))
                 (entity . (("a#SharedCfg" 1 "rust" "struct" "struct_item" "SharedCfg" 10)
                            ("b#SharedCfg" 2 "rust" "struct" "struct_item" "SharedCfg" 25)
                            ("t#SharedCfg" 3 "rust" "struct" "struct_item" "SharedCfg" 5)
                            ("c#Adapter"   4 "rust" "struct" "struct_item" "Adapter" 12)
                            ("d#SharedDoc" 5 "org"  "heading" "org_heading" "SharedCfg Design Notes" 1)
                            ("a#fn_new"    1 "rust" "function" "fn_item" "new" 40)
                            ("b#fn_new"    2 "rust" "function" "fn_item" "new" 41)
                            ("c#fn_new"    4 "rust" "function" "fn_item" "new" 42)
                            ("a#fn_parse"  1 "rust" "function" "fn_item" "parse" 50)
                            ("b#fn_parse"  2 "rust" "function" "fn_item" "parse" 51)
                            ("a#fn_run"    1 "rust" "function" "fn_item" "run" 60)
                            ("b#fn_run"    2 "rust" "function" "fn_item" "run" 61)
                            ("t#test_call" 3 "rust" "function" "fn_item" "test_shared" 30)))
                 (edge . (("c#Adapter" "SharedCfg" "calls" "crates/c/src/adapter.rs" 20)
                          ("t#test_call" "SharedCfg" "calls" "crates/a/tests/mock_core.rs" 32)
                          ("crates/c/src/adapter.rs" "SharedCfg" "imports" "crates/c/src/adapter.rs" 2)))
                 (attr . (("c#Adapter" "implements" "SharedCfg")))
                 (symbol_ref . (("SharedCfg" "crates/a/src/core.rs" 10)
                                ("SharedCfg" "crates/c/src/adapter.rs" 44)
                                ("SharedCfg" "crates/a/tests/mock_core.rs" 19)))))
             (prod-colls
               (evaluate-search-dsl-over-edb
                 "(find-collisions kind: \"struct\" path-prefix: \"crates/\" lang: \"rust\" exclude-tests?: #t limit: 20)"
                 edb SearchLanguageRegistryValue))
             (neg-str-pred
               (evaluate-search-dsl-over-edb
                 "(ascent-fragment
                    ((answer name p)
                     <-- (rust_struct _ f name _)
                         (file f \"rust\" p)
                         (not (str-prefix p \"crates/a/tests/\"))
                         (not (str-contains p \"adapter\"))))"
                 edb SearchLanguageRegistryValue))
             (neg-pref
               (compile-search-dsl-prefilter
                 "(ascent-fragment
                    ((answer p) <-- (file _ \"rust\" p) (not (path_has_prefix p \"crates/a/tests/\"))))"))
             (impact-pref
               (compile-search-dsl-prefilter
                 "(find-impact \"SharedCfg\" path-prefix: \"crates/\" limit: 40)"))
             (impact-res
               (evaluate-search-dsl-over-edb
                 "(find-impact \"SharedCfg\" path-prefix: \"crates/\" limit: 40)"
                 edb SearchLanguageRegistryValue))
             (impact-rows (vector-ref impact-res 3))
             (impact-prod-only
               (evaluate-search-dsl-over-edb
                 "(find-impact \"SharedCfg\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40)"
                 edb SearchLanguageRegistryValue))
             (name-contains-res
               (evaluate-search-dsl-over-edb
                 "(find-symbols name-contains: (\"shared\" \"adap\") lang: \"any\" exclude-tests?: #t limit: 20)"
                 edb SearchLanguageRegistryValue))
             (name-contains-rows (vector-ref name-contains-res 3))
             (name-prefix-res
               (evaluate-search-dsl-over-edb
                 "(find-symbols name-prefix: \"ada\" path-prefix: \"crates/\" exclude-tests?: #t limit: 20)"
                 edb SearchLanguageRegistryValue))
             (name-prefix-rows (vector-ref name-prefix-res 3))
             (org-doc-res
               (evaluate-search-dsl-over-edb
                 "(find-symbols name-contains: \"shared\" lang: \"org\" limit: 20)"
                 edb SearchLanguageRegistryValue))
             (org-doc-rows (vector-ref org-doc-res 3))
             (diverse-fn-colls
               (evaluate-search-dsl-over-edb
                 "(find-collisions kind: \"fn\" path-prefix: \"crates/\" lang: \"rust\" exclude-tests?: #t limit: 3)"
                 edb SearchLanguageRegistryValue))
             (min3-fn-colls
               (evaluate-search-dsl-over-edb
                 "(find-collisions kind: \"fn\" path-prefix: \"crates/\" lang: \"rust\" min-files: 3 exclude-tests?: #t limit: 20)"
                 edb SearchLanguageRegistryValue))
             (min3-syms (map car (vector-ref min3-fn-colls 3)))
             (multi-kind-colls
               (evaluate-search-dsl-over-edb
                 "(find-collisions kind: (\"struct\" \"fn\") path-prefix: \"crates/\" lang: \"rust\" exclude-tests?: #t limit: 12)"
                 edb SearchLanguageRegistryValue))
             (multi-kind-rows (vector-ref multi-kind-colls 3))
             (multi-kind-tags (map car multi-kind-rows))
             (copilot-rust-kws (vector->list (search-dsl-completions "(find-collisions lang: \"rust\" ")))
             (copilot-rust-kw-names (map (lambda (v) (vector-ref v 0)) copilot-rust-kws))
             (copilot-rust-kinds (vector->list (search-dsl-completions "(find-collisions lang: \"rust\" kind: ")))
             (copilot-rust-kind-names (map (lambda (v) (vector-ref v 0)) copilot-rust-kinds))
             (copilot-rust-lks (vector->list (search-dsl-completions "(find-symbols lang: \"rust\" languageKind: ")))
             (copilot-rust-lk-names (map (lambda (v) (vector-ref v 0)) copilot-rust-lks))
             (copilot-scheme-lks (vector->list (search-dsl-completions "(find-symbols lang: \"scheme\" languageKind: ")))
             (copilot-scheme-lk-names (map (lambda (v) (vector-ref v 0)) copilot-scheme-lks))
             (asp-proto-json
               "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"type\":\"object\",\"$defs\":{\"UniversalSearchProjection\":{\"type\":\"string\",\"enum\":[\"matches\",\"source\",\"callable-skeleton\"]}},\"properties\":{\"mode\":{\"type\":\"string\",\"enum\":[\"selector\",\"syntax\",\"playbook\"]},\"projection\":{\"anyOf\":[{\"$ref\":\"#/$defs/UniversalSearchProjection\"},{\"type\":\"null\"}]},\"exclude-tests?\":{\"type\":[\"boolean\",\"null\"]}}}")
             (yuanxu-proto-entries (json-schema-string->completion-entries "*" asp-proto-json))
             (yuanxu-proto-forms (map car yuanxu-proto-entries))
             (diverse-syms (map car (vector-ref diverse-fn-colls 3)))
             (direct-diverse
               (select-diverse-rows-up-to
                 '(("new" "a" 1) ("new" "b" 2) ("new" "c" 3) ("new" "d" 4)
                   ("parse" "a" 5) ("parse" "b" 6)
                   ("run" "a" 7))
                 3))
             (prod-first-sym-rows
               (select-diverse-rows-up-to
                 '(("MockFn" "crates/a/tests/mock.rs" 1)
                   ("TestHelper" "crates/a/tests/helper_test.rs" 2)
                   ("RealHandler" "crates/a/src/handler.rs" 10))
                 2)))
        (check (length (vector-ref prod-colls 3)) => 1)
        (check (vector-ref prod-colls 4) => '())
        (check (length (vector-ref neg-str-pred 3)) => 2)
        (check (vector-ref neg-str-pred 4) => '())
        (check (vector-ref neg-pref 2) => #f)
        (check (vector-ref impact-pref 2) => "crates/")
        (check (vector-ref impact-pref 3) => #("SharedCfg"))
        (check (and (member '("def" "prod" "SharedCfg" "crates/a/src/core.rs" 10) impact-rows)
                    (member '("def" "test" "SharedCfg" "crates/a/tests/mock_core.rs" 5) impact-rows)
                    (member '("caller" "prod" "Adapter" "crates/c/src/adapter.rs" 20) impact-rows)
                    (member '("caller" "test" "test_shared" "crates/a/tests/mock_core.rs" 32) impact-rows)
                    (member '("impl" "prod" "Adapter" "crates/c/src/adapter.rs" 12) impact-rows)
                    (member '("import" "prod" "crates/c/src/adapter.rs" "crates/c/src/adapter.rs" 2) impact-rows)
                    (member '("ref" "prod" "SharedCfg" "crates/c/src/adapter.rs" 44) impact-rows)
                    (member '("ref" "test" "SharedCfg" "crates/a/tests/mock_core.rs" 19) impact-rows)
                    #t)
               => #t)
        ;; Def-site deduplication: symbol_ref at ("SharedCfg" "crates/a/src/core.rs" 10) must NOT duplicate def!
        (check (member '("ref" "prod" "SharedCfg" "crates/a/src/core.rs" 10) impact-rows) => #f)
        (check (ormap (lambda (r) (string=? (cadr r) "test")) (vector-ref impact-prod-only 3)) => #f)
        ;; Polyglot case-insensitive name_contains + name_has_prefix with exclude-tests?: #t
        ;; and automatic doc-heading exclusion under lang: "any" (while lang: "org" still matches docs).
        (check (vector-ref name-contains-res 4) => '())
        (check (and (member '("SharedCfg" "crates/a/src/core.rs" 10) name-contains-rows)
                    (member '("Adapter" "crates/c/src/adapter.rs" 12) name-contains-rows)
                    #t)
               => #t)
        (check (ormap (lambda (r) (string=? (cadr r) "crates/a/tests/mock_core.rs")) name-contains-rows) => #f)
        (check (ormap (lambda (r) (string=? (cadr r) "docs/shared-notes.org")) name-contains-rows) => #f)
        (check (vector-ref org-doc-res 4) => '())
        (check org-doc-rows => '(("SharedCfg Design Notes" "docs/shared-notes.org" 1)))
        (check (vector-ref name-prefix-res 4) => '())
        (check name-prefix-rows => '(("Adapter" "crates/c/src/adapter.rs" 12)))
        (check (length diverse-syms) => 3)
        (check (and (member "new" diverse-syms)
                    (member "parse" diverse-syms)
                    (member "run" diverse-syms)
                    #t)
               => #t)
        (check (map car direct-diverse) => '("new" "parse" "run"))
        (check (car (car prod-first-sym-rows)) => "RealHandler")
        ;; `min-files: 3` isolates high-multiplicity hubs (`new` in 3 prod files) over 2-file duplicates!
        (check (and (pair? min3-syms) (member "new" min3-syms) (not (member "parse" min3-syms)) (not (member "run" min3-syms)) #t) => #t)
        ;; Multi-kind `(find-collisions kind: ("struct" "fn") ...)` emits 6-column typed rows with fair kind representation!
        (check (vector-ref multi-kind-colls 4) => '())
        (check (and (member "struct" multi-kind-tags)
                    (member "fn" multi-kind-tags)
                    (= (length (car multi-kind-rows)) 6)
                    #t)
               => #t)
        ;; Yuanxu JSON-Schema-driven Copilot completions (zero hardcoded tables!):
        ;;   - Rust language JSON Schema projects `kind:` + `languageKind:` and exact Rust enums
        ;;   - Scheme language JSON Schema projects exact Scheme `languageKind:` enums (`def`, `poo_def`)
        ;;   - Arbitrary ASP/gRPC JSON Schema string projects keywords (`mode:`, `projection:`, `excludeTests:`) + enum/boolean values!
        (check (and (member "kind:" copilot-rust-kw-names)
                    (member "languageKind:" copilot-rust-kw-names)
                    (not (member "lang:" copilot-rust-kw-names))
                    #t)
               => #t)
        (check (and (member "\"struct\"" copilot-rust-kind-names)
                    (member "\"function\"" copilot-rust-kind-names)
                    (member "\"trait\"" copilot-rust-kind-names)
                    #t)
               => #t)
        (check (and (member "\"struct_item\"" copilot-rust-lk-names)
                    (member "\"impl_item\"" copilot-rust-lk-names)
                    (not (member "\"poo_def\"" copilot-rust-lk-names))
                    #t)
               => #t)
        (check (and (member "\"def\"" copilot-scheme-lk-names)
                    (member "\"poo_def\"" copilot-scheme-lk-names)
                    (not (member "\"impl_item\"" copilot-scheme-lk-names))
                    #t)
               => #t)
        (check (and (member "mode:" yuanxu-proto-forms)
                    (member "\"playbook\"" yuanxu-proto-forms)
                    (member "projection:" yuanxu-proto-forms)
                    (member "\"callable-skeleton\"" yuanxu-proto-forms)
                    (member "exclude-tests?:" yuanxu-proto-forms)
                    (member "#t" yuanxu-proto-forms)
                    #t)
               => #t)))

    (test-case "LLM-ASP P0: name-all AND pushdown, special facet filtering, Scheme special attrs, FFI impact, and trace-path"
      (let* ((scheme-src "(export eval-query)\n(def (eval-query q) (helper-fn q))\n(def (helper-fn q) q)\n(c-define (eval-query q) (int) int \"marlin_eval_query\" \"\" (eval-query q))\n")
             (_scheme-facts
               (let-values (((ents edges)
                             (extract-scheme-open-edb-facts "scheme/search-dsl/funcs.ss" scheme-src)))
                 (cons ents edges)))
             (scheme-ents (car _scheme-facts))
             (eval-ent (find (lambda (v) (string=? (vector-ref v 2) "eval-query")) scheme-ents))
             (eval-attrs (if eval-ent (vector->list (vector-ref eval-ent 7)) '()))
             (helper-ent (find (lambda (v) (string=? (vector-ref v 2) "helper-fn")) scheme-ents))
             (helper-attrs (if helper-ent (vector->list (vector-ref helper-ent 7)) '()))
             (edb
               '((file
                   (1 "rust" "crates/bridge/src/runner.rs")
                   (2 "scheme" "scheme/search-dsl/funcs.ss")
                   (3 "scheme" "scheme/c-abi/search-dsl.ss"))
                 (entity
                   ("e1" 1 "rust" "struct" "struct_item" "SearchExecutionRequest" 10)
                   ("e2" 1 "rust" "struct" "struct_item" "GraphLoopRequest" 25)
                   ("e3" 1 "rust" "function" "fn_item" "execute_search_dsl" 40)
                   ("e4" 1 "rust" "function" "fn_item" "invoke_c_abi" 70)
                   ("e5" 2 "scheme" "function" "def_proc" "eval-query" 2)
                   ("e6" 2 "scheme" "function" "def_proc" "helper-fn" 3)
                   ("e7" 3 "scheme" "function" "c_define" "marlin_eval_query" 4))
                 (edge
                   ("e3" "e4" "calls" "crates/bridge/src/runner.rs" 48)
                   ("e4" "e7" "calls" "crates/bridge/src/runner.rs" 75)
                   ("eval-query" "marlin_eval_query" "ffi_binds" "scheme/c-abi/search-dsl.ss" 4)
                   ("e5" "e6" "calls" "scheme/search-dsl/funcs.ss" 2))
                 (attr
                   ("e1" "visibility" "pub")
                   ("e3" "visibility" "pub")
                   ("e3" "async" "true")
                   ("e5" "exported" "true")
                   ("e5" "moduleRole" "funcs")
                   ("e6" "moduleRole" "funcs"))
                 (symbol_ref)))
             ;; 1) `name-all:` AND semantics + Rust trigram prefilter pushdown
             (and-pref
               (compile-search-dsl-prefilter
                 "(find-symbols name-all: (\"search\" \"request\") lang: \"rust\" limit: 20)"
                 SearchLanguageRegistryValue))
             (or-pref
               (compile-search-dsl-prefilter
                 "(find-symbols name-contains: (\"search\" \"request\") lang: \"rust\" limit: 20)"
                 SearchLanguageRegistryValue))
             (and-res
               (evaluate-search-dsl-over-edb
                 "(find-symbols name-all: (\"search\" \"request\") lang: \"rust\" limit: 20)"
                 edb SearchLanguageRegistryValue))
             ;; 2) ASP `special:` facet filtering over Open EDB `attr`
             (rust-special-res
               (evaluate-search-dsl-over-edb
                 "(find-symbols lang: \"rust\" kind: \"fn\" special: ((visibility \"pub\") (async #t)) limit: 20)"
                 edb SearchLanguageRegistryValue))
             (scheme-special-res
               (evaluate-search-dsl-over-edb
                 "(find-symbols lang: \"scheme\" special: ((exported #t) (moduleRole \"funcs\")) limit: 20)"
                 edb SearchLanguageRegistryValue))
             ;; 3) Cross-language `ffi` edges inside `find-impact`
             (ffi-impact-res
               (evaluate-search-dsl-over-edb
                 "(find-impact \"marlin_eval_query\" limit: 20)"
                 edb SearchLanguageRegistryValue))
             ;; 4) Multi-hop `(trace-path from: ... to: ...)` crossing Rust calls -> FFI -> Scheme
             (trace-res
               (evaluate-search-dsl-over-edb
                 "(trace-path from: \"execute_search_dsl\" to: \"helper-fn\" limit: 20)"
                 edb SearchLanguageRegistryValue)))
        ;; Scheme extractor emits ASP `exported` + `moduleRole` special attrs
        (check (and (member #("exported" "true") eval-attrs)
                    (member #("moduleRole" "funcs") eval-attrs)
                    (not (member #("exported" "true") helper-attrs))
                    (member #("moduleRole" "funcs") helper-attrs)
                    #t)
               => #t)
        ;; Prefilter pushes BOTH `name-all:` tokens into `required_tokens`, but skips OR branches
        (check (vector-ref and-pref 3) => #("search" "request"))
        (check (vector-ref or-pref 3) => #())
        ;; `name-all:` matches ONLY `SearchExecutionRequest` (NOT `GraphLoopRequest`)
        (check (vector-ref and-res 3) => '(("SearchExecutionRequest" "crates/bridge/src/runner.rs" 10)))
        ;; `special:` facet filters Rust `pub async` fn and Scheme exported `funcs` fn
        (check (vector-ref rust-special-res 3) => '(("execute_search_dsl" "crates/bridge/src/runner.rs" 40)))
        (check (vector-ref scheme-special-res 3) => '(("eval-query" "scheme/search-dsl/funcs.ss" 2)))
        ;; `find-impact` includes cross-language `"ffi"` C-ABI binding
        (check (and (member '("ffi" "prod" "eval-query" "scheme/c-abi/search-dsl.ss" 4)
                            (vector-ref ffi-impact-res 3))
                    #t)
               => #t)
        ;; `trace-path` discovers the complete cross-language call + FFI chain in 1 query!
        (check (and (member '("calls" "execute_search_dsl" "invoke_c_abi" "crates/bridge/src/runner.rs" 48)
                            (vector-ref trace-res 3))
                    (member '("calls" "invoke_c_abi" "marlin_eval_query" "crates/bridge/src/runner.rs" 75)
                            (vector-ref trace-res 3))
                    (member '("ffi" "marlin_eval_query" "eval-query" "scheme/c-abi/search-dsl.ss" 4)
                            (vector-ref trace-res 3))
                    (member '("calls" "eval-query" "helper-fn" "scheme/search-dsl/funcs.ss" 2)
                            (vector-ref trace-res 3))
                    #t)
               => #t)))

    (test-case "LLM-ASP P1: orthography-normalized cross-language name matching and prefilter stem splitting"
      (let* ((edb
               '((file
                   (1 "rust" "crates/kernel/src/gov.rs")
                   (2 "scheme" "scheme/policy/gov.ss"))
                 (entity
                   ("g1" 1 "rust" "struct" "struct_item" "ReadOnlyGovernanceGate" 12)
                   ("g2" 1 "rust" "function" "fn_item" "enforce_read_only_policy" 28)
                   ("g3" 2 "scheme" "function" "def_proc" "read-only-policy-ok?" 15))
                 (edge)
                 (attr)
                 (symbol_ref)))
             (pref
               (compile-search-dsl-prefilter
                 "(find-symbols name-contains: \"read-only\" lang: \"any\" limit: 20)"
                 SearchLanguageRegistryValue))
             (res
               (evaluate-search-dsl-over-edb
                 "(find-symbols name-contains: \"read-only\" lang: \"any\" limit: 20)"
                 edb SearchLanguageRegistryValue))
             (names (map car (vector-ref res 3))))
        ;; Prefilter splits `"read-only"` into atomic stems `#("read" "only")` so CamelCase & snake_case survive trigram pushdown
        (check (vector-ref pref 3) => #("read" "only"))
        ;; Matches CamelCase Rust struct, snake_case Rust fn, AND kebab-case Scheme fn in one query!
        (check (and (member "ReadOnlyGovernanceGate" names)
                    (member "enforce_read_only_policy" names)
                    (member "read-only-policy-ok?" names)
                    (= (length names) 3)
                    #t)
               => #t)))))

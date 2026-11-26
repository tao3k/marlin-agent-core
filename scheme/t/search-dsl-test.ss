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
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports)
        (only-in :marlin-agent-core/search-dsl/syntax
                 relational-choice-program)
        (only-in :marlin-agent-core/search-dsl/funcs
                 read-ascent-search-form
                 repair-ascent-search-form
                 compile-search-dsl-prefilter
                 evaluate-search-dsl-over-edb
                 evaluate-choice-program-over-edb
                 search-dsl-completions
                 parse-search-dsl-selector))
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
      ;; 4. Missing `(limits ...)` in Dusa `(relational-choice-program ...)`
      (let ((r4 (evaluate-search-dsl-over-edb
                  "(relational-choice-program
                     (choice tier (f t))
                     (rule (closed (tier ?f) ((value 'hot) (value 'warm)))
                           (file ?f (value 'rust) ?_))
                     (rule (closed (tier ?f) ((value 'hot)))
                           (rust_fn ?_ ?f (value 'open_or_build_cached) ?_)))"
                  sample-edb)))
        (check (vector-ref r4 1) => 'choice)
        (check (length (vector-ref r4 3)) => 2)
        (check (and (memq 'append-default-limits (vector-ref r4 4)) #t) => #t)))

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
        (check (vector-ref (vector-ref rust-rows 0) 1) => "relation")))))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO General Language Ontology & Composable Language Extensions (`scheme/search-dsl`).
;;;
;;; Architectural Principle (ASP RFC 10.36):
;;;   1. `GeneralLanguageOntologyFragment.` defines universal cross-language
;;;      `:gerbil-ascent` relations (`callable`, `type_decl`, `module_decl`,
;;;      `import_decl`, `binding_decl`, `macro_decl`, `comment_node`, `test_decl`)
;;;      directly over ASP's `semantic-open-edb.v1` protocol. Any newly registered
;;;      language provider participates in general search immediately.
;;;   2. Each native language extends the General Ontology via POO `.mix`
;;;      (`defasp-language`), supplying ONLY:
;;;        - Native compiler `language_kind` -> General Ontology `kind` mappings
;;;        - Language-specific special relations (`rust_impl`, `scheme_poo_def`,
;;;          `julia_macro`, `ts_interface`, `py_class`, ...)
;;;        - Golden conformance probes projected to ASP's black-box provider harness
;;;   3. Low-level lexical syntax (comment openers `//` / `#` / `;`, delimiter
;;;      pairs, string quotes) is NEVER hardcoded here: each external provider
;;;      owns its native compiler parser and reports `ProviderSyntaxSurface` up.
(import (only-in :clan/poo/object .def .mix seal-object!)
        "./syntax.ss")
(export BaseSearchLanguageFragment.
        GeneralLanguageOntologyFragment.
        RustSearchFragment.
        SchemePooSearchFragment.
        TypeScriptSearchFragment.
        PythonSearchFragment.
        JuliaSearchFragment.
        ModelSearchFragment.
        SearchLanguageRegistry.
        SearchLanguageRegistryValue
        WorkspaceSearchToolSpecValue)

;;; Real compiled `:gerbil-ascent` Open EDB base fragment aligned 1:1 with
;;; ASP's `schemas/semantic-open-edb.v1.schema.json`.
(def +open-edb-base-fragment+
  (ascent-fragment
    (relation file ((id integer?) (lang string?) (path string?)))
    (relation entity
      ((id string?) (file_id integer?) (lang string?)
       (kind string?) (language_kind string?)
       (name string?) (line integer?)))
    (relation edge
      ((src string?) (dst string?) (rel string?) (path string?) (line integer?)))
    (relation attr ((id string?) (key string?) (val string?)))
    (relation is_test_path ((path string?)))
    (relation prod_path ((path string?)))
    (relation path_has_prefix ((path string?) (prefix string?)))
    (relation path_contains ((path string?) (needle string?)))
    (relation name_contains ((name string?) (needle string?)))
    (relation name_has_prefix ((name string?) (prefix string?)))
    (relation symbol_ref ((name string?) (path string?) (line integer?)))))

;;; Universal cross-language General Language Ontology derived over Open EDB.
;;; Every language shares these general semantic categories; language-specific
;;; constructs extend them cleanly through POO `.mix`.
(def +general-language-ontology-fragment+
  (ascent-fragment
    (relation file ((id integer?) (lang string?) (path string?)))
    (relation entity
      ((id string?) (file_id integer?) (lang string?)
       (kind string?) (language_kind string?)
       (name string?) (line integer?)))
    (relation edge
      ((src string?) (dst string?) (rel string?) (path string?) (line integer?)))
    (relation attr
      ((id string?) (key string?) (val string?)))
    (relation is_test_path ((path string?)))
    (relation prod_path ((path string?)))
    (relation path_has_prefix ((path string?) (prefix string?)))
    (relation path_contains ((path string?) (needle string?)))
    (relation name_contains ((name string?) (needle string?)))
    (relation name_has_prefix ((name string?) (prefix string?)))
    (relation symbol_ref ((name string?) (path string?) (line integer?)))
    (relation callable
      ((id string?) (file_id integer?) (lang string?) (name string?) (line integer?)))
    (relation type_decl
      ((id string?) (file_id integer?) (lang string?) (name string?) (line integer?)))
    (relation module_decl
      ((id string?) (file_id integer?) (lang string?) (name string?) (line integer?)))
    (relation import_decl
      ((id string?) (file_id integer?) (lang string?) (name string?) (line integer?)))
    (relation binding_decl
      ((id string?) (file_id integer?) (lang string?) (name string?) (line integer?)))
    (relation macro_decl
      ((id string?) (file_id integer?) (lang string?) (name string?) (line integer?)))
    (relation comment_node
      ((id string?) (file_id integer?) (lang string?) (name string?) (line integer?)))
    (relation test_decl
      ((id string?) (file_id integer?) (lang string?) (name string?) (line integer?)))
    (relation contains_edge
      ((parent string?) (child string?) (path string?) (line integer?)))
    (relation imports_edge
      ((src string?) (dst string?) (path string?) (line integer?)))
    (relation calls_edge
      ((src string?) (dst string?) (path string?) (line integer?)))
    (relation calls_symbol
      ((caller_name string?) (callee_name string?) (path string?) (line integer?)))
    (relation implements_edge
      ((src string?) (dst string?) (path string?) (line integer?)))
    (relation impl_trait
      ((type_name string?) (trait_name string?) (path string?) (line integer?)))
    (relation entity_locator
      ((id string?) (locator string?)))
    (relation depends_on
      ((src string?) (dst string?)))
    (relation verified_tier
      ((mod string?) (level string?)))
    (relation verified_resolution
      ((callsite string?) (target string?)))
    (relation verified_evidence
      ((target string?) (val string?)))

    ((callable id f lang name line) <-- (entity id f lang "function" _ name line))
    ((callable id f lang name line) <-- (entity id f lang "method" _ name line))

    ((type_decl id f lang name line) <-- (entity id f lang "struct" _ name line))
    ((type_decl id f lang name line) <-- (entity id f lang "class" _ name line))
    ((type_decl id f lang name line) <-- (entity id f lang "enum" _ name line))
    ((type_decl id f lang name line) <-- (entity id f lang "trait" _ name line))
    ((type_decl id f lang name line) <-- (entity id f lang "interface" _ name line))
    ((type_decl id f lang name line) <-- (entity id f lang "type" _ name line))

    ((module_decl id f lang name line) <-- (entity id f lang "module" _ name line))
    ((import_decl id f lang name line) <-- (entity id f lang "import" _ name line))
    ((binding_decl id f lang name line) <-- (entity id f lang "binding" _ name line))
    ((binding_decl id f lang name line) <-- (entity id f lang "constant" _ name line))
    ((macro_decl id f lang name line) <-- (entity id f lang "macro" _ name line))
    ((comment_node id f lang name line) <-- (entity id f lang "doc" _ name line))
    ((test_decl id f lang name line) <-- (entity id f lang "test" _ name line))

    ((contains_edge parent child path line) <-- (edge parent child "contains" path line))
    ((imports_edge src dst path line) <-- (edge src dst "imports" path line))
    ((calls_edge src dst path line) <-- (edge src dst "calls" path line))
    ((calls_symbol caller_name callee_name path line)
     <-- (edge src_id dst_id "calls" path line)
         (entity src_id _ _ _ _ caller_name _)
         (entity dst_id _ _ _ _ callee_name _))
    ((calls_symbol caller_name dst_name path line)
     <-- (edge src_id dst_name "calls" path line)
         (entity src_id _ _ _ _ caller_name _)
         (not (entity dst_name _ _ _ _ _ _)))
    ((calls_symbol src_name dst_name path line)
     <-- (edge src_name dst_name "calls" path line)
         (not (entity src_name _ _ _ _ _ _))
         (not (entity dst_name _ _ _ _ _ _)))
    ((implements_edge src dst path line) <-- (edge src dst "implements" path line))
    ((impl_trait name trait_name path line)
     <-- (entity id f _ _ _ name line) (attr id "implements" trait_name) (file f _ path))
    ((impl_trait name trait_name path line)
     <-- (entity id f _ _ _ name line) (attr id "trait" trait_name) (file f _ path))
    ((entity_locator id loc) <-- (attr id "locator" loc))
    ((verified_tier mod level) <-- (attr mod "verified_tier" level))
    ((verified_resolution callsite target) <-- (attr callsite "verified_resolution" target))
    ((verified_evidence target val) <-- (attr target "verified_evidence" val))
    ((depends_on src dst) <-- (edge src dst "imports" _ _))
    ((depends_on src dst) <-- (edge src dst "calls" _ _))
    ((depends_on src dst) <-- (edge src dst "implements" _ _))
    ((depends_on src dst) <-- (depends_on src mid) (depends_on mid dst))))

(.def BaseSearchLanguageFragment.
  languages: '()
  language-contracts: '()
  protocol-json-schemas: '()
  ascent-fragments: (list +open-edb-base-fragment+)
  relation-meta:
  '((file            "edb" "file"            _ "Open EDB file fact (id, lang, path)")
    (entity          "edb" "entity"          _ "Open EDB entity fact (id, file_id, lang, kind, language_kind, name, line)")
    (edge            "edb" "edge"            _ "Open EDB directed relation edge (src, dst, rel, path, line)")
    (attr            "edb" "attr"            _ "Open EDB extensible attribute facet (id, key, val)")
    (is_test_path    "edb" "is_test_path"    _ "True when file path is a test, mock, or fixture file (path)")
    (prod_path       "edb" "prod_path"       _ "True when file path is production code, not test/mock/fixture (path)")
    (path_has_prefix "edb" "path_has_prefix" _ "True when file path starts with literal prefix (path, prefix)")
    (path_contains   "edb" "path_contains"   _ "True when file path contains literal substring (path, needle)")
    (name_contains   "edb" "name_contains"   _ "Case-insensitive entity name substring match (name, needle)")
    (name_has_prefix "edb" "name_has_prefix" _ "Case-insensitive entity name prefix match (name, prefix)")
    (symbol_ref      "edb" "symbol_ref"      _ "Exact word-boundary symbol reference in scoped RAM files (name, path, line)"))
  syntax-forms:
  '(("(find-impact ...)"
     "ascent"
     "(find-impact \"Symbol\" path-prefix: \"crates/\" exclude-tests?: #f limit: 60)"
     "ascent"
     "One-shot def/caller/impl/import/ref blast radius (100% RAM coverage, prod vs test)"
     "(find-impact \"SymbolName\" path-prefix: \"crates/\" limit: 60)")
    ("(find-symbols ...)"
     "ascent"
     "(find-symbols lang: \"rust\" kind: \"struct\" name-contains: \"...\" exclude-tests?: #t limit: 60)"
     "ascent"
     "Polyglot case-insensitive symbol discovery across code languages"
     "(find-symbols lang: \"rust\" kind: \"struct\" exclude-tests?: #t limit: 60)")
    ("(find-collisions ...)"
     "ascent"
     "(find-collisions lang: \"rust\" kind: \"struct\" path-prefix: \"crates/\" min-files: 2 exclude-tests?: #t limit: 40)"
     "ascent"
     "Cross-file duplicate symbol detector with niche-diverse sampling"
     "(find-collisions lang: \"rust\" kind: \"struct\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40)")
    ("(find-callers ...)"
     "ascent"
     "(find-callers \"fn_name\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40 context: 2)"
     "ascent"
     "Structural call-graph reverse lookup for callers of a symbol"
     "(find-callers \"target_fn\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40)")
    ("(find-impls ...)"
     "ascent"
     "(find-impls \"TraitName\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40)"
     "ascent"
     "Find all types implementing a trait or interface"
     "(find-impls \"TraitName\" path-prefix: \"crates/\" exclude-tests?: #t limit: 40)")
    ("(brute-search ...)"
     "parser"
     "(brute-search \"pattern\" lang: \"rust\" path-prefix: \"crates/\" exclude-tests?: #t context: 2 limit: 32)"
     "l0-ram"
     "Sub-millisecond pure-Rust RAM trigram/BM25/regex text search"
     "(brute-search \"pattern\" lang: \"rust\" path-prefix: \"crates/\" exclude-tests?: #t limit: 32)")
    ("(file-outline ...)"
     "parser"
     "(file-outline \"path/\" lang: \"rust\" kind: \"any\" exclude-tests?: #t limit: 80)"
     "l0-ram"
     "Zero-I/O RAM AST symbol skeleton with exact line numbers"
     "(file-outline \"crates/\" lang: \"rust\" kind: \"struct\" exclude-tests?: #t limit: 80)")
    ("(read-file ...)"
     "parser"
     "(read-file \"path/to/file.rs\" symbol: \"Name\" context: 25)"
     "l0-ram"
     "Zero-I/O RAM file reader anchored by symbol name or line range"
     "(read-file \"path/to/file.rs\" symbol: \"Name\" context: 25)")
    ("(solve-placement ...)"
     "dusa"
     "(solve-placement path-prefix: \"crates/.../src/\" module-limit: 6 choices: (resident provider) forbid-hot-path: provider state-budget: 64)"
     "dusa"
     "Dusa finite-choice architecture placement solver with forced/free provenance"
     "(solve-placement path-prefix: \"crates/marlin-agent-search-engine/src/\" module-limit: 6 choices: (resident provider) forbid-hot-path: provider state-budget: 64)")
    ("(solve-resolve ...)"
     "dusa"
     "(solve-resolve path-prefix: \"crates/.../src/\" module-limit: 6 exclude-tests?: #t state-budget: 32)"
     "dusa"
     "Dusa finite-choice multi-definition callsite disambiguator"
     "(solve-resolve path-prefix: \"crates/marlin-agent-search-engine/src/\" module-limit: 6 exclude-tests?: #t state-budget: 32)")
    ("(ascent-fragment ...)"
     "ascent"
     "(ascent-fragment (relation answer ((path string?) (name string?))) ((answer path name) <-- ...))"
     "ascent"
     "Real :gerbil-ascent composable fragment (zero-eval closed-source)"
     "(ascent-fragment (relation answer ((path string?) (name string?) (line integer?))) ((answer path name line) <-- (file f \"rust\" path) (rust_fn id f name line)))")
    ("(ascent ...)"
     "ascent"
     "(ascent (relation ...) ((head ...) <-- body ...))"
     "ascent"
     "Real :gerbil-ascent Datalog program (recursion, stratified negation, guards)"
     "(ascent (relation reach ((a string?) (b string?))) ((reach a b) <-- (edge a b _ _ _)) ((reach a c) <-- (reach a b) (edge b c _ _ _)))")
    ("(search-choice-program ...)"
     "dusa"
     "(search-choice-program (use rust) (choice r (k v)) (rule (closed ...) ...) (solve r state-budget: 32 provenance?: #t))"
     "dusa"
     "High-level Dusa POPL 2025 Search + Disambiguation + Architecture Constraint Solver with provenance witnesses"
     "(search-choice-program (use rust) (choice tier (mod level)) (rule (closed (tier ?m) (resident provider)) (module-in-scope ?m)) (rule (closed (tier ?m) ()) (hot-path-module ?m) (tier ?m 'provider)) (solve tier state-budget: 64 provenance?: #t))")
    ("(relational-choice-program ...)"
     "dusa"
     "(relational-choice-program (choice r (k v)) (rule (closed ...) ...) (limits ...))"
     "dusa"
     "Real :gerbil-ascent Lean-verified Dusa POPL 2025 finite-choice program"
     "(relational-choice-program (choice tier (f t)) (rule (closed (tier ?f) ((value 'hot) (value 'warm))) (file ?f (value 'rust) ?_)) (limits 4096 4096 4096))")
    ("(search-selector \"...\")"
     "parser"
     "(search-selector \"lang:rust & rust::open\")"
     "gerbil-parser"
     "Typed structured selector parsed in Scheme via :gerbil-parser"
     "(search-selector \"lang:rust & rust::open\")")
    ("(not (...))"
     "ascent"
     "(not (rel arg ...))"
     "ascent"
     "Stratified negation in :gerbil-ascent rule bodies"
     "(not (edge from to \"imports\" _ _))")
    ("(if (x) expr)"
     "ascent"
     "(if (x) (< x 100))"
     "ascent"
     "Closed checked guard expression in :gerbil-ascent rule bodies"
     "(if (line) (> line 10))")
    ("(closed (rel ?k ...) (v1 v2))"
     "dusa"
     "(rule (closed (rel ?k ...) (v ...)) body ...)"
     "dusa"
     "Dusa closed choice constraint (empty () = hard forbid)"
     "(rule (closed (role ?id) ((value 'primary) (value 'fallback))) (entity ?id ?_ ?_ ?_ ?_ ?_ ?_))")
    ("(open (rel ?k ...) v)"
     "dusa"
     "(rule (open (rel ?k ...) v) body ...)"
     "dusa"
     "Dusa open choice offer (non-exclusive candidate)"
     "(rule (open (role ?id) (value 'candidate)) (entity ?id ?_ ?_ ?_ ?_ ?_ ?_))")))

(.def GeneralLanguageOntologyFragment.
  (ascent-fragments => append (list +general-language-ontology-fragment+))
  (relation-meta => append
    '((callable     "general" "callable"     _ "General Ontology callable across all languages (function | method)")
      (type_decl    "general" "type_decl"    _ "General Ontology type declaration (struct | class | enum | trait | interface | type)")
      (module_decl  "general" "module_decl"  _ "General Ontology module/namespace declaration")
      (import_decl  "general" "import_decl"  _ "General Ontology module import declaration")
      (binding_decl "general" "binding_decl" _ "General Ontology binding or constant declaration")
      (macro_decl   "general" "macro_decl"   _ "General Ontology macro/metaprogramming declaration")
      (comment_node    "general"    "comment_node" _ "General Ontology doc/comment node (lexed natively by provider)")
      (test_decl       "general"    "test_decl"    _ "General Ontology test case declaration")
      (contains_edge   "graph"      "contains"     _ "Structural parent-to-child AST containment edge (parent, child, path, line)")
      (imports_edge    "graph"      "imports"      _ "Module/file import dependency edge (src, dst, path, line)")
      (calls_edge      "graph"      "calls"        _ "Directed call graph edge (src_id, dst_id, path, line)")
      (calls_symbol    "graph"      "calls_symbol" _ "Symbol-level call graph edge (caller_name, callee_name, path, line)")
      (implements_edge "graph"      "implements"   _ "Trait/interface implementation edge (src, dst, path, line)")
      (impl_trait      "graph"      "impl_trait"   _ "Trait/interface implementation relation (type_name, trait_name, path, line)")
      (entity_locator      "graph-pair" "locator"             _ "Authoritative AST patch locator for an entity (id, locator)")
      (verified_tier       "graph-pair" "verified_tier"       _ "Admitted M1 Scenario B architecture tier evidence (mod, level)")
      (verified_resolution "graph-pair" "verified_resolution" _ "Admitted M1 Scenario B callsite disambiguation evidence (callsite, target)")
      (verified_evidence   "graph-pair" "verified_evidence"   _ "Admitted M1 search evidence chain fact (target, val)")
      (depends_on          "graph-pair" "depends_on"          _ "Transitive dependency closure over imports, calls, and implements"))))

(defasp-language RustSearchFragment.
  (languages rust)
  (provider-language "rust")
  (extensions ".rs")
  (project-markers "Cargo.toml")
  (kind-map
    ("fn_item"     "function" "Rust free function item")
    ("impl_fn"     "method"   "Rust inherent or trait impl method")
    ("struct_item" "struct"   "Rust struct declaration")
    ("enum_item"   "enum"     "Rust enum declaration")
    ("trait_item"  "trait"    "Rust trait declaration")
    ("type_item"   "type"     "Rust type alias")
    ("impl_item"   "item"     "Rust impl block (POO special extension)")
    ("macro_rules" "macro"    "Rust macro_rules! definition")
    ("mod_item"    "module"   "Rust inline or external module")
    ("use_item"    "import"   "Rust use import declaration"))
  (relations
    (rust_fn     "rust" "function" _           "Rust function declaration")
    (rust_method "rust" "method"   _           "Rust method declaration")
    (rust_struct "rust" "struct"   _           "Rust struct declaration")
    (rust_enum   "rust" "enum"     _           "Rust enum declaration")
    (rust_trait  "rust" "trait"    _           "Rust trait declaration")
    (rust_type   "rust" "type"     _           "Rust type definition")
    (rust_impl   "rust" "item"     "impl_item" "Rust impl block (native language_kind=impl_item)")
    (rust_macro  "rust" "macro"    _           "Rust macro declaration (macro_rules!/proc_macro)")
    (rust_mod    "rust" "module"   _           "Rust module declaration"))
  (conformance-probes
    ("rust-valid-struct-and-fn"
     "src/lib.rs"
     "pub struct Widget {\n    pub count: usize,\n}\n\npub fn add(a: usize, b: usize) -> usize {\n    a + b\n}\n"
     #t
     (("struct" "struct_item" "Widget")
      ("function" "fn_item" "add")))
    ("rust-invalid-unclosed-brace"
     "src/broken.rs"
     "pub fn broken(x: i32) -> i32 {\n    x + 1\n"
     #f
     ())))

(defasp-language SchemePooSearchFragment.
  (languages scheme)
  (provider-language "gerbil-scheme")
  (extensions ".ss" ".scm")
  (project-markers "gerbil.pkg" "manifest.ss")
  (kind-map
    ("def"         "function" "Gerbil procedure or method definition")
    ("defconst"    "constant" "Gerbil constant binding")
    ("defstruct"   "struct"   "Gerbil struct declaration")
    ("defclass"    "type"     "Gerbil class declaration")
    ("defsyntax"   "macro"    "Gerbil hygienic macro (defsyntax/defrules)")
    ("poo_def"     "item"     "Gerbil POO prototype (.def ...)")
    ("deflanguage" "item"     ":gerbil-parser deflanguage grammar")
    ("import"      "import"   "Gerbil module import declaration"))
  (relations
    (scheme_def     "scheme" "function" _             "Gerbil procedure or method definition")
    (scheme_const   "scheme" "constant" _             "Gerbil value binding")
    (scheme_struct  "scheme" "type"     _             "Gerbil struct/type declaration")
    (scheme_class   "scheme" "class"    _             "Gerbil class declaration")
    (scheme_macro   "scheme" "macro"    _             "Gerbil defsyntax / defrules macro")
    (scheme_poo_def "scheme" "item"     "poo_def"     "Gerbil POO prototype (.def ...)")
    (scheme_grammar "scheme" "item"     "deflanguage" ":gerbil-parser deflanguage grammar"))
  (conformance-probes
    ("scheme-valid-def-and-poo"
     "src/sample.ss"
     ";; Gerbil Scheme comment\n(def (add x y)\n  (+ x y))\n\n(.def SampleProto.\n  value: 42)\n"
     #t
     (("function" "def" "add")
      ("item" "poo_def" "SampleProto.")))
    ("scheme-invalid-unclosed-paren"
     "src/broken.ss"
     "(def (broken x)\n  (+ x 1)\n"
     #f
     ())))

(defasp-language TypeScriptSearchFragment.
  (languages typescript)
  (provider-language "typescript")
  (extensions ".ts" ".tsx")
  (project-markers "tsconfig.json" "package.json")
  (kind-map
    ("function_declaration"   "function" "TypeScript function declaration")
    ("method_declaration"     "method"   "TypeScript class method declaration")
    ("class_declaration"      "struct"   "TypeScript class declaration")
    ("interface_declaration"  "trait"    "TypeScript interface declaration")
    ("type_alias_declaration" "type"     "TypeScript type alias declaration")
    ("import_declaration"     "import"   "TypeScript import declaration"))
  (relations
    (ts_fn        "typescript" "function" _                       "TypeScript function declaration")
    (ts_class     "typescript" _          "class_declaration"     "TypeScript class declaration")
    (ts_interface "typescript" _          "interface_declaration" "TypeScript interface declaration")
    (ts_type      "typescript" "type"     _                       "TypeScript type/enum declaration"))
  (conformance-probes
    ("ts-valid-interface-and-fn"
     "src/math.ts"
     "// TypeScript comment\nexport interface Point {\n  x: number;\n}\n\nexport function sum(a: number, b: number): number {\n  return a + b;\n}\n"
     #t
     (("trait" "interface_declaration" "Point")
      ("function" "function_declaration" "sum")))
    ("ts-invalid-unclosed-brace"
     "src/broken.ts"
     "export function broken(x: number): number {\n  return x + 1;\n"
     #f
     ())))

(defasp-language PythonSearchFragment.
  (languages python)
  (provider-language "python")
  (extensions ".py")
  (project-markers "pyproject.toml")
  (kind-map
    ("function_def"       "function" "Python function definition")
    ("function_def"       "method"   "Python class method definition")
    ("async_function_def" "function" "Python async function definition")
    ("async_function_def" "method"   "Python async class method definition")
    ("class_def"          "struct"   "Python class definition")
    ("import"             "import"   "Python import statement")
    ("import_from"        "import"   "Python from-import statement"))
  (relations
    (py_fn    "python" "function" _           "Python function definition")
    (py_class "python" _          "class_def" "Python class definition"))
  (conformance-probes
    ("python-valid-class-and-fn"
     "src/math_ops.py"
     "# Python comment\nclass Box:\n    def get(self):\n        return 1\n\ndef add(a, b):\n    return a + b\n"
     #t
     (("struct" "class_def" "Box")
      ("function" "function_def" "add")))
    ("python-invalid-unclosed-paren"
     "src/broken.py"
     "def broken(x:\n    return x\n"
     #f
     ())))

(defasp-language JuliaSearchFragment.
  (languages julia)
  (provider-language "julia")
  (extensions ".jl")
  (project-markers "Project.toml")
  (kind-map
    ("function" "function" "Julia function / multiple-dispatch method definition")
    ("macro"    "macro"    "Julia hygienic macro definition")
    ("struct"   "struct"   "Julia concrete struct definition")
    ("abstract" "trait"    "Julia abstract type hierarchy node")
    ("module"   "module"   "Julia module namespace")
    ("using"    "import"   "Julia using import declaration")
    ("import"   "import"   "Julia import declaration"))
  (relations
    (julia_fn       "julia" "function" _          "Julia function / multiple-dispatch method")
    (julia_macro    "julia" "macro"    _          "Julia macro definition")
    (julia_struct   "julia" "struct"   _          "Julia struct type definition")
    (julia_abstract "julia" "trait"    "abstract" "Julia abstract type definition")
    (julia_module   "julia" "module"   _          "Julia module declaration"))
  (conformance-probes
    ("julia-valid-struct-and-fn"
     "src/math.jl"
     "# Julia comment\nstruct Point\n    x::Float64\nend\n\nfunction add(x, y)\n    return x + y\nend\n"
     #t
     (("struct" "struct" "Point")
      ("function" "function" "add")))
    ("julia-invalid-missing-end"
     "src/broken.jl"
     "function broken(x)\n    return x + 1\n"
     #f
     ())))

(defsearch-language ModelSearchFragment.
  (languages sysml quint org)
  (relations
    (sysml_requirement "sysml" "item"     "requirement_def" "OpenSysML v2 requirement block")
    (sysml_part        "sysml" "type"     _                 "OpenSysML v2 part/type definition")
    (sysml_state       "sysml" "item"     "state_def"       "OpenSysML v2 state machine")
    (sysml_action      "sysml" "function" _                 "OpenSysML v2 action definition")
    (quint_action      "quint" "function" _                 "Quint formal state action")
    (quint_invariant   "quint" "item"     "quint_invariant" "Quint formal invariant (val)")
    (org_heading       "org"   "heading"  _                 "Org-mode heading")))

(def SearchLanguageRegistry.
  (.mix ModelSearchFragment.
        JuliaSearchFragment.
        PythonSearchFragment.
        TypeScriptSearchFragment.
        SchemePooSearchFragment.
        RustSearchFragment.
        GeneralLanguageOntologyFragment.
        BaseSearchLanguageFragment.))

(def SearchLanguageRegistryValue
  (seal-object! SearchLanguageRegistry.))

(defsearch-tool-spec WorkspaceSearchToolSpecValue
  (forms brute-search file-outline read-file ascent-fragment ascent search-choice-program relational-choice-program)
  (edb file entity edge attr)
  (registry SearchLanguageRegistryValue))

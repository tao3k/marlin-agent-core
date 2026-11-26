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
    (relation attr ((id string?) (key string?) (val string?)))))

;;; Universal cross-language General Language Ontology derived over Open EDB.
;;; Every language shares these general semantic categories; language-specific
;;; constructs extend them cleanly through POO `.mix`.
(def +general-language-ontology-fragment+
  (ascent-fragment
    (relation entity
      ((id string?) (file_id integer?) (lang string?)
       (kind string?) (language_kind string?)
       (name string?) (line integer?)))
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
    ((test_decl id f lang name line) <-- (entity id f lang "test" _ name line))))

(.def BaseSearchLanguageFragment.
  languages: '()
  language-contracts: '()
  ascent-fragments: (list +open-edb-base-fragment+)
  relation-meta:
  '((file   "edb" "file"   _ "Open EDB file fact (id, lang, path)")
    (entity "edb" "entity" _ "Open EDB entity fact (id, file_id, lang, kind, language_kind, name, line)")
    (edge   "edb" "edge"   _ "Open EDB directed relation edge (src, dst, rel, path, line)")
    (attr   "edb" "attr"   _ "Open EDB extensible attribute facet (id, key, val)"))
  syntax-forms:
  '(("(ascent-fragment ...)"
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
      (comment_node "general" "comment_node" _ "General Ontology doc/comment node (lexed natively by provider)")
      (test_decl    "general" "test_decl"    _ "General Ontology test case declaration"))))

(defasp-language RustSearchFragment.
  (languages rust)
  (provider-language "rust")
  (extensions ".rs")
  (project-markers "Cargo.toml")
  (kind-map
    ("fn_item"     "function" "Rust free function item")
    ("impl_method" "method"   "Rust inherent or trait impl method")
    ("struct_item" "struct"   "Rust struct declaration")
    ("enum_item"   "enum"     "Rust enum declaration")
    ("trait_item"  "trait"    "Rust trait declaration")
    ("type_item"   "type"     "Rust type alias")
    ("impl_item"   "item"     "Rust impl block (POO special extension)")
    ("macro_rules" "macro"    "Rust macro_rules! definition")
    ("mod_item"    "module"   "Rust inline or external module"))
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
    ("deflanguage" "item"     ":gerbil-parser deflanguage grammar"))
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
    ("type_alias_declaration" "type"     "TypeScript type alias declaration"))
  (relations
    (ts_fn        "typescript" "function"  _ "TypeScript function declaration")
    (ts_class     "typescript" "class"     _ "TypeScript class declaration")
    (ts_interface "typescript" "interface" _ "TypeScript interface declaration")
    (ts_type      "typescript" "type"      _ "TypeScript type/enum declaration"))
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
    ("async_function_def" "function" "Python async function definition")
    ("class_def"          "struct"   "Python class definition")
    ("import"             "import"   "Python import statement")
    ("import_from"        "import"   "Python from-import statement"))
  (relations
    (py_fn    "python" "function" _ "Python function definition")
    (py_class "python" "class"    _ "Python class definition"))
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
    ("module"   "module"   "Julia module namespace"))
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
  (forms ascent-fragment ascent relational-choice-program)
  (edb file entity edge attr)
  (registry SearchLanguageRegistryValue))

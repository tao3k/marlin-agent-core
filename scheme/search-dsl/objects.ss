;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO Language Fragments & Search Language Registry (`scheme/search-dsl`).
;;; Every language fragment compiles a REAL `:gerbil-ascent` `ascent-fragment`
;;; via `defsearch-language` (`defrules`) purely over ASP's zero-regex
;;; `semantic-open-edb.v1` protocol (`lang`, universal `kind`, native
;;; `language_kind`) and composes through `.mix`.
;;;
;;; Fact sources populating ASP's Open EDB (zero regex extractors anywhere):
;;;   1. External native language providers (HTTP/gRPC: `asp-rust`,
;;;      `asp-typescript`, `asp-python`, `asp-julia`, `asp-gerbil-scheme`)
;;;      ingesting compiler AST facts (`language_kind`: `impl_item`,
;;;      `macro_rules`, `struct_item`, etc.).
;;;   2. Marlin Scheme `:gerbil-parser` (`grammar.ss`) & `orgize-fold`
;;;      ingesting lightweight/DSL/query AST facts (`poo_def`, `deflanguage`,
;;;      `requirement_def`, `quint_invariant`, etc.).
;;;   3. ASP's resident index baseline (`source = "resident-index"`) when a
;;;      file has not yet received an external provider ingestion batch.
(import (only-in :clan/poo/object .def .mix seal-object!)
        "./syntax.ss")
(export BaseSearchLanguageFragment.
        RustSearchFragment.
        SchemePooSearchFragment.
        TypeScriptSearchFragment.
        PythonSearchFragment.
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

(.def BaseSearchLanguageFragment.
  languages: '()
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

(defsearch-language RustSearchFragment.
  (languages rust)
  (relations
    (rust_fn     "rust" "function" _           "Rust function declaration")
    (rust_method "rust" "method"   _           "Rust method declaration")
    (rust_struct "rust" "struct"   _           "Rust struct declaration")
    (rust_enum   "rust" "enum"     _           "Rust enum declaration")
    (rust_trait  "rust" "trait"    _           "Rust trait declaration")
    (rust_type   "rust" "type"     _           "Rust type definition")
    (rust_impl   "rust" "item"     "impl_item" "Rust impl block (native language_kind=impl_item)")
    (rust_macro  "rust" "macro"    _           "Rust macro declaration (macro_rules!/proc_macro)")
    (rust_mod    "rust" "module"   _           "Rust module declaration")))

(defsearch-language SchemePooSearchFragment.
  (languages scheme)
  (relations
    (scheme_def     "scheme" "function" _             "Gerbil procedure or method definition")
    (scheme_const   "scheme" "constant" _             "Gerbil value binding")
    (scheme_struct  "scheme" "type"     _             "Gerbil struct/type declaration")
    (scheme_class   "scheme" "class"    _             "Gerbil class declaration")
    (scheme_macro   "scheme" "macro"    _             "Gerbil defsyntax / defrules macro")
    (scheme_poo_def "scheme" "item"     "poo_def"     "Gerbil POO prototype (.def ...)")
    (scheme_grammar "scheme" "item"     "deflanguage" ":gerbil-parser deflanguage grammar")))

(defsearch-language TypeScriptSearchFragment.
  (languages typescript)
  (relations
    (ts_fn        "typescript" "function"  _ "TypeScript function declaration")
    (ts_class     "typescript" "class"     _ "TypeScript class declaration")
    (ts_interface "typescript" "interface" _ "TypeScript interface declaration")
    (ts_type      "typescript" "type"      _ "TypeScript type/enum declaration")))

(defsearch-language PythonSearchFragment.
  (languages python)
  (relations
    (py_fn    "python" "function" _ "Python function definition")
    (py_class "python" "class"    _ "Python class definition")))

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
        PythonSearchFragment.
        TypeScriptSearchFragment.
        SchemePooSearchFragment.
        RustSearchFragment.
        BaseSearchLanguageFragment.))

(def SearchLanguageRegistryValue
  (seal-object! SearchLanguageRegistry.))

(defsearch-tool-spec WorkspaceSearchToolSpecValue
  (forms ascent-fragment ascent relational-choice-program)
  (edb file entity edge attr)
  (registry SearchLanguageRegistryValue))

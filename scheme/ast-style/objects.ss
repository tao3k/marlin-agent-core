;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Composable POO Relational Coding Style Packs (`scheme/ast-style`).
;;;
;;; Every fragment compiles a REAL `:gerbil-ascent` `ascent-fragment` over ASP's
;;; universal `semantic-open-edb.v1` + native AST attribute facets, encoding
;;; `asp-rust-rules.md` and Marlin's `scheme/CODING-STYLE.org` as executable
;;; Horn clauses with stratified negation instead of human-oriented prose.
(import (only-in :clan/poo/object .def .mix seal-object!)
        "./syntax.ss")
(export BaseCodingStyleFragment.
        RustCodingStyleFragment.
        SchemePooCodingStyleFragment.
        CodingStyleRegistry.
        CodingStyleRegistryValue)

(def +style-base-edb-fragment+
  (ascent-fragment
    (relation file ((id integer?) (lang string?) (path string?)))
    (relation entity
      ((id string?) (file_id integer?) (lang string?)
       (kind string?) (language_kind string?)
       (name string?) (line integer?)))
    (relation edge
      ((src string?) (dst string?) (rel string?) (path string?) (line integer?)))
    (relation attr ((id string?) (key string?) (val string?)))
    (relation style_finding
      ((rule_id string?) (class string?) (entity_id string?)
       (path string?) (name string?) (line integer?)))))

(.def BaseCodingStyleFragment.
  languages: '()
  style-rules: '()
  ascent-fragments: (list +style-base-edb-fragment+))

(def +rust-style-ascent-fragment+
  (ascent-fragment
    (relation file ((id integer?) (lang string?) (path string?)))
    (relation entity
      ((id string?) (file_id integer?) (lang string?)
       (kind string?) (language_kind string?)
       (name string?) (line integer?)))
    (relation attr ((id string?) (key string?) (val string?)))
    (relation style_finding
      ((rule_id string?) (class string?) (entity_id string?)
       (path string?) (name string?) (line integer?)))

    ;; RUST-MOD-R001: mod.rs / lib.rs must remain pure module facades.
    ((style_finding "RUST-MOD-R001" "propose-ast-patch" id path name line)
     <-- (file f "rust" path)
         (entity id f "rust" "function" _ name line)
         (attr id "in_mod_rs" "true"))

    ;; RUST-AGENT-API-TYPE-012: public semantic identifiers require domain newtypes.
    ((style_finding "RUST-AGENT-API-TYPE-012" "propose-ast-patch" id path name line)
     <-- (file f "rust" path)
         (entity id f "rust" "function" _ name line)
         (attr id "visibility" "pub")
         (attr id "primitive_semantic_param" "true"))

    ;; RUST-AGENT-ITER-IMPL-026: manual Vec::new() push loop without early exit -> iterator chain.
    ((style_finding "RUST-AGENT-ITER-IMPL-026" "auto-elevate-local" id path name line)
     <-- (file f "rust" path)
         (entity id f "rust" _ _ name line)
         (attr id "mut_vec_push_loop" "true")
         (not (attr id "early_exit" "true")))

    ;; RUST-AGENT-CFG-PUBLIC-015: redundant `match opt { Some(v) => Some(E), None => None }` -> `.map`.
    ((style_finding "RUST-AGENT-CFG-PUBLIC-015" "auto-elevate-local" id path name line)
     <-- (file f "rust" path)
         (entity id f "rust" _ _ name line)
         (attr id "option_map_match" "true"))

    ;; RUST-AGENT-DATA-MEMBERSHIP-029: loop-local linear membership scan -> precomputed Set/Map.
    ((style_finding "RUST-AGENT-DATA-MEMBERSHIP-029" "pre-gen-contract" id path name line)
     <-- (file f "rust" path)
         (entity id f "rust" _ _ name line)
         (attr id "loop_linear_scan" "true"))))

(defstyle-pack RustCodingStyleFragment.
  (languages rust)
  (rules
    ("RUST-MOD-R001" "rust" "propose-ast-patch"
     "file_basename = \"mod.rs\" ⇒ items ⊆ {mod_decl, use_reexport}"
     #f rust_mod_rs_impl_item)
    ("RUST-AGENT-API-TYPE-012" "rust" "propose-ast-patch"
     "∀p ∈ pub_params(fn): semantic_id(p) ⇒ ¬primitive_str(type(p))"
     #f rust_primitive_semantic_param)
    ("RUST-AGENT-ITER-IMPL-026" "rust" "auto-elevate-local"
     "∀L ∈ for_loops: mut_vec_push_only(L) ∧ ¬early_exit(L) ⇒ elevatable_iter(L)"
     "(let mut v = Vec::new(); for x in xs { v.push(F(x)); }) ⇝ xs.into_iter().map(F).collect()"
     rust_elevatable_loop)
    ("RUST-AGENT-CFG-PUBLIC-015" "rust" "auto-elevate-local"
     "∀m ∈ match_exprs: option_map_shape(m) ⇒ elevatable_option_map(m)"
     "match opt { Some(v) => Some(E), None => None } ⇝ opt.map(|v| E)"
     rust_elevatable_option_map)
    ("RUST-AGENT-DATA-MEMBERSHIP-029" "rust" "pre-gen-contract"
     "¬∃(L ∈ loops, c ∈ calls(L)): linear_scan(c, slice)"
     "loop_any(xs, k) ⇝ HashSet::from_iter(xs).contains(k)"
     rust_loop_linear_scan))
  (fragments +rust-style-ascent-fragment+))

(def +scheme-poo-style-ascent-fragment+
  (ascent-fragment
    (relation file ((id integer?) (lang string?) (path string?)))
    (relation entity
      ((id string?) (file_id integer?) (lang string?)
       (kind string?) (language_kind string?)
       (name string?) (line integer?)))
    (relation attr ((id string?) (key string?) (val string?)))
    (relation style_finding
      ((rule_id string?) (class string?) (entity_id string?)
       (path string?) (name string?) (line integer?)))

    ;; SCM-POO-001: POO prototype definitions belong in objects.ss, never funcs.ss.
    ((style_finding "SCM-POO-001" "propose-ast-patch" id path name line)
     <-- (file f "scheme" path)
         (entity id f "scheme" "item" "poo_def" name line)
         (attr id "in_funcs_ss" "true"))

    ;; SCM-POO-002: Zero ad-hoc regex extraction over structured AST domains.
    ((style_finding "SCM-POO-002" "propose-ast-patch" id path name line)
     <-- (file f "scheme" path)
         (entity id f "scheme" _ _ name line)
         (attr id "uses_regex_over_ast" "true"))

    ;; SCM-POO-003: Redundant let wrapper inside POO prototype -> lexical `(slot := expr)`.
    ((style_finding "SCM-POO-003" "auto-elevate-local" id path name line)
     <-- (file f "scheme" path)
         (entity id f "scheme" "item" "poo_def" name line)
         (attr id "redundant_let_slot" "true"))))

(defstyle-pack SchemePooCodingStyleFragment.
  (languages scheme)
  (rules
    ("SCM-POO-001" "scheme" "propose-ast-patch"
     "∀d ∈ poo_defs: file_basename(d) = \"objects.ss\" ∧ ¬in_funcs_ss(d)"
     #f scheme_poo_placement)
    ("SCM-POO-002" "scheme" "propose-ast-patch"
     "¬∃f ∈ ast_pipeline: uses_regex_over_ast(f)"
     "regex_scan(src) ⇝ gerbil_parser_or_provider_edb(src)"
     scheme_no_regex_ast)
    ("SCM-POO-003" "scheme" "auto-elevate-local"
     "∀s ∈ poo_slots: lexical_const_let(s) ⇒ elevatable_poo_binding(s)"
     "(let ((v E)) (.def P s: v)) ⇝ (.def P (s := E))"
     scheme_poo_slot_binding))
  (fragments +scheme-poo-style-ascent-fragment+))

(def CodingStyleRegistry.
  (.mix SchemePooCodingStyleFragment.
        RustCodingStyleFragment.
        BaseCodingStyleFragment.))

(def CodingStyleRegistryValue
  (seal-object! CodingStyleRegistry.))

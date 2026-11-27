;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO prototypes and closed grammar constants for Marlin's modular `repair`
;;; module (`scheme/repair`).
(import (only-in :clan/poo/object .def seal-object!))
(export +max-dsl-bytes+
        +max-dsl-nodes+
        +max-dsl-depth+
        +max-disjunction-rules+
        +max-close-parens-repair+
        +max-trim-parens-repair+
        +high-level-l1-heads+
        +high-level-l2-heads+
        +admitted-top-heads+
        +closed-column-predicates+
        +numeric-guard-ops+
        +ascent-body-builtins+
        +closed-guard-ops+
        +guard-expr-ops+
        +comparison-atom-ops+
        +general-ontology-5col-rels+
        +path-prefix-ops+
        +path-contains-ops+
        RepairPolicy.
        DefaultRepairPolicyValue)

(def +max-dsl-bytes+ 65536)
(def +max-dsl-nodes+ 4096)
(def +max-dsl-depth+ 64)
(def +max-disjunction-rules+ 24)
(def +max-close-parens-repair+ 6)
(def +max-trim-parens-repair+ 4)

(def +high-level-l1-heads+
  '(find-collisions find-callers find-impls find-symbols find-impact trace-path))
(def +high-level-l2-heads+
  '(solve-placement solve-resolve))
(def +admitted-top-heads+
  '(ascent ascent-fragment relational-choice-program search-choice-program search-selector
    find-collisions find-callers find-impls find-symbols find-impact trace-path solve-placement solve-resolve))

(def +closed-column-predicates+ '(symbol? integer? exact-integer? boolean? string?))
(def +numeric-guard-ops+ '(< <= > >= = + - * /))
(def +ascent-body-builtins+ '(not if for agg let))
(def +closed-guard-ops+ '(< <= > >= = equal? + - * / not and or even? odd? zero? positive? negative?))
(def +guard-expr-ops+ '(< <= > >= = equal? string=? != not= <> + - * / not and or even? odd? zero? positive? negative?))
(def +comparison-atom-ops+ '(= == equal? string=? != not= <> < <= > >=))
(def +general-ontology-5col-rels+
  '(callable type_decl module_decl import_decl binding_decl macro_decl comment_node test_decl))
(def +path-prefix-ops+
  '(path_has_prefix path-has-prefix path-prefix? str-prefix string-prefix? starts-with?))
(def +path-contains-ops+
  '(path_contains path-contains path-contains? str-contains string-contains string-contains? contains?))

(.def RepairPolicy.
  identity: 'marlin/repair-policy
  max-bytes: +max-dsl-bytes+
  max-nodes: +max-dsl-nodes+
  max-depth: +max-dsl-depth+
  max-disjunction-rules: +max-disjunction-rules+
  max-close-parens: +max-close-parens-repair+
  max-trim-parens: +max-trim-parens-repair+)

(def DefaultRepairPolicyValue
  (seal-object! RepairPolicy.))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Hygienic macro surface for Marlin's Frontier 1 Machine-Native Relational
;;; Coding Style Engine (`scheme/ast-style`, RFC 40.185 / ASP RFC 10.35).
;;;
;;; One single `defstyle-pack` declaration compiles BOTH:
;;;   1. A REAL executable `:gerbil-ascent` `ascent-fragment` operating over
;;;      ASP's `semantic-open-edb.v1` + AST attribute facets (`file`, `entity`,
;;;      `edge`, `attr`, `style_finding`); and
;;;   2. Machine-native symbolic rule metadata (`∀`, `¬∃`, `LHS ⇝ RHS`,
;;;      elevation class) projected directly into compact `Pi_LLM` contracts.
(import (only-in :clan/poo/object .def)
        (only-in :gerbil-ascent/program/syntax ascent-fragment))
(export (import: :gerbil-ascent/program/syntax)
        defstyle-pack)

(defrules defstyle-pack (languages rules fragments)
  ((_ PackName
      (languages lang-sym ...)
      (rules (rule-id lang-str class-str formula-str rewrite-opt rel-name) ...)
      (fragments frag-expr ...))
   (.def PackName
     (languages => append '(lang-sym ...))
     (style-rules => append
       (list (list rule-id lang-str class-str formula-str rewrite-opt 'rel-name) ...))
     (ascent-fragments => append (list frag-expr ...)))))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure Functional Orchestrator for Marlin's Modular Repair Pipeline
;;; (`scheme/repair/funcs.ss`).
;;;
;;; Composes five focused sub-modules with zero circular dependencies:
;;;   - `reader.ss`:      Stage A bounded fence/paren/non-eval S-exp reading
;;;   - `desugar.ss`:     Declarative L1/L2 surface-form lowering
;;;   - `ascent.ss`:      Closed `:gerbil-ascent` AST repair & validation
;;;   - `choice.ss`:      Dusa POPL 2025 finite-choice repair & canonical formatting
;;;   - `diagnostics.ss`: POO-registry schemas, similarity ranking & diagnostics
(import "./objects.ss"
        "./reader.ss"
        "./diagnostics.ss"
        "./desugar.ss"
        "./ascent.ss"
        "./choice.ss")
(export repair-and-read-ascent-query
        repair-ascent-ast
        suggest-similar-relations
        format-canonical-search-form)

(def +wrapped-tool-heads+
  '(structural_query structural-query constraint_solve constraint-solve))

;;; Unwrap accidental tool-name wrapper calls such as
;;; `(structural_query "(find-symbols ...)")` or `(constraint_solve "(solve-placement ...)")`
;;; while recording an explicit `unwrap-tool-wrapper` repair receipt.
(def (unwrap-tool-wrapper-form form tags)
  (if (and (pair? form)
           (list? form)
           (= (length form) 2)
           (memq (car form) +wrapped-tool-heads+))
    (let ((inner (cadr form)))
      (cond
        ((string? inner)
         (let-values (((parsed inner-tags) (read-bounded-search-sexp inner)))
           (values parsed (add-repair-tag (append tags inner-tags) 'unwrap-tool-wrapper))))
        ((pair? inner)
         (values inner (add-repair-tag tags 'unwrap-tool-wrapper)))
        (else (values form tags))))
    (values form tags)))

;;; Pure AST-to-AST safe repair pass over an already-read Scheme datum.
;;; Returns `(values repaired-form repair-tags)`.
(def (repair-ascent-ast form registry (initial-tags '()))
  (let*-values (((unwrapped t-wrap) (unwrap-tool-wrapper-form form initial-tags))
                ((desugared) (desugar-high-level-search-form unwrapped))
                ((f0 t0)
                 (if (rule-clause? desugared)
                   (values (list 'ascent-fragment desugared)
                           (add-repair-tag t-wrap 'wrap-bare-rule))
                   (values desugared t-wrap))))
    (unless (and (pair? f0)
                 (list? f0)
                 (memq (car f0) +admitted-top-heads+))
      (raise-repair-diagnostic
        'invalid_top_form
        "expected one native :gerbil-ascent, Dusa choice, or declarative L1/L2 search form"
        "(find-impact ...) | (find-collisions ...) | (find-callers ...) | (find-impls ...) | (find-symbols ...) | (ascent-fragment ...) | (solve-placement ...) | (solve-resolve ...) | (search-choice-program ...)"
        "(find-collisions kind: \"struct\" path-prefix: \"crates/\" lang: \"rust\" exclude-tests?: #t limit: 40)"))
    (case (car f0)
      ((ascent ascent-fragment)
       (let-values (((repaired-clauses t1)
                     (repair-ascent-fragment-clauses (cdr f0) registry t0)))
         (values (cons (car f0) repaired-clauses) t1)))
      ((relational-choice-program search-choice-program)
       (let-values (((repaired-clauses t1)
                     (repair-choice-clauses (cdr f0) registry t0)))
         (values (cons (car f0) repaired-clauses) t1)))
      ((search-selector)
       (values f0 t0))
      (else (error "unsupported top-level search form" (car f0))))))

;;; Safe bounded pre-tool-call read + AST repair pipeline.
;;; Returns `(values repaired-form repair-tags)`.
(def (repair-and-read-ascent-query text registry)
  (let-values (((form lexical-tags) (read-bounded-search-sexp text)))
    (repair-ascent-ast form registry lexical-tags)))

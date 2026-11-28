;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure functional operations for Marlin's Frontier 1 Machine-Native Relational
;;; Coding Style Engine (`scheme/ast-style`, RFC 40.185 / ASP RFC 10.35).
(import (only-in :clan/poo/object .get .ref)
        (only-in :gerbil-ascent/program/api
                 gerbil-ascent-program
                 gerbil-ascent-open-session
                 gerbil-ascent-session-replace-source!
                 gerbil-ascent-session-run)
        "./objects.ss")
(export evaluate-coding-style-over-edb
        project-symbolic-style-contract
        compute-style-cost-tuple
        style-cost<?
        partition-style-findings
        ast-style-eval-edb-vector
        ast-style-project-contract-vector
        ast-style-cost-less-vector)

(def +style-edb-relations+ '(file entity edge attr))

(def (edb-rows edb key)
  (let ((entry (assq key edb)))
    (if entry (cdr entry) '())))

(def (compose-style-program fragments)
  (let ((relations (make-hash-table-eq))
        (order '())
        (rules (apply append (map (lambda (p) (.ref p 'rules)) fragments))))
    (for-each
      (lambda (part)
        (for-each
          (lambda (rel)
            (let ((name (.ref rel 'name)))
              (unless (hash-get relations name)
                (hash-put! relations name rel)
                (set! order (cons name order)))))
          (.ref part 'relations)))
      fragments)
    (let ((ordered (map (lambda (n) (hash-ref relations n)) (reverse order))))
      (gerbil-ascent-program ordered rules 100000 100000 100000 +style-edb-relations+))))

;;; Evaluate the composed `:gerbil-ascent` Coding Style theory over an Open EDB
;;; alist `((file . ...) (entity . ...) (edge . ...) (attr . ...))` and return
;;; the derived `style_finding` rows `(rule_id class entity_id path name line)`.
(def (evaluate-coding-style-over-edb edb (registry CodingStyleRegistryValue))
  (let* ((program (compose-style-program (.get registry ascent-fragments)))
         (session (gerbil-ascent-open-session program)))
    (gerbil-ascent-session-run session)
    (for-each
      (lambda (rel-name)
        (gerbil-ascent-session-replace-source! session rel-name (edb-rows edb rel-name)))
      +style-edb-relations+)
    (let ((result (gerbil-ascent-session-run session)))
      ((.ref result 'rows-of) 'style_finding))))

;;; Partition derived `style_finding` rows by elevation safety class:
;;; `#(auto-elevate-rows propose-patch-rows pre-gen-rows)`.
(def (partition-style-findings rows)
  (let ((auto (filter (lambda (r) (string=? (cadr r) "auto-elevate-local")) rows))
        (propose (filter (lambda (r) (string=? (cadr r) "propose-ast-patch")) rows))
        (pregen (filter (lambda (r) (string=? (cadr r) "pre-gen-contract")) rows)))
    (vector auto propose pregen)))

;;; Compute lexicographic style cost tuple `(hard_violations loop_scans auto_debt)`
;;; directly from derived `:gerbil-ascent` `style_finding` rows.
(def (compute-style-cost-tuple rows)
  (let ((parts (partition-style-findings rows)))
    (list (length (vector-ref parts 1))
          (length (vector-ref parts 2))
          (length (vector-ref parts 0)))))

;;; Lexicographic strict order `a < b` on style cost tuples.
(def (style-cost<? a b)
  (cond
    ((or (null? a) (null? b)) #f)
    ((< (car a) (car b)) #t)
    ((> (car a) (car b)) #f)
    (else (style-cost<? (cdr a) (cdr b)))))

;;; Project compact machine-native symbolic Coding Style contract `Pi_LLM(T, target)`
;;; directly from registered `:gerbil-ascent` rules and live in-scope domain types.
(def (project-symbolic-style-contract lang-str target-locator in-scope-types
                                      (registry CodingStyleRegistryValue))
  (let ((rules (filter (lambda (r) (string=? (cadr r) lang-str))
                       (.get registry style-rules))))
    (call-with-output-string
      (lambda (port)
        (display "TARGET: " port)
        (display target-locator port)
        (display " (lang=" port)
        (display lang-str port)
        (display ")\n" port)
        (when (pair? in-scope-types)
          (display "IN_SCOPE_TYPES: " port)
          (display (string-join in-scope-types ", ") port)
          (newline port))
        (display "RELATIONAL_INVARIANTS (:gerbil-ascent):\n" port)
        (for-each
          (lambda (r)
            (display "  [" port)
            (display (car r) port)
            (display "] " port)
            (display (list-ref r 3) port)
            (let ((rw (list-ref r 4)))
              (when (string? rw)
                (display "  |  " port)
                (display rw port)))
            (newline port))
          rules)
        (display "COST_LATTICE: minimize (hard_violations, loop_linear_scans, auto_elevate_debt)\n"
                 port)))))

(def (deep-list->vector val)
  (cond
    ((vector? val)
     (list->vector (map deep-list->vector (vector->list val))))
    ((list? val)
     (list->vector (map deep-list->vector val)))
    (else val)))

(def (vec->rows v)
  (map vector->list (vector->list v)))

;;; Pure rooted-vector adapter for `#(files entities edges attrs)` -> `#(findings-vec cost-tuple-vec)`.
(def (ast-style-eval-edb-vector input (registry CodingStyleRegistryValue))
  (unless (and (vector? input) (= (vector-length input) 4))
    (error "ast-style-eval-edb-vector requires #(files entities edges attrs)"))
  (let* ((edb (list (cons 'file (vec->rows (vector-ref input 0)))
                    (cons 'entity (vec->rows (vector-ref input 1)))
                    (cons 'edge (vec->rows (vector-ref input 2)))
                    (cons 'attr (vec->rows (vector-ref input 3)))))
         (findings (evaluate-coding-style-over-edb edb registry))
         (cost (compute-style-cost-tuple findings)))
    (vector (deep-list->vector findings)
            (list->vector cost))))

;;; Pure rooted-vector adapter for `#(lang target-locator in-scope-types-vec)` -> `Pi_LLM` string.
(def (ast-style-project-contract-vector input (registry CodingStyleRegistryValue))
  (unless (and (vector? input) (= (vector-length input) 3))
    (error "ast-style-project-contract-vector requires #(lang target-locator in-scope-types-vec)"))
  (let ((lang (vector-ref input 0))
        (target (vector-ref input 1))
        (types (let ((raw (vector-ref input 2)))
                 (if (vector? raw) (vector->list raw) raw))))
    (project-symbolic-style-contract lang target types registry)))

;;; Pure rooted-vector adapter for `#(cost-a-vec cost-b-vec)` -> boolean.
(def (ast-style-cost-less-vector input)
  (unless (and (vector? input) (= (vector-length input) 2))
    (error "ast-style-cost-less-vector requires #(cost-a-vec cost-b-vec)"))
  (style-cost<? (vector->list (vector-ref input 0))
                (vector->list (vector-ref input 1))))

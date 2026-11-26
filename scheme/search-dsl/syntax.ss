;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Hygienic Gerbil Scheme metaprogramming (`defsyntax` / `defrules`) built
;;; directly on the upstream `:gerbil-ascent` library and `:clan/poo/object`.
;;;
;;; Zero pseudo S-expressions, zero regexes, zero `string-append`, zero `eval-syntax`:
;;;   - Re-exports real upstream `:gerbil-ascent` macros (`ascent`,
;;;     `ascent-fragment`, `relational-program`, `relational-fragment`,
;;;     `relational-choice-program`, `relational-choice-solve`).
;;;   - `defsearch-language` (`defrules`): expands ONE declarative language
;;;     specification purely over ASP's universal `semantic-open-edb.v1`
;;;     `(language, kind, language_kind)` protocol into a POO prototype carrying
;;;     a REAL compiled `:gerbil-ascent` `ascent-fragment` (publishing real typed
;;;     Datalog relations such as `rust_fn`, `rust_impl`, `rust_macro`,
;;;     `scheme_poo_def`, `sysml_requirement`) plus live metadata.
;;;   - `defsearch-tool-spec` (`defsyntax`): derives `WorkspaceSearchToolSpecValue`
;;;     by introspecting the real `:gerbil-ascent` forms and POO registry.
(import (only-in :clan/poo/object .def .get .o .ref seal-object!)
        (only-in :gerbil-ascent/program/syntax ascent ascent-fragment)
        (only-in :gerbil-ascent/program/api
                 relational-program
                 relational-fragment
                 relational-choice-program
                 relational-choice-solve))
(export (import: :gerbil-ascent/program/syntax)
        relational-program
        relational-fragment
        relational-choice-program
        relational-choice-solve
        defsearch-language
        defsearch-tool-spec
        collect-unique-relations
        render-search-spec-summary
        render-relation-signature)

;;; Deduplicate compiled `:gerbil-ascent` relation objects across POO fragments
;;; preserving declaration order.
(def (collect-unique-relations fragments)
  (let ((seen (make-hash-table-eq))
        (ordered '()))
    (for-each
      (lambda (frag)
        (for-each
          (lambda (rel)
            (let ((name (.ref rel 'name)))
              (unless (hash-get seen name)
                (hash-put! seen name #t)
                (set! ordered (cons rel ordered)))))
          (.ref frag 'relations)))
      fragments)
    (reverse ordered)))

(def (render-relation-signature rel)
  (let* ((name (.ref rel 'name))
         (arity (.ref rel 'arity))
         (cols (let loop ((i 0) (acc '()))
                 (if (= i arity)
                   (reverse acc)
                   (loop (+ i 1)
                         (cons (string->symbol
                                 (call-with-output-string
                                   (lambda (p) (display "c" p) (display i p))))
                               acc))))))
    (call-with-output-string
      (lambda (port) (write (cons name cols) port)))))

(def (render-search-spec-summary form-heads edb-rels lang-names)
  (call-with-output-string
    (lambda (port)
      (display "Native :gerbil-ascent search over ASP Open EDB. Forms: " port)
      (display (string-join
                 (map (lambda (h)
                        (call-with-output-string
                          (lambda (p) (write (list h '...) p))))
                      form-heads)
                 ", ")
               port)
      (display ". EDB: " port)
      (display (string-join (map symbol->string edb-rels) ",") port)
      (display ". Langs: " port)
      (display (string-join lang-names ",") port)
      (display "." port))))

;;; Declare a composable POO language fragment via `defrules` purely over ASP's
;;; universal `semantic-open-edb.v1` protocol (ZERO regexes!).
;;; Each `(rel-name lang-str kind-str lang-kind-pat doc-str)` entry binds a real
;;; `:gerbil-ascent` relation over `(entity id f lang-str kind-str lang-kind-pat name line)`
;;; where `lang-kind-pat` is either `_` (any native AST node kind) or an exact
;;; native parser node kind string such as `"impl_item"` or `"poo_def"`.
(defrules defsearch-language (languages relations)
  ((_ FragmentName
      (languages lang-sym ...)
      (relations (rel-name lang-str kind-str lang-kind-pat doc-str) ...))
   (.def FragmentName
     (languages => append '(lang-sym ...))
     (ascent-fragments => append
       (list
         (ascent-fragment
           (relation entity
             ((id string?) (file_id integer?) (lang string?)
              (kind string?) (language_kind string?)
              (name string?) (line integer?)))
           (relation rel-name ((id string?) (file_id integer?) (name string?) (line integer?))) ...
           ((rel-name id f name line)
            <-- (entity id f lang-str kind-str lang-kind-pat name line)) ...)))
     (relation-meta => append
       (list (list 'rel-name lang-str kind-str 'lang-kind-pat doc-str) ...)))))

;;; Compile-time `defsyntax` projecting `WorkspaceSearchToolSpecValue` from
;;; real `:gerbil-ascent` forms, Open EDB relations, and the POO registry.
(defsyntax (defsearch-tool-spec stx)
  (syntax-case stx (forms edb registry)
    ((_ SpecName
        (forms form-head ...)
        (edb edb-rel ...)
        (registry reg-expr))
     (identifier? #'SpecName)
     #'(def SpecName
         (let* ((reg reg-expr)
                (form-heads '(form-head ...))
                (edb-rels '(edb-rel ...))
                (lang-names (map symbol->string (.get reg languages)))
                (relations (collect-unique-relations (.get reg ascent-fragments)))
                (relation-names (map (lambda (r) (.ref r 'name)) relations))
                (summary-text (render-search-spec-summary form-heads edb-rels lang-names)))
           (seal-object!
             (.o forms: form-heads
                 edb-relations: edb-rels
                 derived-relations: relation-names
                 languages: lang-names
                 summary: summary-text)))))
    (_ (raise-syntax-error #f "invalid defsearch-tool-spec form" stx))))

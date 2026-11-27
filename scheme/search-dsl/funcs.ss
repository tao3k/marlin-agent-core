;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure Functional Public Facade for Marlin's Unified Search DSL
;;; (`scheme/search-dsl/funcs.ss`).
;;;
;;; Composes five focused sub-modules with zero circular dependencies:
;;;   - `prefilter.ss`:     Zero-regex Open EDB prefilter hints & active EDB cone
;;;   - `ascent-eval.ss`:   Closed `:gerbil-ascent` pruning, path/name EDBs & diversity sampling
;;;   - `choice-inputs.ss`: Dusa term lowering, closed-forbid propagation & Scenario B inputs
;;;   - `choice-eval.ss`:   Dusa POPL 2025 finite-choice evaluation & bounded slicing
;;;   - `conformance.ss`:   Helm TAB completions, ASP conformance packs & dynamic registry extension
(import (only-in :marlin-agent-core/repair/funcs repair-and-read-ascent-query)
        "./grammar.ss"
        "./objects.ss"
        "./prefilter.ss"
        "./ascent-eval.ss"
        "./choice-eval.ss"
        "./conformance.ss")
(export read-ascent-search-form
        repair-ascent-search-form
        compile-search-dsl-prefilter
        compile-closed-ascent-session
        run-compiled-ascent-session
        prune-ascent-parts-for-query
        select-diverse-rows-up-to
        evaluate-search-dsl-over-edb
        evaluate-choice-program-over-edb
        search-dsl-completions
        json-schema->completion-entries
        json-schema-file->completion-entries
        json-schema-string->completion-entries
        search-dsl-tool-description
        search-dsl-tool-summary
        parse-search-dsl-selector
        project-language-conformance-pack
        project-all-conformance-packs
        project-language-json-schema
        extend-search-language-registry)

(def (read-ascent-search-form text (registry SearchLanguageRegistryValue))
  (let-values (((form _repairs) (repair-and-read-ascent-query text registry)))
    form))

(def (repair-ascent-search-form text (registry SearchLanguageRegistryValue))
  (let-values (((form repairs) (repair-and-read-ascent-query text registry)))
    (vector form repairs)))

(def (parse-search-dsl-selector source)
  (search-selector-artifact-summary source))

(def (evaluate-search-dsl-over-edb text edb (registry SearchLanguageRegistryValue))
  (let-values (((form repairs) (repair-and-read-ascent-query text registry)))
    (case (car form)
      ((ascent ascent-fragment)
       (evaluate-closed-ascent-over-edb form edb registry repairs))
      ((relational-choice-program search-choice-program)
       (evaluate-closed-choice-over-edb form edb registry repairs))
      ((search-selector)
       (unless (and (= (length form) 2) (string? (cadr form)))
         (error "(search-selector ...) requires one string argument" form))
       (parse-search-dsl-selector (cadr form)))
      (else (error "unsupported search form" (car form))))))

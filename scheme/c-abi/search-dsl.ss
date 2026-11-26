;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted Scheme C-ABI entrypoints for Marlin's unified `:gerbil-ascent` +
;;; `:gerbil-parser` + POO Search DSL. Zero JSON across the native ABI boundary.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/search-dsl/config SearchDslScope)
        (only-in :marlin-agent-core/search-dsl/objects SearchLanguageRegistryValue)
        (only-in :marlin-agent-core/search-dsl/funcs
                 compile-search-dsl-prefilter
                 evaluate-search-dsl-over-edb
                 search-dsl-completions
                 extend-search-language-registry))
(export marlin-search-dsl-compile-prefilter
        marlin-search-dsl-eval-edb
        marlin-search-dsl-extract-facts
        marlin-search-dsl-completions
        marlin-search-dsl-tool-description
        marlin-search-dsl-register-provider-descriptor)

;;; Boundary-owned active registry cell extended dynamically when ASP forwards
;;; new or updated provider descriptors over HTTP (pure transitions in `funcs.ss`).
(def *active-search-registry* SearchLanguageRegistryValue)

(def (marlin-search-dsl-compile-prefilter input)
  (let ((query (if (string? input)
                 input
                 "(ascent-fragment (relation answer ((path string?))) ((answer path) <-- (file _ _ path)))")))
    (compile-search-dsl-prefilter query *active-search-registry*)))

(def (deep-list->vector val)
  (cond
    ((vector? val)
     (list->vector (map deep-list->vector (vector->list val))))
    ((list? val)
     (list->vector (map deep-list->vector val)))
    (else val)))

;;; `input` is a rooted 2-element vector `#(query-string edb-vector)` where
;;; `edb-vector` is `#(files-vec entities-vec edges-vec attrs-vec)`.
(def (marlin-search-dsl-eval-edb input)
  (unless (and (vector? input) (= (vector-length input) 2))
    (error "marlin-search-dsl-eval-edb requires #(query edb-vector)"))
  (let* ((query (vector-ref input 0))
         (edb-vec (vector-ref input 1))
         (vec->rows (lambda (v) (map vector->list (vector->list v))))
         (edb (list (cons 'file (vec->rows (vector-ref edb-vec 0)))
                    (cons 'entity (vec->rows (vector-ref edb-vec 1)))
                    (cons 'edge (vec->rows (vector-ref edb-vec 2)))
                    (cons 'attr (vec->rows (vector-ref edb-vec 3))))))
    (deep-list->vector (evaluate-search-dsl-over-edb query edb *active-search-registry*))))

(def (marlin-search-dsl-extract-facts input)
  ((.get SearchDslScope extract-facts) input))

(def (marlin-search-dsl-completions input)
  (let ((prefix (if (string? input) input "")))
    (search-dsl-completions prefix *active-search-registry*)))

(def (marlin-search-dsl-tool-description _input)
  ((.get SearchDslScope tool-description)))

;;; Dynamically register an ASP HTTP-forwarded language provider descriptor:
;;;   #(lang-str provider-lang-str extensions-vec kind-map-vec special-specs-vec)
;;; Returns `#(registered lang-str total-relation-count)` with zero JSON.
(def (marlin-search-dsl-register-provider-descriptor input)
  (let ((next (extend-search-language-registry *active-search-registry* input)))
    (set! *active-search-registry* next)
    (vector 'registered
            (vector-ref input 0)
            (length (.get next relation-meta)))))

(begin-foreign
  (c-define (search-dsl-compile-prefilter root) (int64) int64
    "marlin_search_dsl_compile_prefilter" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/search-dsl#marlin-search-dsl-compile-prefilter))
  (c-define (search-dsl-eval-edb root) (int64) int64
    "marlin_search_dsl_eval_edb" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/search-dsl#marlin-search-dsl-eval-edb))
  (c-define (search-dsl-extract-facts root) (int64) int64
    "marlin_search_dsl_extract_facts" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/search-dsl#marlin-search-dsl-extract-facts))
  (c-define (search-dsl-completions root) (int64) int64
    "marlin_search_dsl_completions" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/search-dsl#marlin-search-dsl-completions))
  (c-define (search-dsl-tool-description root) (int64) int64
    "marlin_search_dsl_tool_description" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/search-dsl#marlin-search-dsl-tool-description))
  (c-define (search-dsl-register-provider-descriptor root) (int64) int64
    "marlin_search_dsl_register_provider_descriptor" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/search-dsl#marlin-search-dsl-register-provider-descriptor)))

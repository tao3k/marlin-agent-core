;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted Scheme C-ABI entrypoints for Marlin's unified `:gerbil-ascent` +
;;; `:gerbil-parser` + POO Search DSL. Zero JSON across the native ABI boundary.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :clan/poo/object .get)
        (only-in :std/error Error? Error-message Error-irritants)
        (only-in :marlin-agent-core/search-dsl/config SearchDslScope)
        (only-in :marlin-agent-core/search-dsl/objects SearchLanguageRegistryValue)
        (only-in :marlin-agent-core/search-dsl/funcs
                 repair-ascent-search-form
                 compile-search-dsl-prefilter
                 compile-closed-ascent-session
                 run-compiled-ascent-session
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
(def +max-cached-sessions+ 64)
(def +max-cached-prefilters+ 128)
(def *compiled-ascent-session-cache* (make-hash-table))
(def *compiled-prefilter-cache* (make-hash-table))

(def (reset-search-dsl-caches!)
  (set! *compiled-ascent-session-cache* (make-hash-table))
  (set! *compiled-prefilter-cache* (make-hash-table)))

;;; Strip noisy internal Gambit continuation stack traces for structured
;;; validation/repair diagnostics so LLM context receives compact, actionable
;;; error_class / expected_schema / suggested_fix guidance.
(def (exception->diagnostic-string err)
  (cond
    ((Error? err)
     (let ((msg (Error-message err))
           (irrs (Error-irritants err)))
       (call-with-output-string
         (lambda (port)
           (display msg port)
           (when (and (pair? irrs) (not (and (string? msg) (>= (string-length msg) 21)
                                             (string=? (substring msg 0 21) "DSL_REPAIR_DIAGNOSTIC"))))
             (display " (irritants: " port)
             (write irrs port)
             (display ")" port))))))
    (else
     (call-with-output-string
       (lambda (port)
         (display-exception err port))))))

(def (marlin-search-dsl-compile-prefilter input)
  (with-catch
    (lambda (err)
      (vector 'search-error (exception->diagnostic-string err)))
    (lambda ()
      (let* ((query (if (string? input)
                      input
                      "(ascent-fragment (relation answer ((path string?))) ((answer path) <-- (file _ _ path)))"))
             (cached (hash-get *compiled-prefilter-cache* query)))
        (or cached
            (let ((compiled (compile-search-dsl-prefilter query *active-search-registry*)))
              (when (>= (hash-length *compiled-prefilter-cache*) +max-cached-prefilters+)
                (set! *compiled-prefilter-cache* (make-hash-table)))
              (hash-put! *compiled-prefilter-cache* query compiled)
              compiled))))))

(def (deep-list->vector val)
  (cond
    ((vector? val)
     (list->vector (map deep-list->vector (vector->list val))))
    ((list? val)
     (list->vector (map deep-list->vector val)))
    (else val)))

(def (vec->rows v)
  (map vector->list (vector->list v)))

(def (edb-vector->selective-alist edb-vec active-edb-names)
  (let* ((needs-path-base?
           (ormap (lambda (r) (memq r '(is_test_path prod_path path_has_prefix path_contains)))
                  active-edb-names))
         (needs-name-base?
           (ormap (lambda (r) (memq r '(name_contains name_has_prefix)))
                  active-edb-names))
         (keys0 (if (and needs-path-base? (not (memq 'file active-edb-names)))
                  (cons 'file active-edb-names)
                  active-edb-names))
         (keys (if (and needs-name-base? (not (memq 'entity keys0)))
                 (cons 'entity keys0)
                 keys0)))
    (map (lambda (rel-name)
           (cons rel-name
                 (case rel-name
                   ((file) (vec->rows (vector-ref edb-vec 0)))
                   ((entity) (vec->rows (vector-ref edb-vec 1)))
                   ((edge) (vec->rows (vector-ref edb-vec 2)))
                   ((attr) (vec->rows (vector-ref edb-vec 3)))
                   ((symbol_ref)
                    (if (> (vector-length edb-vec) 4)
                      (vec->rows (vector-ref edb-vec 4))
                      '()))
                   (else '()))))
         keys)))

(def (edb-vector->full-alist edb-vec)
  (list (cons 'file (vec->rows (vector-ref edb-vec 0)))
        (cons 'entity (vec->rows (vector-ref edb-vec 1)))
        (cons 'edge (vec->rows (vector-ref edb-vec 2)))
        (cons 'attr (vec->rows (vector-ref edb-vec 3)))
        (cons 'symbol_ref
              (if (> (vector-length edb-vec) 4)
                (vec->rows (vector-ref edb-vec 4))
                '()))))

;;; `input` is a rooted 2-element vector `#(query-string edb-vector)` where
;;; `edb-vector` is `#(files-vec entities-vec edges-vec attrs-vec)`.
(def (marlin-search-dsl-eval-edb input)
  (with-catch
    (lambda (err)
      (vector 'search-error (exception->diagnostic-string err)))
    (lambda ()
      (unless (and (vector? input) (= (vector-length input) 2))
        (error "marlin-search-dsl-eval-edb requires #(query edb-vector)"))
      (let* ((query (vector-ref input 0))
             (edb-vec (vector-ref input 1))
             (cached (hash-get *compiled-ascent-session-cache* query)))
        (if cached
          (let* ((active-edb (vector-ref cached 4))
                 (edb (edb-vector->selective-alist edb-vec active-edb)))
            (deep-list->vector (run-compiled-ascent-session cached edb)))
          (let* ((repaired (repair-ascent-search-form query *active-search-registry*))
                 (form (vector-ref repaired 0))
                 (repairs (vector-ref repaired 1)))
            (if (and (pair? form) (memq (car form) '(ascent ascent-fragment)))
              (let ((compiled (compile-closed-ascent-session form *active-search-registry* repairs)))
                (when (>= (hash-length *compiled-ascent-session-cache*) +max-cached-sessions+)
                  (set! *compiled-ascent-session-cache* (make-hash-table)))
                (hash-put! *compiled-ascent-session-cache* query compiled)
                (let* ((active-edb (vector-ref compiled 4))
                       (edb (edb-vector->selective-alist edb-vec active-edb)))
                  (deep-list->vector (run-compiled-ascent-session compiled edb))))
              (deep-list->vector
                (evaluate-search-dsl-over-edb
                  query
                  (edb-vector->full-alist edb-vec)
                  *active-search-registry*)))))))))

(def (marlin-search-dsl-extract-facts input)
  ((.get SearchDslScope extract-facts) input))

(def (marlin-search-dsl-completions input)
  (let ((prefix (if (string? input) input "")))
    (search-dsl-completions prefix *active-search-registry*)))

(def (marlin-search-dsl-tool-description _input)
  ((.get SearchDslScope tool-description)))

;;; Dynamically register an ASP HTTP-forwarded language provider descriptor:
;;;   #(lang-str provider-lang-str extensions-vec kind-map-vec special-specs-vec)
;;; Returns `#(registered lang-str total-relation-count added-count)` with zero JSON.
(def (marlin-search-dsl-register-provider-descriptor input)
  (let* ((prev *active-search-registry*)
         (prev-langs (length (.get prev languages)))
         (prev-rels (length (.get prev relation-meta)))
         (next (extend-search-language-registry prev input))
         (next-langs (length (.get next languages)))
         (next-rels (length (.get next relation-meta)))
         (added (+ (max 0 (- next-langs prev-langs))
                   (max 0 (- next-rels prev-rels)))))
    (when (> added 0)
      (set! *active-search-registry* next)
      (reset-search-dsl-caches!))
    (vector 'registered
            (vector-ref input 0)
            next-rels
            added)))

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

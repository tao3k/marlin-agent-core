#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; Build script for the Marlin Gerbil runtime assets.

(import :std/make
        :std/source
        :clan/building
        (only-in :gslph/src/build-api/source-coverage
                 gslph-source-coverage)
        :gerbil/gambit
        (only-in :gerbil/compiler/base
                 __available-cores))

(def +marlin-gerbil-package-name+ "marlin-deck-runtime")
(def package-root (path-normalize (path-directory (this-source-file))))
(def source-root (path-expand "src" package-root))

(def +marlin-special-source-files+
  '("marlin/deck-runtime-native.ss"
    "marlin/agent-policy-routing-native.ss"))

(gslph-source-coverage
 roots: '("src")
 runtime-roots: '("src")
 explanation: "Marlin Gerbil runtime adapters live under gerbil/src; build.ss only declares the package source universe while gslph gxtest owns policy execution.")

;;; Boundary:
;;; - gerbil.pkg owns dependency declarations.
;;; - gslph-source-coverage owns the policy source universe.
;;; - std/make and clan/building own compilation discovery.
(def (marlin-package-source-file? module-path)
  (not (member module-path +marlin-special-source-files+)))

(def (with-source-root thunk)
  (let (previous-directory (current-directory))
    (dynamic-wind
      (lambda () (current-directory source-root))
      thunk
      (lambda () (current-directory previous-directory)))))

(def (marlin-runtime-build-spec)
  (with-source-root
   (lambda ()
     (filter marlin-package-source-file? (all-gerbil-modules)))))

(def (spec)
  (marlin-runtime-build-spec))

(%set-build-environment!
 (path-expand "build.ss" source-root)
 name: +marlin-gerbil-package-name+
 deps: '("poo-flow" "gslph")
 spec: marlin-runtime-build-spec)

(def (marlin-build-positive-integer-from-env name)
  (let* ((raw (getenv name #f))
         (configured (and raw (string->number raw))))
    (and configured
         (integer? configured)
         (> configured 0)
         configured)))

(def (marlin-build-worker-count)
  (or (marlin-build-positive-integer-from-env "GERBIL_BUILD_CORES")
      (marlin-build-positive-integer-from-env "CARGO_BUILD_JOBS")
      (marlin-build-positive-integer-from-env "NUM_JOBS")
      (max 1 (##cpu-count))))

(def (marlin-sync-build-worker-count!)
  (let (worker-count (marlin-build-worker-count))
    (set! __available-cores worker-count)
    worker-count))

(def (marlin-build-options options)
  (append options [parallelize: (marlin-sync-build-worker-count!)]))

(def (marlin-build-parse-options opts)
  (let lp ((rest opts) (options []))
    (match rest
      ([] options)
      (["--release" . rest]
       (lp rest (cons* build-release: #t options)))
      (["--optimized" . rest]
       (lp rest (cons* build-optimized: #t options)))
      (["--debug" . rest]
       (lp rest (cons* debug: #t options)))
      (else
       (error "Unexpected build option" rest)))))

(def (marlin-package-compile options)
  (apply make (marlin-runtime-build-spec)
         srcdir: source-root
         (marlin-build-options options)))

(def (marlin-package-clean)
  (apply make-clean (marlin-runtime-build-spec)
         srcdir: source-root
         (marlin-build-options [])))

(def (marlin-package-test)
  (let (test-entry (path-expand "t/all-test.ss" package-root))
    (let (previous-directory (current-directory))
      (dynamic-wind
        (lambda () (current-directory package-root))
        (lambda () (load test-entry))
        (lambda () (current-directory previous-directory))))))

(def (main . args)
  (match args
    (["meta"] (write '("spec" "compile" "clean" "test")) (newline))
    (["spec" . _]
     (pretty-print (spec)))
    (["compile" . options]
     (marlin-package-compile (marlin-build-parse-options options)))
    (["clean"] (marlin-package-clean))
    (["test"] (marlin-package-test))
    ([] (marlin-package-compile []))
    (else
     (error "Unexpected build command" args))))

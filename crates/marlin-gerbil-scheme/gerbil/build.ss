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

(def +marlin-excluded-package-source-files+
  +marlin-special-source-files+)


(gslph-source-coverage
 roots: '("src")
 runtime-roots: '("src")
 explanation: "Marlin Gerbil runtime adapters live under gerbil/src; build.ss owns the Scheme harness source coverage activation while gerbil.pkg owns package dependencies and policy.")

;;; Boundary:
;;; - gerbil.pkg owns dependency declarations.
;;; - gxpkg owns dependency installation and package context.
;;; - clan/building owns source discovery under src/.
(def (marlin-package-source-file? module-path)
  (not (member module-path +marlin-excluded-package-source-files+)))

(def (marlin-runtime-build-spec)
  (let (previous-directory (current-directory))
    (dynamic-wind
      (lambda () (current-directory source-root))
      (lambda () (filter marlin-package-source-file? (all-gerbil-modules)))
      (lambda () (current-directory previous-directory)))))

(def (marlin-package-build-spec options)
  (marlin-runtime-build-spec))

(def (spec)
  (marlin-package-build-spec []))

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

(def (marlin-build-make-options options)
  (match options
    ([key value . rest]
     (cons key
           (cons value
                 (marlin-build-make-options rest))))
    ([] [])))

(def (marlin-package-options options)
  (append (marlin-build-make-options options)
          [parallelize: (marlin-sync-build-worker-count!)]))

(def (marlin-build-message action label stage)
  (display "... marlin-gerbil ")
  (display action)
  (display " ")
  (display label)
  (display " targets=")
  (display (length stage))
  (display " parallelize=")
  (display (marlin-build-worker-count))
  (newline)
  (force-output))

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
       (error "Unexpected " rest)))))

(def (marlin-make label stage options)
  (marlin-build-message "compile" label stage)
  (apply make stage
         srcdir: source-root
         (marlin-package-options options)))

(def (marlin-make-clean label stage)
  (marlin-build-message "clean" label stage)
  (apply make-clean stage
         srcdir: source-root
         (marlin-package-options [])))

(def (marlin-package-compile options)
  (marlin-make "package"
               (marlin-package-build-spec options)
               options))

(def (marlin-package-clean)
  (marlin-make-clean "package"
                     (marlin-package-build-spec [])))

(def (main . args)
  (match args
    (["meta"] (write '("spec" "compile" "clean")) (newline))
    (["spec" . options]
     (pretty-print
      (marlin-package-build-spec
       (marlin-build-parse-options options))))
    (["compile" . options]
     (marlin-package-compile (marlin-build-parse-options options)))
    (["clean"] (marlin-package-clean))
    ([] (marlin-package-compile []))
    (else
     (error "Unexpected build command" args))))

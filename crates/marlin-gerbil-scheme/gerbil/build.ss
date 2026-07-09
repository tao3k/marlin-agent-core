#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; Build script for the Marlin Gerbil runtime assets.

(import :std/make
        :std/source
        :clan/building
        (only-in :gslph/src/build-api/source-coverage
                 gslph-source-coverage)
        (only-in :gslph/src/testing/build
                 testing-build
                 testing-build-main)
        (only-in :gerbil/tools/env
                 setup-local-pkg-env!)
        :gerbil/gambit
        (only-in :gerbil/compiler/base
                 __available-cores))

(def +marlin-gerbil-package-name+ "marlin-deck-runtime")
(def package-root (path-normalize (path-directory (this-source-file))))
(def source-root (path-expand "src" package-root))
(def marlin-build-root-configured? #f)

(def +marlin-testing-project+
  (testing-build
   name: "marlin-deck-runtime"
   root: "."
   contract-root: "."
   gxtest: [["all" "t/all-test.ss"]]
   roots: ["src" "t"]
   batch-size: 2
   max-selected-files: 4
   max-selected-sources: 8
   max-selected-outputs: 8))

(def (marlin-test-target options)
  (testing-build-main +marlin-testing-project+ options))

(def +marlin-special-source-files+
  '("marlin/_deck-runtime-native.ssi"
    "marlin/deck-runtime-native.ss"
    "marlin/_agent-policy-routing-native.ssi"
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

(def (marlin-configure-build-root!)
  (unless marlin-build-root-configured?
    (current-directory package-root)
    (setup-local-pkg-env! #t)
    (%set-build-environment!
     (path-expand "build.ss" source-root)
     name: +marlin-gerbil-package-name+
     deps: '("poo-flow" "gslph")
     spec: marlin-runtime-build-spec)
    (set! marlin-build-root-configured? #t)))

(def (marlin-ensure-build-root!)
  (marlin-configure-build-root!))

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
    (setenv "GERBIL_BUILD_CORES" (number->string worker-count))
    worker-count))

(def (marlin-compile-spec)
  (marlin-ensure-build-root!)
  (marlin-runtime-build-spec))

(def (marlin-build-request target spec options)
  [target spec options])

(def (marlin-build-request-target request)
  (list-ref request 0))

(def (marlin-build-request-spec request)
  (list-ref request 1))

(def (marlin-build-request-options request)
  (list-ref request 2))

(def (marlin-make-options options)
  (append options [parallelize: (marlin-sync-build-worker-count!)]))

(def (marlin-compile-request options)
  (marlin-build-request 'compile (marlin-compile-spec) options))

(def (marlin-clean-request)
  (marlin-build-request 'clean (marlin-compile-spec) []))

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

(def (marlin-run-compile-request request)
  (marlin-ensure-build-root!)
  (unless (eq? (marlin-build-request-target request) 'compile)
    (error "Expected compile build request" request))
  (apply make (marlin-build-request-spec request)
         srcdir: source-root
         (marlin-make-options (marlin-build-request-options request))))

(def (marlin-run-clean-request request)
  (marlin-ensure-build-root!)
  (unless (eq? (marlin-build-request-target request) 'clean)
    (error "Expected clean build request" request))
  (apply make-clean (marlin-build-request-spec request)
         srcdir: source-root
         (marlin-make-options (marlin-build-request-options request))))

(def (marlin-compile-target options)
  (marlin-run-compile-request (marlin-compile-request options)))

(def (marlin-clean-target)
  (marlin-run-clean-request (marlin-clean-request)))

(def (main . args)
  (match args
    (["meta"] (write '("spec" "compile" "clean" "test")) (newline))
    (["spec" . _]
     (pretty-print (marlin-compile-spec)))
    (["compile" . options]
     (marlin-compile-target (marlin-build-parse-options options)))
    (["clean"] (marlin-clean-target))
    (["test" . options] (marlin-test-target options))
    ([] (marlin-compile-target []))
    (else
     (error "Unexpected build command" args))))

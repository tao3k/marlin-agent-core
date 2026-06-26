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
(def +marlin-build-phase-receipt-schema+
  "marlin-gerbil.build.phase-receipt.v1")

(def package-root (path-normalize (path-directory (this-source-file))))
(def source-root (path-expand "src" package-root))

(def +marlin-special-source-files+
  '("marlin/deck-runtime-native.ss"
    "marlin/agent-policy-routing-native.ss"))

(def +marlin-excluded-package-source-files+
  +marlin-special-source-files+)

(def +marlin-build-cache-root+
  (path-expand ".cache" package-root))

(def +marlin-build-stamp-file+
  (path-expand "marlin-gerbil-build.stamp" +marlin-build-cache-root+))

(def +marlin-build-config-files+
  (list (this-source-file)
        (path-expand "gerbil.pkg" package-root)
        (path-expand "harness-policy/gerbil.ss" package-root)))


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

(def (marlin-build-source-file module-path)
  (path-expand module-path source-root))

(def (marlin-build-input-files stage)
  (append (map marlin-build-source-file stage)
          +marlin-build-config-files+))

(def (marlin-file-mtime path)
  (time->seconds
   (file-info-last-modification-time
    (file-info path))))

(def (marlin-any-input-newer-than-stamp? input-files stamp-time)
  (match input-files
    ([] #f)
    ([path . rest]
     (or (> (marlin-file-mtime path) stamp-time)
         (marlin-any-input-newer-than-stamp? rest stamp-time)))))

(def (marlin-build-cache-fresh? stage)
  (and (file-exists? +marlin-build-stamp-file+)
       (not
        (marlin-any-input-newer-than-stamp?
         (marlin-build-input-files stage)
         (marlin-file-mtime +marlin-build-stamp-file+)))))

(def (marlin-ensure-build-cache!)
  (unless (file-exists? +marlin-build-cache-root+)
    (create-directory +marlin-build-cache-root+)))

(def (marlin-build-phase-receipt event action label stage)
  `((schemaId . ,+marlin-build-phase-receipt-schema+)
    (schemaVersion . 1)
    (package . ,+marlin-gerbil-package-name+)
    (event . ,event)
    (action . ,action)
    (label . ,label)
    (targets . ,(length stage))
    (parallelize . ,(marlin-build-worker-count))
    (sourceRoot . ,source-root)
    (cacheStamp . ,+marlin-build-stamp-file+)))

(def (marlin-build-emit-phase-receipt event action label stage)
  (write (marlin-build-phase-receipt event action label stage))
  (newline)
  (force-output))

(def (marlin-write-build-stamp! stage)
  (marlin-ensure-build-cache!)
  (when (file-exists? +marlin-build-stamp-file+)
    (delete-file +marlin-build-stamp-file+))
  (call-with-output-file +marlin-build-stamp-file+
    (lambda (port)
      (write (marlin-build-phase-receipt
              "phase-complete"
              "compile"
              "package"
              stage)
             port)
      (newline port))))

(def (marlin-delete-build-stamp!)
  (when (file-exists? +marlin-build-stamp-file+)
    (delete-file +marlin-build-stamp-file+)))

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
  (marlin-build-emit-phase-receipt "phase-start" "compile" label stage)
  (apply make stage
         srcdir: source-root
         (marlin-package-options options)))

(def (marlin-make-clean label stage)
  (marlin-build-emit-phase-receipt "phase-start" "clean" label stage)
  (apply make-clean stage
         srcdir: source-root
         (marlin-package-options [])))

(def (marlin-package-compile options)
  (let (stage (marlin-package-build-spec options))
    (if (and (null? options)
             (marlin-build-cache-fresh? stage))
      (marlin-build-emit-phase-receipt "phase-skip" "compile" "package" stage)
      (begin
        (marlin-make "package" stage options)
        (when (null? options)
          (marlin-write-build-stamp! stage))))))

(def (marlin-package-clean)
  (marlin-delete-build-stamp!)
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

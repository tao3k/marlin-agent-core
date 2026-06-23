#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; Build script for the Marlin Gerbil runtime assets.

(import :std/make
        :std/source
        :clan/building
        :gerbil/gambit)

(def +marlin-gerbil-package-name+ "marlin-deck-runtime")

(def package-root (path-normalize (path-directory (this-source-file))))
(def source-root (path-expand "src" package-root))

(def +marlin-native-aot-only-modules+
  '("marlin/deck-runtime-native.ss"
    "marlin/agent-policy-routing-native.ss"))

(def (marlin-package-module? module-path)
  (not (member module-path +marlin-native-aot-only-modules+)))

;;; Boundary:
;;; - gerbil.pkg owns dependency declarations.
;;; - gxpkg owns dependency installation and package context.
;;; - clan/building owns source discovery under src/.
(def (marlin-runtime-build-spec)
  (let (previous-directory (current-directory))
    (dynamic-wind
      (lambda () (current-directory source-root))
      (lambda () (filter marlin-package-module? (all-gerbil-modules)))
      (lambda () (current-directory previous-directory)))))

(%set-build-environment!
 (path-expand "build.ss" source-root)
 name: +marlin-gerbil-package-name+
 deps: '("poo-flow")
 spec: marlin-runtime-build-spec)

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

(def (marlin-build-shell-quote value)
  (let lp ((rest (string->list value)) (out "'"))
    (match rest
      ([] (string-append out "'"))
      ([#\' . rest]
       (lp rest (string-append out "'\"'\"'")))
      ([char . rest]
       (lp rest (string-append out (string char)))))))

(def (marlin-build-gxc-program)
  (or (with-catch (lambda (_) #f)
        (lambda () (getenv "MARLIN_GERBIL_GXC")))
      "gxc"))

(def (marlin-build-gxc-native-target srcdir source-path)
  (let* ((command
          (string-append
           "cd " (marlin-build-shell-quote srcdir)
           " && GERBIL_LOADPATH=src "
           (marlin-build-shell-quote (marlin-build-gxc-program))
           " -target C -s -S -O "
           (marlin-build-shell-quote source-path)))
         (status (shell-command command)))
    (unless (zero? status)
      (error "native AOT gxc target failed" source-path status))))

(def (marlin-build-profile profile)
  (match profile
    ("deck-runtime"
     [["src/marlin/deck-runtime-native.ss"
       "src/marlin/deck-runtime-native~0.scm"
       "deck-runtime-native~0.scm"]])
    ("agent-policy-routing"
     [["src/marlin/agent-policy-routing-native.ss"
       "src/marlin/agent-policy-routing-native~0.scm"
       "agent-policy-routing-native~0.scm"]])
    (else
     (error "Unknown native AOT profile" profile))))

(def (marlin-build-stage-native-aot-unit srcdir output-root unit)
  (match unit
    ([source-path cache-relative staged-file]
     (marlin-build-gxc-native-target srcdir source-path)
     (let* ((cache-file
             (path-expand
              (string-append "~/.gerbil/lib/" +marlin-gerbil-package-name+ "/"
                             cache-relative)))
            (stage-dir (path-expand (path-expand ".gerbil/native" output-root)))
            (stage-file (path-expand staged-file stage-dir)))
       (unless (file-exists? cache-file)
         (error "native AOT compiled runtime artifact missing" cache-file))
       (create-directory* stage-dir)
       (when (file-exists? stage-file)
         (delete-file stage-file))
       (copy-file cache-file stage-file)
       (displayln stage-file)))))

(def (marlin-build-stage-native-aot srcdir profile output-root)
  (for-each
   (lambda (unit)
     (marlin-build-stage-native-aot-unit srcdir output-root unit))
   (marlin-build-profile profile)))

(def (marlin-runtime-build-main args build-spec that-file)
  (def (build options)
    (add-load-path! source-root)
    (apply make (build-spec) srcdir: source-root options))
  (def (clean)
    (make-clean (build-spec) srcdir: source-root))

  (match args
    (["meta"] (write '("spec" "compile" "clean" "stage-native-aot")) (newline))
    (["spec"] (pretty-print (build-spec)))
    (["compile" . options] (build (marlin-build-parse-options options)))
    (["clean"] (clean))
    (["stage-native-aot" profile output-root]
     (marlin-build-stage-native-aot package-root profile output-root))
    ([] (build []))
    (else
     (error "Unexpected build command" args))))

(def (main . args)
  (marlin-runtime-build-main args marlin-runtime-build-spec (this-source-file)))

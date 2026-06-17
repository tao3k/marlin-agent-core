#!/usr/bin/env gxi
;;; -*- Gerbil -*-
;;; Build script for the Marlin Gerbil runtime assets.

(import :std/make
        :gerbil/gambit)

(def +marlin-gerbil-package-name+ "marlin-deck-runtime")

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
  (def srcdir
    (path-normalize (path-directory that-file)))
  (def (build options)
    (add-load-path! (path-expand "src" srcdir))
    (apply make build-spec srcdir: srcdir options))
  (def (clean)
    (make-clean build-spec srcdir: srcdir))

  (match args
    (["meta"] (write '("spec" "compile" "clean" "stage-native-aot")) (newline))
    (["spec"] (pretty-print build-spec))
    (["compile" . options] (build (marlin-build-parse-options options)))
    (["clean"] (clean))
    (["stage-native-aot" profile output-root]
     (marlin-build-stage-native-aot srcdir profile output-root))
    ([] (build []))
    (else
     (error "Unexpected build command" args))))

(def marlin-runtime-build-spec
  '("src/marlin/protocol-types"
    "src/marlin/protocol"
    "src/marlin/request"
    "src/marlin/parser"
    "src/marlin/adapter"
    "src/marlin/deck-runtime"
    "src/marlin/deck-runtime-native-projection"
    "src/marlin/graph-loop-continuation-native-projection"
    "src/marlin/agent-policy-routing-native-projection"
    (gsc: "src/marlin/deck-runtime-native")
    (gsc: "src/marlin/agent-policy-routing-native")
    "src/marlin/deck-runtime-compiled-policy"
    "src/marlin/deck-runtime-compiled-policy-sample"
    "src/marlin/deck-runtime-strategy-context"
    "src/marlin/deck-runtime-condition-policy"
    "src/marlin/deck-runtime-dynamic-hook"
    "src/marlin/deck-runtime-matcher"
    "src/marlin/deck-runtime-policy-engine"
    "src/marlin/deck-runtime-agent-policy"
    "src/marlin/deck-runtime-extension"
    "src/marlin/deck-runtime-extension-safety"
    "src/marlin/deck-runtime-extension-catalog"
    "src/marlin/deck-runtime-extension-receipt"
    "src/marlin/deck-runtime-extension-template"
    "src/marlin/deck-runtime-script"
    "src/marlin/deck-runtime-loop-graph"
    "src/marlin/deck-runtime-user-option"
    "src/marlin/deck-runtime-user-module"
    "src/modules/kinds"
    "src/modules/core"
    "src/modules/policy-extension"
    "src/modules/policy-module"
    "src/modules/policy-object"
    "src/modules/workspace-policy"
    "src/modules/session-policy"
    "src/modules/agent-policy"
    "src/modules/hook-selection-policy"
    "src/modules/model-route-policy"
    "src/modules/continuation-profile-policy"
    "src/modules/human-review-policy"
    "src/modules/evidence-policy"
    "src/modules/failure-policy"
    "src/modules/memory-policy"
    "src/modules/catalog-projection-policy"
    "src/modules/evaluation"
    "src/modules/policy-pack"
    "src/modules/lib"
    "src/modules/prefabs/default-policy"
    "src/modules/prefabs/user-interface"
    "src/modules/prefabs/user-interface-delivery"
    "src/marlin/deck-runtime-strategy"))

(def (main . args)
  (marlin-runtime-build-main args marlin-runtime-build-spec (this-source-file)))

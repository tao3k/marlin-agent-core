#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Build the package selected by Yuanxu.

(import (only-in :clan/poo/object .call .get)
        (only-in :std/build-script defbuild-script)
        (only-in :std/make make make-clean)
        (only-in :std/misc/process run-process)
        (only-in :std/source this-source-directory)
        (only-in "./c-abi/build.ss" Build)
        (only-in "./init.ss" marlin-agent-core.init.exports))

(def package-root (this-source-directory))

(def required-lab-sentinels
  '("modules/durable/work.ssi"
    "modules/funflow/interface.ssi"
    "modules/funflow/durable-dag.ssi"
    "modules/gitops/interface.ssi"
    "modules/module-authoring/interface.ssi"
    "modules/search-engine/interface.ssi"
    "modules/query/semantic/orgize-source-interface.ssi"
    "modules/ai-agentic-context/features/durable/objects.ssi"))

(def (ensure-lab-dependencies!)
  (let* ((gpath (or (getenv "GERBIL_PATH" #f)
                    (path-expand ".gerbil" package-root)))
         (lab-lib (path-expand "lib/tao3k/agentic-ai-lab" gpath))
         (lab-root (path-expand "pkg/github.com/tao3k/agentic-ai-lab" gpath))
         (missing? (ormap (lambda (rel)
                            (not (file-exists? (path-expand rel lab-lib))))
                          required-lab-sentinels)))
    (when (and missing? (file-exists? (path-expand "init.ss" lab-root)))
      (run-process
       ["gxi" "-e"
        "(begin
           (import (only-in :clan/poo/object .call)
                   (only-in :std/make make)
                   (only-in \"./init.ss\" tao3k/agentic-ai-lab.init.exports))
           (.call tao3k/agentic-ai-lab.init.exports .compile-selection
             (quote (modules/ai-agentic-context
                     modules/durable
                     modules/funflow
                     modules/temporal-causality
                     modules/gitops
                     modules/module-authoring
                     modules/search-engine))
             (quote ())
             (lambda (targets root) (make targets srcdir: root))
             package-roots?: #t)
           (make (quote (\"modules/ai-agentic-context/features/durable/org-adapter\"
                         \"modules/ai-agentic-context/features/durable/objects\"
                         \"modules/ai-agentic-context/features/durable/interface\"
                         \"modules/query/semantic/orgize-source-interface\"))
                 srcdir: (current-directory)))"]
       directory: lab-root
       stdout-redirection: #f
       stderr-redirection: #f))))

(def (main . args)
  (match args
    ((or [] ["compile" . _])
     (ensure-lab-dependencies!)
     (make '("interface") srcdir: package-root))
    (["clean"]
     (make-clean '("interface" "ui/task-workflow") srcdir: package-root))
    (else (void)))
  (defvalues (build-main)
    (.call marlin-agent-core.init.exports .with-link (.get Build with-link)
      (let ()
        (defbuild-script (.get marlin-agent-core.init.exports std-make-spec))
        main)))
  (apply build-main args)
  (match args
    ((or [] ["compile" . _])
     (make '("ui/task-workflow") srcdir: package-root))
    (else (void))))

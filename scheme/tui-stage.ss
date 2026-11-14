#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Stage the selected compiled Gerbil modules for linking with the VM. The runtime
;;; imports come from the same Yuanxu projection as the package build.
(import (only-in :clan/poo/object .get .call)
        (only-in :tao3k/yuanxu/module-system/building/products current-build-source-directory)
        (only-in "./build.ss" marlin-build-projection)
        (only-in :gerbil-scheme-rust/scheme/program-build gerbil-rs-stage-program))
(export main)

(def (main . arguments)
  (unless (null? arguments)
    (error "tui-stage accepts no arguments"))
  (def projection (marlin-build-projection))
  (def imports
    (map (lambda (id) (string->symbol (string-append ":" (symbol->string id))))
         (.get projection compiler-identities)))
  ;; The SDK observes metadata in a child process. Its normal Gerbil load path
  ;; must include the same Yuanxu-selected compiler inputs as this process.
  (.call projection .with-compiler-inputs
    (lambda ()
      (def source-directory (current-build-source-directory))
      (unless source-directory (error "TUI compiler input directory unavailable"))
      (setenv "GERBIL_LOADPATH"
        (string-append (path-expand "lib" (gerbil-path)) ":"
                       source-directory ":" (getenv "GERBIL_LOADPATH" "")))
      (gerbil-rs-stage-program
        (path-expand "tui-entry.ss" (current-directory))
        (path-expand ".gerbil/native/tui" (current-directory))
        imports))))

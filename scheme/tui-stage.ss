#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Stage the selected compiled Gerbil modules for linking with the VM. The runtime
;;; imports come from the same Yuanxu projection as the package build.
(import (only-in :clan/poo/object .get)
        (only-in "./build.ss" marlin-build-projection)
        (only-in :gerbil-scheme-rust/scheme/program-build gerbil-rs-stage-program))
(export main)

(def (main . arguments)
  (unless (null? arguments)
    (error "tui-stage accepts no arguments"))
  (def imports
    (map (lambda (id) (string->symbol (string-append ":" (symbol->string id))))
         (.get (marlin-build-projection) compiler-identities)))
  (gerbil-rs-stage-program
    (path-expand "tui-entry.ss" (current-directory))
    (path-expand ".gerbil/native/tui" (current-directory))
    imports))

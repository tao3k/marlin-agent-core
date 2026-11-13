#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Stage the selected compiled Gerbil modules for linking with the VM. The runtime
;;; imports come from init's one std-make-spec projection, never a second list.
(import (only-in :clan/poo/object .get)
        (only-in "./init.ss" marlin-agent-core.init.exports)
        (only-in :gerbil-scheme-rust/scheme/program-build gerbil-rs-stage-program))
(export main)

(def (main . arguments)
  (unless (null? arguments)
    (error "tui-stage accepts no arguments"))
  (def specification ((.get marlin-agent-core.init.exports std-make-spec) '("init")))
  (unless (and (pair? specification)
               (andmap (lambda (target)
                         (and (pair? target) (eq? (car target) 'gxc:)
                              (pair? (cdr target)) (string? (cadr target))))
                       specification))
    (error "selected TUI build specification contains an unsupported target"))
  (def imports
    (map (lambda (target) (string->symbol (string-append ":" (cadr target))))
         specification))
  (gerbil-rs-stage-program
    (path-expand "tui-entry.ss" (current-directory))
    (path-expand ".gerbil/native/tui" (current-directory))
    imports))

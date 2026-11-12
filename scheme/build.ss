#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Build the package selected by Yuanxu.

(import (for-syntax
          (only-in :std/source this-source-file)
          (only-in :yuanxu/module-system/building/source-package register-module-build-source!)))
(begin-syntax
  (register-module-build-source! (path-directory (this-source-file)) 'marlin-agent-core))

(import (only-in :clan/poo/object .get)
        (only-in :std/build-script defbuild-script)
        (only-in :std/source this-source-file)
        (only-in :yuanxu/module-system/building/interface call-with-init-build-spec)
        (only-in :yuanxu/module-system/building/products current-build-source-directory)
        (only-in "./init.ss" marlin-agent-core.init.exports))

(export main)

(def (main . args)
  (def entry-source (this-source-file))
  (call-with-init-build-spec
    (lambda () ((.get marlin-agent-core.init.exports std-make-spec) '("init")))
    (lambda (spec arguments (package-prefix #f))
      (when package-prefix
        (error "Marlin build has an unexpected package prefix" package-prefix))
      (def (this-source-file)
        (let (source-directory (current-build-source-directory))
          (if source-directory
            (path-expand (path-strip-directory entry-source) source-directory)
            entry-source)))
      (let ()
        (defbuild-script spec)
        (apply main arguments)))
    args #f entry-source))

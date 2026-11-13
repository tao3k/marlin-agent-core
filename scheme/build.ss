#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Build the package selected by Yuanxu.

(import (for-syntax
          (only-in :std/source this-source-file)
          (only-in :tao3k/yuanxu/module-system/building/source-package register-module-build-source!)))
(begin-syntax
  (register-module-build-source! (path-directory (this-source-file)) 'marlin-agent-core))

(import (only-in :clan/poo/object .get .call)
        (only-in :std/build-script defbuild-script)
        (only-in :std/source this-source-file)
        (only-in :tao3k/yuanxu/module-system/building/build-projection yuanxu-project-source-spec)
        (only-in :tao3k/yuanxu/module-system/building/products current-build-source-directory))

(export main marlin-build-projection)

(def build-script (this-source-file))
(def source-root (path-directory build-script))

(def (marlin-build-projection)
  (yuanxu-project-source-spec (path-expand "init.ss" source-root) '("init")
    source-root build-script))

(def (main . arguments)
  (def projection (marlin-build-projection))
  (def spec (.get projection spec))
  (def (dispatch)
    (def (this-source-file)
      (path-expand "build.ss" (or (current-build-source-directory) source-root)))
    (defbuild-script spec)
    (apply main arguments))
  (match arguments
    (["clean"] (.call projection .with-clean-outputs dispatch))
    (["spec"] (dispatch))
    (["meta"] (dispatch))
    (else (.call projection .with-compiler-inputs dispatch))))

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
        (only-in :yuanxu/module-system/building/script defbuild-script)
        (only-in "./init.ss" marlin-agent-core.init.exports))

(export main)

(defbuild-script ((.get marlin-agent-core.init.exports std-make-spec) '("init")))

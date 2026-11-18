#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Build the package selected by Yuanxu.

(import (only-in :clan/poo/object .call .get)
        (only-in :std/build-script defbuild-script)
        (only-in "./c-abi/build.ss" Build)
        (only-in "./init.ss" marlin-agent-core.init.exports))

(defvalues (main)
  (.call marlin-agent-core.init.exports .with-link (.get Build with-link)
    (let ()
      (defbuild-script (.get marlin-agent-core.init.exports std-make-spec))
      main)))

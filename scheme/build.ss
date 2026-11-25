#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Build the package selected by Yuanxu.
;;;
;;; INVARIANT (DO NOT BLOAT THIS FILE):
;;; Yuanxu's `defbuild-script` + `std-make-spec` + `.with-link` owns `compile`,
;;; `clean`, and `spec` flags as well as clean `.ssi` artifact lifecycle.
;;; NEVER add `required-lab-sentinels`, ad hoc `ensure-lab-dependencies!`
;;; shellouts, or hand-maintained upstream `.ssi` build lists here.
;;; Clean and rebuild linked upstream packages via `gerbil pkg build` /
;;; `just scheme-clean` / `just scheme-build` instead.

(import (only-in :clan/poo/object .call .get)
        (only-in :std/build-script defbuild-script)
        (only-in "./c-abi/build.ss" Build)
        (only-in "./init.ss" marlin-agent-core.init.exports))

(defvalues (main)
  (.call marlin-agent-core.init.exports .with-link (.get Build with-link)
    (let ()
      (defbuild-script (.get marlin-agent-core.init.exports std-make-spec))
      main)))

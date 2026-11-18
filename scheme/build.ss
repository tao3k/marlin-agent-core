#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Build the package selected by Yuanxu.

(import (only-in :clan/poo/object .get .o)
        (only-in :std/source this-source-file)
        (only-in :std/build-script defbuild-script)
        (only-in :tao3k/yuanxu/module-system/building/build-script defmodule-build-script)
        (only-in "./init.ss" marlin-agent-core.init.exports))

(defmodule-build-script
  (defbuild-script (.get marlin-agent-core.init.exports std-make-spec))
  (.o))

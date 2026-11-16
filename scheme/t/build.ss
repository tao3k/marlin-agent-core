#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Test support only: the imported UI module supplies these C ABI symbols
;;; when the driver is loaded. Darwin must defer their lookup until then.
(import (only-in :std/build-script defbuild-script))

(defbuild-script
  (cond-expand
    (darwin '((gxc: "ui-c-abi-driver" "-ld-options" "-Wl,-undefined,dynamic_lookup")))
    (else '("ui-c-abi-driver"))))

#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Test-only package built through the SDK's standard Gerbil make interface.
(import (only-in :std/build-script defbuild-script))
(def (ui-test-spec) '("fixture" "row-action"))
(defbuild-script (ui-test-spec))

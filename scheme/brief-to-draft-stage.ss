#!/usr/bin/env gxi
;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :gerbil-scheme-rust/scheme/program-build gerbil-rs-stage-program))
(export main)

;; The ABI entry imports the init-selected publication. The SDK discovers its
;; compiler graph and runtime metadata from that entry.
(def (main)
  (gerbil-rs-stage-program
    (path-expand "c-abi/brief-to-draft.ss" (current-directory))
    (path-expand ".gerbil/native/brief-to-draft" (current-directory))))

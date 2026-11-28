;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Rooted typed values use the shared Gerbil Scheme Rust ABI.
(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :tao3k/yuanxu/prelude yuanxu-build-autoloads)
        (for-template :marlin-agent-core/context-system/types)
        (for-syntax :gerbil/expander
          (only-in "./build.ss" Build)))
(export main)

(yuanxu-build-autoloads Build workflow)

(def (main . _) (void))

(begin-foreign
  (c-define (validate-settings root) (int64) int64
    "marlin_brief_to_draft_validate_settings" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/brief-to-draft/entry#validate-settings)))

(begin-foreign
  (c-define (propose root) (int64) int64
    "marlin_brief_to_draft_propose" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/brief-to-draft/entry#propose)))

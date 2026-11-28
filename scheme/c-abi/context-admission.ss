;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Build-owned entry for the upstream rooted-object C ABI.

(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        "../context-system/batch.ss")

(begin-foreign
  (c-define (context-admit-batch root) (int64) int64
    "marlin_context_admit_batch" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/context-system/batch#marlin-context-admit-batch)))

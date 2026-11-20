;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :marlin-agent-core/envrc/bridge plan-project-envrc))

(begin-foreign
  (c-define (envrc-plan root) (int64) int64
    "marlin_envrc_plan" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/envrc/bridge#plan-project-envrc)))

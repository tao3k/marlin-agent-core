;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :marlin-agent-core/c-abi/roots marlin-rooted-call)
        (only-in :tao3k/yuanxu/prelude yuanxu-build-autoloads)
        (for-syntax (only-in "./build.ss" Build)))

(yuanxu-build-autoloads Build artifacts)

(begin-foreign
  (c-define (artifacts-exchange root) (int64) int64
    "marlin_context_artifacts_exchange" "extern"
    (marlin-agent-core/c-abi/roots#marlin-rooted-call root
      marlin-agent-core/c-abi/artifacts#exchange)))

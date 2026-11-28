;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .o))

(export BenchOperations)

(def BenchOperations
  (.o identity: 'marlin/bench
      program-entry: 'marlin-agent-core/bench/mailbox-entry
      program-runtime-imports: '()))

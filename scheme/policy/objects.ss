;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Operations provided by the Policy module.

(import (only-in :clan/poo/object .o)
        "./func.ss")

(export PolicyOperations)

(def PolicyOperations
  (.o project-routing: marlin-agent-policy-routing-project-poo-policy
      compile-loop-graph: marlin-deck-runtime-compile-loop-graph
      next-continuation: marlin-graph-loop-continuation-next-action))

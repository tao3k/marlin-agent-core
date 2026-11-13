;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(def tooling (.get marlin-agent-core.init.exports Marlin tooling))
(def declarations ((.get tooling project)))
(unless (and (eq? (.get tooling identity) 'marlin/tooling)
             (= (vector-length declarations) 1)
             (equal? (vector-ref declarations 0)
                     #(bash "Run a Bash command in the workspace after approval."
                       shell ask command 30 65536)))
  (error "selected tooling declaration changed"))
(displayln "TOOLING-OK")

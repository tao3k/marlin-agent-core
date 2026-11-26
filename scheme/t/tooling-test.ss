;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get)
        (only-in :std/string/misc string-contains)
        (only-in :marlin-agent-core/init marlin-agent-core.init.exports))

(def tooling (.get marlin-agent-core.init.exports Marlin tooling))
(def declarations ((.get tooling project)))
(def search-decl (vector-ref declarations 1))
(def search-desc (vector-ref search-decl 1))
(unless (and (eq? (.get tooling identity) 'marlin/tooling)
             (= (vector-length declarations) 2)
             (equal? (vector-ref declarations 0)
                     #(bash "Run a Bash command in the workspace after approval."
                       shell ask command 30 65536))
             (eq? (vector-ref search-decl 0) 'workspace_search)
             (string? search-desc)
             (<= (string-length search-desc) 256)
             (string-contains search-desc "(ascent-fragment ...)")
             (string-contains search-desc "(relational-choice-program ...)")
             (not (string-contains search-desc "@sym:"))
             (eq? (vector-ref search-decl 2) 'search)
             (eq? (vector-ref search-decl 3) 'ask)
             (eq? (vector-ref search-decl 4) 'query)
             (= (vector-ref search-decl 5) 10)
             (= (vector-ref search-decl 6) 65536))
  (error "selected tooling declaration changed" search-decl))
(displayln "TOOLING-OK")

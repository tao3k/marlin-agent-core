;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .o)
        (only-in :std/make pkg-config-libs)
        (only-in :tao3k/yuanxu/module-system/source/build-binding
                 module-build-feature-selected?))
(export Build)

;;; The selected native adapter Module owns its compiler inputs.
(.def Build
  defbuild-script:
  (.o spec:
      (append
        (cond-expand
          (darwin '((gxc: "roots" "-ld-options" "-Wl,-undefined,dynamic_lookup")))
          (else '("roots")))
        '("context" "deck-runtime" "ui" "tooling" "envrc" "context-admission" "worktree-context")))
  .worktree-provider:
  (lambda () (and (module-build-feature-selected? 'gitops 'worktrunk) 'worktrunk))
  autoload:
  (.o deck:
      (.o module: ':marlin-agent-core/deck-runtime/config
          exports: (.o deck-scope: 'deck-scope))
      context:
      (.o module: ':marlin-agent-core/context-system/config
          exports: (.o context-scope: 'context-scope))
      worktree-context:
      (.o module: ':marlin-agent-core/context-system/worktree
          exports: (.o prepare-context: 'prepare-worktree-context
                       deliver-context: 'deliver-worktree-context
                       revoke-context: 'revoke-worktree-context)))
  with-link:
  (lambda () ["-ld-options" (pkg-config-libs "openssl" "zlib" "sqlite3")]))

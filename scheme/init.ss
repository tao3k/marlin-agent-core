;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Select the modules owned by Marlin.

(import (only-in :tao3k/yuanxu/prelude yuanxu!))

(yuanxu!
  :load-modules "."
  mode: explicit
  mount: Marlin
  :test-dir "./t"
  (context-system +causal +durable)
  (artifacts)
  (brief-to-draft)
  (ui-components)
  (ui-elements)
  (ui-centaur-tabs)
  (ui-hydra)
  (ui-meow)
  (ui-modeline)
  (ui-transcript)
  (ui-themes)
  (ui-layout)
  (ui-workspaces)
  (ui-modes)
  (ui-emacs)
  (ui-magit)
  (search-engine)
  (repair +ast-patch)
  (search-dsl)
  (ast-style)
  (ui-helm)
  (ui)
  (ui-tui)
  (ui-mygo)
  (tooling)
  (agent-testing)
  (agent-hooks)
  (scheme-package-hooks)
  (policy)
  (gitops +worktrunk)
  (graph-engine +sqlite-turso)
  (envrc)
  (deck-runtime)
  (bench)
  (c-abi))

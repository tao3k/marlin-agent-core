;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Select the modules owned by Marlin.

(yuanxu!
  :load-modules "modules"
  package: "github.com/tao3k/agentic-ai-lab"
  mode: explicit
  rename: Lab
  (ai-agentic-context)
  :load-modules "."
  mode: explicit
  rename: Marlin
  (context +causal)
  (ui)
  (hooks)
  (policy)
  (deck-runtime)
  (c-abi))

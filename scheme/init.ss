;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Select the modules owned by Marlin.

(import (only-in :tao3k/yuanxu/prelude yuanxu!))

(yuanxu!
  :load-modules "."
  mode: explicit
  rename: Marlin
  :test-dir "./t"
  (context-system +causal)
  (brief-to-draft)
  (ui-components)
  (ui-elements)
  (ui-hydra)
  (ui-modeline)
  (ui-transcript)
  (ui-themes)
  (ui-layout)
  (ui)
  (ui-tui)
  (tooling)
  (hooks)
  (policy)
  (gitops +worktrunk)
  (envrc)
  (deck-runtime)
  (c-abi))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
(import (only-in :clan/poo/object .o))
(export UiTuiOperations)
(def UiTuiOperations
  (.o identity: 'marlin/ui-tui
      program-entry: 'marlin-agent-core/ui-tui/entry
      ;; Gerbil resolves compiled plugin exports through these interfaces.
      ;; Include their compiler metadata for demand loading from the image.
      program-runtime-imports: '(:gerbil/expander :marlin-agent-core/ui/session)))

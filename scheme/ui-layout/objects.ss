;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Scheme owns presentation choices; the native host clips them to cells.
(import (only-in :clan/poo/object .def seal-object!))
(export marlin-ui-layout-policy)

(.def UiLayoutBase.
  min-transcript-rows: 2
  max-chatbox-rows: 10
  max-command-rows: 8)
(def marlin-ui-layout-policy
  (seal-object! UiLayoutBase.))

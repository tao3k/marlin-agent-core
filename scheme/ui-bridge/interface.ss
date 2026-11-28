;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Unified Scheme Bridge Hub (`scheme/ui-bridge`): modular Scheme package
;;; dispatcher for Org Mode (`orgize`), Markdown, and future Emacs modules.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiBridgeScope)
        (only-in "./types.ss" UiBridgeScopeContract))
(export UiBridge)

(.def UiBridge
  identity: 'ui-bridge
  core: (.o scope-contract: UiBridgeScopeContract
            exports: UiBridgeScope))

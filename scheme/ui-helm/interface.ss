;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Declarative Modern Helm interface for Marlin TUI and native search-engine.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiHelmScope)
        (only-in "./types.ss" HelmScopeContract))
(export UiHelm)

(.def UiHelm
  identity: 'ui-helm
  core: (.o scope-contract: HelmScopeContract
            exports: UiHelmScope))

;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .o)
        "./config.ss" "./types.ss")
(export UiCentaurTabs)

(.def UiCentaurTabs
  identity: 'ui-centaur-tabs
  core: (.o scope-contract: UiCentaurTabsScopeContract exports: UiCentaurTabsScope))

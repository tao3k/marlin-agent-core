;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiElementsScope)
        (only-in "./types.ss" UiElementsScopeContract))
(export UiElements)

(.def UiElements
  identity: 'ui-elements
  core: (.o scope-contract: UiElementsScopeContract exports: UiElementsScope))

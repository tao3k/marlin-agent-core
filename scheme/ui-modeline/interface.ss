;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Host-neutral mode line slots. The terminal owns cells and colors.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiModelineScope)
        (only-in "./types.ss" UiModelineScopeContract))
(export UiModeline)

(.def UiModeline
  identity: 'ui-modeline
  core: (.o scope-contract: UiModelineScopeContract
            exports: UiModelineScope))

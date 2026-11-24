;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Pure-Scheme Target Profile Overlays & Slot Policy for MyGo GPU/Mobile hosts.
(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" UiMygoScope)
        (only-in "./types.ss" UiMygoScopeContract))
(export UiMygo)

(.def UiMygo
  identity: 'ui-mygo
  core: (.o scope-contract: UiMygoScopeContract
            exports: UiMygoScope))

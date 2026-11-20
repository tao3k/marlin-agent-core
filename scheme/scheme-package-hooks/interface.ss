;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .o)
        (only-in "./config.ss" SchemePackageHooksScope)
        (only-in "./types.ss" SchemePackageHooksScopeContract))
(export SchemePackageHooks)

(.def SchemePackageHooks
  identity: 'scheme-package-hooks
  core: (.o scope-contract: SchemePackageHooksScopeContract
            exports: SchemePackageHooksScope))

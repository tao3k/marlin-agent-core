;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in "./funcs.ss" make-ui-package-hooks)
        (only-in "./objects.ss" SchemePackageHooks.))
(export SchemePackageHooksScope)

(def SchemePackageHooksScope (make-ui-package-hooks SchemePackageHooks.))

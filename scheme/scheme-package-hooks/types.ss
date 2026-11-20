;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Any Fun Record Symbol))
(export SchemePackageHooksScopeContract)

(def SchemePackageHooksScopeContract
  (.cc (Record .run: [(Fun Any <- Symbol Any Any)])
       identity: 'marlin/scheme-package-hooks))

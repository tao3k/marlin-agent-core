;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .cc .o)
        (only-in :clan/poo/mop Type.)
        (only-in :clan/poo/object-contract ObjectContract.))

(export WorkingCopyScopeContract)

(def WorkingCopyScopeContract
  (.cc ObjectContract. responsibilities: (.o select-provider: (.cc Type. '.element? procedure?))))

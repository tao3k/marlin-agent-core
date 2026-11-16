;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; The Module identity is metadata, not a native ABI readiness receipt.
(import (only-in :clan/poo/object .cc)
        (only-in :clan/poo/type Exactly Record))
(export CAbiScopeContract)
(def CAbiScopeContract
  (.cc (Record identity: [(Exactly 'marlin/c-abi)])
       identity: 'marlin/c-abi))

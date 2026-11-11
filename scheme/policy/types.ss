;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Contract for the callable policy scope.

(import (only-in :clan/poo/object .cc .o)
        (only-in :clan/poo/mop Type.)
        (only-in :clan/poo/object-contract ObjectContract.))

(export PolicyScopeContract)

(def PolicyOperation (.cc Type. '.element? procedure?))

(def PolicyScopeContract
  (.cc ObjectContract.
       responsibilities: (.o project-routing: PolicyOperation
                             compile-loop-graph: PolicyOperation
                             next-continuation: PolicyOperation)))

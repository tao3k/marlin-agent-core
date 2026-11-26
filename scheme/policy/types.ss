;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Contract for the callable policy scope.

(import (only-in :clan/poo/object .cc .o object?)
        (only-in :clan/poo/type Type.)
        (only-in :clan/poo/types/object-contract ObjectContract.))

(export PolicyScopeContract)

(def PolicyOperation (.cc Type. '.element? procedure?))
(def PolicyOperationBundle (.cc Type. '.element? object?))

(def PolicyScopeContract
  (.cc ObjectContract.
       responsibilities: (.o project-routing: PolicyOperation
                             compile-loop-graph: PolicyOperation
                             next-continuation: PolicyOperation
                             security-governance: PolicyOperationBundle
                             higher-order-governance: PolicyOperationBundle)))

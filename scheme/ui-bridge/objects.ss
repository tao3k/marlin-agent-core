;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; POO prototypes for `scheme/ui-bridge` (strictly pure POO value declarations
;;; per Yuanxu module role contract — zero lambdas or functions in objects.ss).
(import (only-in :clan/poo/object .def))
(export SchemeBridgeModule.)

(.def SchemeBridgeModule.
  namespace: ""
  owner-id: "marlin.scheme"
  description: ""
  handle-query: #f)

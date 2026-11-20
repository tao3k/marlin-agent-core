;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; PTY control: use the original eager Yuanxu export lookup before completion.
(import (only-in :tao3k/yuanxu/prelude define-yuanxu-module-declaration yuanxu-module-export)
        (only-in "./catalog.ss" test-catalog))
(export eager-catalog)

(define-yuanxu-module-declaration eager-catalog
  (and (yuanxu-module-export ':marlin-tui-ui-test/fixture 'test-publication)
       test-catalog))

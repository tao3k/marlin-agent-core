;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; PTY control: demand the publication at completion time using the same loader.
(import (only-in :tao3k/yuanxu/prelude define-yuanxu-module-declaration yuanxu-module-declaration)
        (only-in :marlin-agent-core/ui/types UiScopeContract)
        (only-in "./catalog.ss" test-catalog))
(export eager-catalog)

(define-yuanxu-module-declaration eager-catalog
  (and (yuanxu-module-declaration ':marlin-tui-ui-test/fixture 'test-publication UiScopeContract)
       test-catalog))

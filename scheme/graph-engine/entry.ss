;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Link the GraphEngine Oracle + Fleet Controller C-ABI exports for Linked-AOT
;;; `marlin-scheme-host-aot` (zero dynamic `.o1` dlopen overhead at runtime).
(import (only-in :tao3k/yuanxu/module-system/source/autoload-declarations YuanxuLibraryModule)
        :marlin-agent-core/c-abi/graph-engine)
(export main)

(def (main . _)
  (void YuanxuLibraryModule))

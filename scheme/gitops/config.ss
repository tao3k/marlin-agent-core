;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .get .o)
        (only-in "./objects.ss" GitOpsHostOperations))
(export make-gitops-core)

(def (make-gitops-core lab-core)
  (.o (:: @ lab-core)
      exports: (.o (:: @ [GitOpsHostOperations (.get lab-core exports)]))))

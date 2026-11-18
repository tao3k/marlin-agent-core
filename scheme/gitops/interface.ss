;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;;
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

(import (only-in :clan/poo/object .def .get .o)
        (only-in :marlin-agent-core/interface LabGitOps)
        (only-in "./config.ss" GitOpsHostOperations))
(export GitOps)

(.def GitOps
  identity: 'gitops
  core: (.o (:: @ (.get LabGitOps core))
            exports: (.o (:: @ [GitOpsHostOperations (.get LabGitOps core exports)])))
  features: (.get LabGitOps features))

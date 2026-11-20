;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Import Lab's native declarations for use inside Marlin's selected modules.
(import (only-in :clan/poo/object .get)
        (only-in :tao3k/agentic-ai-lab/modules/ai-agentic-context/interface Context)
        (only-in :tao3k/agentic-ai-lab/modules/temporal-causality/interface TemporalCausality)
        (rename-in (only-in :tao3k/agentic-ai-lab/modules/gitops/interface GitOps)
                   (GitOps LabGitOps))
        (only-in :tao3k/agentic-ai-lab/modules/module-authoring/interface
                 yuanxu-module-interface agentic-ai-lab-modules
                 agentic-ai-lab-module-closure agentic-ai-lab-module-names
                 agentic-ai-lab-module-option-schemas
                 agentic-ai-lab-module-validation-receipts))
(export LabContext LabTemporal LabGitOps
        yuanxu-module-interface agentic-ai-lab-modules
        agentic-ai-lab-module-closure agentic-ai-lab-module-names
        agentic-ai-lab-module-option-schemas
        agentic-ai-lab-module-validation-receipts)

(def LabContext (.get Context core exports))
(def LabTemporal (.get TemporalCausality core exports))

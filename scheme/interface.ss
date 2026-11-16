;;; SPDX-FileCopyrightText: 2026 tao3k team and Contributors
;;; SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

;;; Keep the selected Module objects intact. Consumers use native POO slot
;;; access and dispatch rather than a parallel catalog of function aliases.
(import (only-in :clan/poo/object .get .o)
        (only-in :tao3k/agentic-ai-lab/init tao3k/agentic-ai-lab.init.exports)
        (only-in :tao3k/agentic-ai-lab/modules/module-authoring/interface
                 yuanxu-module-interface agentic-ai-lab-modules
                 agentic-ai-lab-module-closure agentic-ai-lab-module-names
                 agentic-ai-lab-module-option-schemas
                 agentic-ai-lab-module-validation-receipts))
(export make-lab-dependencies LabContext LabTemporal
        yuanxu-module-interface agentic-ai-lab-modules
        agentic-ai-lab-module-closure agentic-ai-lab-module-names
        agentic-ai-lab-module-option-schemas
        agentic-ai-lab-module-validation-receipts)

(def (make-lab-dependencies selection)
  (.o context: (.get selection modules ai-agentic-context)
      temporal: (.get selection modules temporal-causality)))

(def dependencies (make-lab-dependencies tao3k/agentic-ai-lab.init.exports))
(def LabContext (.get dependencies context))
(def LabTemporal (.get dependencies temporal))
